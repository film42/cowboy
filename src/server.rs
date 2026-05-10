use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket};
use axum::extract::{Path, Query, State, WebSocketUpgrade};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::Json;
use axum::Router;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;
use tracing::{info, warn};

use crate::game::Phase;
use crate::lobby::{ClientMessage, InviteCode, LobbyStore, ServerMessage};
use crate::player::PlayerId;

#[derive(Clone)]
pub struct AppState {
    pub lobbies: LobbyStore,
}

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/api/lobby", post(create_lobby))
        .route("/api/lobby/{code}", get(get_lobby))
        .route("/api/lobby/{code}/join", post(join_lobby))
        .route("/ws/{code}", get(ws_upgrade))
        .layer(CorsLayer::permissive())
        .with_state(state)
}

// --- HTTP Handlers ---

#[derive(Deserialize)]
pub struct CreateLobbyRequest {
    pub host_name: String,
}

#[derive(Serialize)]
pub struct CreateLobbyResponse {
    pub code: InviteCode,
    pub player_id: PlayerId,
    pub session_token: String,
}

async fn create_lobby(
    State(state): State<AppState>,
    Json(req): Json<CreateLobbyRequest>,
) -> impl IntoResponse {
    let mut lobby = crate::lobby::Lobby::new();
    let (player_id, session_token) = lobby.add_player(req.host_name).unwrap();
    let code = lobby.code.clone();

    let lobby = Arc::new(RwLock::new(lobby));
    state.lobbies.write().await.insert(code.clone(), lobby);

    info!("Lobby {code} created");

    Json(CreateLobbyResponse {
        code,
        player_id,
        session_token,
    })
}

async fn get_lobby(
    State(state): State<AppState>,
    Path(code): Path<String>,
) -> impl IntoResponse {
    let lobbies = state.lobbies.read().await;
    match lobbies.get(&code) {
        Some(lobby) => {
            let lobby = lobby.read().await;
            Ok(Json(lobby.lobby_state()))
        }
        None => Err(StatusCode::NOT_FOUND),
    }
}

#[derive(Deserialize)]
pub struct JoinLobbyRequest {
    pub name: String,
}

#[derive(Serialize)]
pub struct JoinLobbyResponse {
    pub player_id: PlayerId,
    pub session_token: String,
}

async fn join_lobby(
    State(state): State<AppState>,
    Path(code): Path<String>,
    Json(req): Json<JoinLobbyRequest>,
) -> impl IntoResponse {
    let lobbies = state.lobbies.read().await;
    match lobbies.get(&code) {
        Some(lobby) => {
            let mut lobby = lobby.write().await;
            match lobby.add_player(req.name) {
                Ok((player_id, session_token)) => {
                    // Broadcast lobby update
                    let state = lobby.lobby_state();
                    let _ = lobby.tx.send(ServerMessage::LobbyUpdate { state });
                    Ok(Json(JoinLobbyResponse {
                        player_id,
                        session_token,
                    }))
                }
                Err(e) => Err((StatusCode::BAD_REQUEST, e)),
            }
        }
        None => Err((StatusCode::NOT_FOUND, "Lobby not found".to_string())),
    }
}

// --- WebSocket ---

#[derive(Deserialize)]
pub struct WsQuery {
    pub session_token: String,
}

async fn ws_upgrade(
    State(state): State<AppState>,
    Path(code): Path<String>,
    Query(query): Query<WsQuery>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    let lobbies = state.lobbies.read().await;
    let lobby_arc = match lobbies.get(&code) {
        Some(l) => l.clone(),
        None => return StatusCode::NOT_FOUND.into_response(),
    };

    // Validate session token and get player_id
    let player_id = {
        let mut lobby = lobby_arc.write().await;
        match lobby.reconnect(&query.session_token) {
            Ok(id) => id,
            Err(_) => return StatusCode::UNAUTHORIZED.into_response(),
        }
    };

    let session_token = query.session_token.clone();

    ws.on_upgrade(move |socket| handle_ws(socket, lobby_arc, player_id, session_token))
}

async fn handle_ws(
    socket: WebSocket,
    lobby: Arc<RwLock<crate::lobby::Lobby>>,
    player_id: PlayerId,
    session_token: String,
) {
    let (mut ws_tx, mut ws_rx) = socket.split();
    use futures_util::{SinkExt, StreamExt};

    // Subscribe to broadcast channel
    let mut broadcast_rx = {
        let lobby = lobby.read().await;
        lobby.tx.subscribe()
    };

    info!("Player {player_id} connected via WebSocket");

    // Send welcome message
    let welcome = ServerMessage::Welcome {
        player_id,
        session_token: session_token.clone(),
    };
    if let Ok(msg) = serde_json::to_string(&welcome) {
        let _ = ws_tx.send(Message::Text(msg.into())).await;
    }

    // Send current lobby state
    {
        let lobby = lobby.read().await;
        let state = lobby.lobby_state();
        if let Ok(msg) = serde_json::to_string(&ServerMessage::LobbyUpdate { state }) {
            let _ = ws_tx.send(Message::Text(msg.into())).await;
        }

        // If game is active, send current game state
        if let Some(game_state) = lobby.player_game_state(player_id) {
            if let Ok(msg) = serde_json::to_string(&ServerMessage::GameState {
                state: game_state,
            }) {
                let _ = ws_tx.send(Message::Text(msg.into())).await;
            }
        }
    }

    // Broadcast that this player connected
    {
        let lobby = lobby.read().await;
        let state = lobby.lobby_state();
        let _ = lobby.tx.send(ServerMessage::LobbyUpdate { state });
    }

    // Spawn task to forward broadcast messages to this client's WebSocket
    let forward_lobby = lobby.clone();
    let forward_player_id = player_id;
    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = broadcast_rx.recv().await {
            // For GameState messages, only send if it's for this player
            // For other messages, send to everyone
            let should_send = match &msg {
                ServerMessage::GameState { .. } => false, // handled separately
                _ => true,
            };

            if should_send {
                if let Ok(text) = serde_json::to_string(&msg) {
                    if ws_tx.send(Message::Text(text.into())).await.is_err() {
                        break;
                    }
                }
            }

            // After game events, send this player's personal game state
            if matches!(msg, ServerMessage::GameEvents { .. }) {
                let lobby = forward_lobby.read().await;
                if let Some(game_state) = lobby.player_game_state(forward_player_id) {
                    if let Ok(text) = serde_json::to_string(&ServerMessage::GameState {
                        state: game_state,
                    }) {
                        if ws_tx.send(Message::Text(text.into())).await.is_err() {
                            break;
                        }
                    }
                }
            }
        }
    });

    // Handle incoming messages from this client
    let recv_lobby = lobby.clone();
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = ws_rx.next().await {
            match msg {
                Message::Text(text) => {
                    let text_str: &str = &text;
                    match serde_json::from_str::<ClientMessage>(text_str) {
                        Ok(client_msg) => {
                            handle_client_message(
                                &recv_lobby,
                                player_id,
                                client_msg,
                            )
                            .await;
                        }
                        Err(e) => {
                            warn!("Invalid message from player {player_id}: {e}");
                            let lobby = recv_lobby.read().await;
                            let _ = lobby.tx.send(ServerMessage::Error {
                                message: format!("Invalid message: {e}"),
                            });
                        }
                    }
                }
                Message::Close(_) => break,
                _ => {}
            }
        }
    });

    // Wait for either task to finish (connection closed or error)
    tokio::select! {
        _ = &mut send_task => {
            recv_task.abort();
        }
        _ = &mut recv_task => {
            send_task.abort();
        }
    }

    // Mark player as disconnected
    {
        let mut lobby = lobby.write().await;
        lobby.disconnect(player_id);
        let state = lobby.lobby_state();
        let _ = lobby.tx.send(ServerMessage::LobbyUpdate { state });
    }

    info!("Player {player_id} disconnected");
}

async fn handle_client_message(
    lobby: &Arc<RwLock<crate::lobby::Lobby>>,
    player_id: PlayerId,
    msg: ClientMessage,
) {
    let mut lobby = lobby.write().await;

    match msg {
        ClientMessage::StartGame {
            lives,
            blocker_tokens,
            exemption_tokens,
        } => {
            match lobby.start_game(player_id, lives, blocker_tokens, exemption_tokens) {
                Ok(()) => {
                    info!("Game started in lobby {}", lobby.code);
                    let game = lobby.game.as_mut().unwrap();
                    let events = game.take_events();

                    // Broadcast events
                    let _ = lobby
                        .tx
                        .send(ServerMessage::GameEvents { events });

                    // Lobby state update (game_active = true)
                    let state = lobby.lobby_state();
                    let _ = lobby.tx.send(ServerMessage::LobbyUpdate { state });
                }
                Err(e) => {
                    let _ = lobby.tx.send(ServerMessage::Error { message: e });
                }
            }
        }

        ClientMessage::GameAction { action } => {
            let game = match lobby.game.as_mut() {
                Some(g) => g,
                None => {
                    let _ = lobby.tx.send(ServerMessage::Error {
                        message: "No active game".to_string(),
                    });
                    return;
                }
            };

            match game.act(player_id, action) {
                Ok(()) => {
                    let events = game.take_events();

                    // Check if game ended
                    let game_over = matches!(game.phase, Phase::GameOver { .. });
                    let winner_id = if let Phase::GameOver { winner } = &game.phase {
                        *winner
                    } else {
                        None
                    };

                    // Broadcast events
                    let _ = lobby
                        .tx
                        .send(ServerMessage::GameEvents { events });

                    if game_over {
                        if let Some(wid) = winner_id {
                            lobby.record_game_result(wid);
                            let state = lobby.lobby_state();
                            let _ = lobby.tx.send(ServerMessage::LobbyUpdate { state });
                        }
                    }
                }
                Err(e) => {
                    let _ = lobby.tx.send(ServerMessage::Error {
                        message: e.to_string(),
                    });
                }
            }
        }

        ClientMessage::TransferHost { to_player_id } => {
            match lobby.transfer_host(player_id, to_player_id) {
                Ok(()) => {
                    let state = lobby.lobby_state();
                    let _ = lobby.tx.send(ServerMessage::LobbyUpdate { state });
                }
                Err(e) => {
                    let _ = lobby.tx.send(ServerMessage::Error { message: e });
                }
            }
        }

        ClientMessage::KickPlayer {
            player_id: kick_id,
        } => {
            if lobby.host_id != player_id {
                let _ = lobby.tx.send(ServerMessage::Error {
                    message: "Only the host can kick players".to_string(),
                });
                return;
            }
            match lobby.remove_player(kick_id) {
                Ok(()) => {
                    let state = lobby.lobby_state();
                    let _ = lobby.tx.send(ServerMessage::LobbyUpdate { state });
                }
                Err(e) => {
                    let _ = lobby.tx.send(ServerMessage::Error { message: e });
                }
            }
        }
    }
}
