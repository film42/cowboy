use std::sync::Arc;

use axum::Json;
use axum::Router;
use axum::extract::ws::{Message, WebSocket};
use axum::extract::{Path, Query, State, WebSocketUpgrade};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;
use tower_http::services::{ServeDir, ServeFile};
use tracing::{info, warn};

use crate::game::Phase;
use crate::lobby::{ClientMessage, InviteCode, LobbyStore, ServerMessage};
use crate::player::PlayerId;

#[derive(Clone)]
pub struct AppState {
    pub lobbies: LobbyStore,
    pub livekit_api_key: String,
    pub livekit_api_secret: String,
    pub livekit_url: String,
}

pub fn create_router(state: AppState) -> Router {
    let spa_fallback =
        ServeDir::new("./client/dist").fallback(ServeFile::new("./client/dist/index.html"));

    Router::new()
        .route("/api/lobby", post(create_lobby))
        .route("/api/lobby/{code}", get(get_lobby))
        .route("/api/lobby/{code}/join", post(join_lobby))
        .route("/api/lobby/{code}/leave", post(leave_lobby))
        .route("/api/livekit/token", post(get_livekit_token))
        .route("/ws/{code}", get(ws_upgrade))
        .fallback_service(spa_fallback)
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

async fn get_lobby(State(state): State<AppState>, Path(code): Path<String>) -> impl IntoResponse {
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

#[derive(Deserialize)]
pub struct LeaveLobbyRequest {
    pub player_id: PlayerId,
}

async fn leave_lobby(
    State(state): State<AppState>,
    Path(code): Path<String>,
    Json(req): Json<LeaveLobbyRequest>,
) -> impl IntoResponse {
    let lobbies = state.lobbies.read().await;
    match lobbies.get(&code) {
        Some(lobby) => {
            let mut lobby = lobby.write().await;
            match lobby.remove_player(req.player_id) {
                Ok(()) => {
                    let state = lobby.lobby_state();
                    let _ = lobby.tx.send(ServerMessage::LobbyUpdate { state });
                    Ok(StatusCode::OK)
                }
                Err(e) => Err((StatusCode::BAD_REQUEST, e)),
            }
        }
        None => Err((StatusCode::NOT_FOUND, "Lobby not found".to_string())),
    }
}

// --- LiveKit Token ---

#[derive(Deserialize)]
pub struct LivekitTokenRequest {
    pub lobby_code: String,
    pub player_name: String,
    pub player_id: PlayerId,
}

#[derive(Serialize)]
pub struct LivekitTokenResponse {
    pub token: String,
    pub url: String,
}

async fn get_livekit_token(
    State(state): State<AppState>,
    Json(req): Json<LivekitTokenRequest>,
) -> impl IntoResponse {
    // Verify the lobby exists and player is in it
    let lobbies = state.lobbies.read().await;
    match lobbies.get(&req.lobby_code) {
        Some(lobby) => {
            let lobby = lobby.read().await;
            let in_lobby = lobby.players.iter().any(|p| p.id == req.player_id);
            if !in_lobby {
                return Err((StatusCode::FORBIDDEN, "Not in this lobby".to_string()));
            }
        }
        None => return Err((StatusCode::NOT_FOUND, "Lobby not found".to_string())),
    }

    // Room name = lobby code
    let room_name = format!("cowboy-{}", req.lobby_code);
    let identity = format!("player-{}", req.player_id);

    let grants = livekit_api::access_token::VideoGrants {
        room_join: true,
        room: room_name,
        can_publish: true,
        can_subscribe: true,
        ..Default::default()
    };

    let token = livekit_api::access_token::AccessToken::with_api_key(
        &state.livekit_api_key,
        &state.livekit_api_secret,
    )
    .with_identity(&identity)
    .with_name(&req.player_name)
    .with_grants(grants)
    .to_jwt();

    match token {
        Ok(jwt) => Ok(Json(LivekitTokenResponse {
            token: jwt,
            url: state.livekit_url.clone(),
        })),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to generate token: {e}"),
        )),
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
            if let Ok(msg) = serde_json::to_string(&ServerMessage::GameState { state: game_state })
            {
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
                    if let Ok(text) =
                        serde_json::to_string(&ServerMessage::GameState { state: game_state })
                    {
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
                            handle_client_message(&recv_lobby, player_id, client_msg).await;
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

/// After processing a game action, check if we entered cowboy vote phase
/// and spawn a timer to auto-resolve after 5 seconds.
fn maybe_spawn_cowboy_timer(lobby_arc: &Arc<RwLock<crate::lobby::Lobby>>) {
    let lobby_clone = lobby_arc.clone();
    // We check the phase inside the spawned task after acquiring the lock
    tokio::spawn(async move {
        // Small delay to let the current lock release
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;

        // Check if we're actually in cowboy vote phase
        {
            let lobby = lobby_clone.read().await;
            let in_cowboy_vote = lobby
                .game
                .as_ref()
                .is_some_and(|g| matches!(g.phase, Phase::CowboyVote { .. }));
            if !in_cowboy_vote {
                return;
            }
        }

        info!("Cowboy vote timer started (30 seconds)");
        tokio::time::sleep(std::time::Duration::from_secs(30)).await;

        // Resolve the vote
        let mut lobby = lobby_clone.write().await;

        // Check phase and resolve without holding a sub-borrow across broadcast
        let should_resolve = lobby
            .game
            .as_ref()
            .is_some_and(|g| matches!(g.phase, Phase::CowboyVote { .. }));

        if !should_resolve {
            return;
        }

        info!("Cowboy vote timer expired, auto-resolving");
        let game = lobby.game.as_mut().unwrap();
        if game.act(0, crate::game::Action::ResolveCowboyVote).is_ok() {
            let events = game.take_events();
            let game_over_winner = match &game.phase {
                Phase::GameOver { winner } => *winner,
                _ => None,
            };

            // Drop the game borrow before broadcasting
            let _ = lobby.tx.send(ServerMessage::GameEvents { events });

            if let Some(wid) = game_over_winner {
                lobby.record_game_result(wid);
                let state = lobby.lobby_state();
                let _ = lobby.tx.send(ServerMessage::LobbyUpdate { state });
            }
        }
    });
}

/// Spawn a 30-second timer for the current player's turn.
/// Auto-defaults: Pass for normal turn, AcceptTrade for block, DealerPass for dealer.
fn maybe_spawn_turn_timer(lobby_arc: &Arc<RwLock<crate::lobby::Lobby>>) {
    let lobby_clone = lobby_arc.clone();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;

        // Snapshot who's acting and what phase
        let (actor_id, phase_tag) = {
            let lobby = lobby_clone.read().await;
            let game = match lobby.game.as_ref() {
                Some(g) => g,
                None => return,
            };
            let actor = match game.current_actor() {
                Some(a) => a,
                None => return,
            };
            let tag = match &game.phase {
                Phase::NormalTurn { .. } => "normal",
                Phase::WaitingForBlock { .. } => "block",
                Phase::DealerTurn => "dealer",
                _ => return,
            };
            (actor, tag)
        };

        tokio::time::sleep(std::time::Duration::from_secs(30)).await;

        let mut lobby = lobby_clone.write().await;
        let game = match lobby.game.as_mut() {
            Some(g) => g,
            None => return,
        };

        // Only act if the same player is still the actor in the same type of phase
        let still_waiting = game.current_actor() == Some(actor_id)
            && match (&game.phase, phase_tag) {
                (Phase::NormalTurn { .. }, "normal") => true,
                (Phase::WaitingForBlock { .. }, "block") => true,
                (Phase::DealerTurn, "dealer") => true,
                _ => false,
            };

        if !still_waiting {
            return;
        }

        let default_action = match phase_tag {
            "normal" => crate::game::Action::Pass,
            "block" => crate::game::Action::AcceptTrade,
            "dealer" => crate::game::Action::DealerPass,
            _ => return,
        };

        info!("Turn timer expired for player {actor_id}, auto-defaulting");
        if game.act(actor_id, default_action).is_ok() {
            let mut events = game.take_events();

            // If all cowboy votes are in after this action, resolve
            if game.all_cowboy_votes_in() {
                if game.act(0, crate::game::Action::ResolveCowboyVote).is_ok() {
                    events.extend(game.take_events());
                }
            }

            let game_over_winner = match &game.phase {
                Phase::GameOver { winner } => *winner,
                _ => None,
            };

            let _ = lobby.tx.send(ServerMessage::GameEvents { events });

            if let Some(wid) = game_over_winner {
                lobby.record_game_result(wid);
                let state = lobby.lobby_state();
                let _ = lobby.tx.send(ServerMessage::LobbyUpdate { state });
            }
        }
    });
}

async fn handle_client_message(
    lobby: &Arc<RwLock<crate::lobby::Lobby>>,
    player_id: PlayerId,
    msg: ClientMessage,
) {
    let mut lobby_guard = lobby.write().await;

    match msg {
        ClientMessage::StartGame {
            lives,
            blocker_tokens,
            exemption_tokens,
        } => {
            match lobby_guard.start_game(player_id, lives, blocker_tokens, exemption_tokens) {
                Ok(()) => {
                    info!("Game started in lobby {}", lobby_guard.code);
                    let game = lobby_guard.game.as_mut().unwrap();
                    let events = game.take_events();

                    let in_cowboy = matches!(game.phase, Phase::CowboyVote { .. });

                    // Broadcast events
                    let _ = lobby_guard.tx.send(ServerMessage::GameEvents { events });

                    // Lobby state update (game_active = true)
                    let state = lobby_guard.lobby_state();
                    let _ = lobby_guard.tx.send(ServerMessage::LobbyUpdate { state });

                    if in_cowboy {
                        drop(lobby_guard);
                        maybe_spawn_cowboy_timer(lobby);
                        return;
                    }

                    // Start turn timer for the first player
                    drop(lobby_guard);
                    maybe_spawn_turn_timer(lobby);
                    return;
                }
                Err(e) => {
                    let _ = lobby_guard.tx.send(ServerMessage::Error { message: e });
                }
            }
        }

        ClientMessage::GameAction { action } => {
            let game = match lobby_guard.game.as_mut() {
                Some(g) => g,
                None => {
                    let _ = lobby_guard.tx.send(ServerMessage::Error {
                        message: "No active game".to_string(),
                    });
                    return;
                }
            };

            match game.act(player_id, action) {
                Ok(()) => {
                    let mut events = game.take_events();

                    // If all cowboy votes are in, resolve immediately
                    if game.all_cowboy_votes_in() {
                        info!("All cowboy votes received, resolving immediately");
                        if game.act(0, crate::game::Action::ResolveCowboyVote).is_ok() {
                            events.extend(game.take_events());
                        }
                    }

                    let game_over = matches!(game.phase, Phase::GameOver { .. });
                    let winner_id = match &game.phase {
                        Phase::GameOver { winner } => *winner,
                        _ => None,
                    };
                    let in_cowboy = matches!(game.phase, Phase::CowboyVote { .. });
                    let has_actor = game.current_actor().is_some();

                    let _ = lobby_guard.tx.send(ServerMessage::GameEvents { events });

                    if game_over {
                        if let Some(wid) = winner_id {
                            lobby_guard.record_game_result(wid);
                            let state = lobby_guard.lobby_state();
                            let _ = lobby_guard.tx.send(ServerMessage::LobbyUpdate { state });
                        }
                    }

                    if in_cowboy {
                        drop(lobby_guard);
                        maybe_spawn_cowboy_timer(lobby);
                        return;
                    }

                    if has_actor && !game_over {
                        drop(lobby_guard);
                        maybe_spawn_turn_timer(lobby);
                        return;
                    }
                }
                Err(e) => {
                    let _ = lobby_guard.tx.send(ServerMessage::Error {
                        message: e.to_string(),
                    });
                }
            }
        }

        ClientMessage::TransferHost { to_player_id } => {
            match lobby_guard.transfer_host(player_id, to_player_id) {
                Ok(()) => {
                    let state = lobby_guard.lobby_state();
                    let _ = lobby_guard.tx.send(ServerMessage::LobbyUpdate { state });
                }
                Err(e) => {
                    let _ = lobby_guard.tx.send(ServerMessage::Error { message: e });
                }
            }
        }

        ClientMessage::EndGame => match lobby_guard.end_game(player_id) {
            Ok(()) => {
                info!("Game ended early by host in lobby {}", lobby_guard.code);
                let state = lobby_guard.lobby_state();
                let _ = lobby_guard.tx.send(ServerMessage::LobbyUpdate { state });
            }
            Err(e) => {
                let _ = lobby_guard.tx.send(ServerMessage::Error { message: e });
            }
        },

        ClientMessage::EndLobby => {
            if lobby_guard.host_id != player_id {
                let _ = lobby_guard.tx.send(ServerMessage::Error {
                    message: "Only the host can end the lobby".to_string(),
                });
                return;
            }
            info!("Lobby {} closed by host", lobby_guard.code);
            let _ = lobby_guard.tx.send(ServerMessage::LobbyClosed);
        }

        ClientMessage::KickPlayer { player_id: kick_id } => {
            if lobby_guard.host_id != player_id {
                let _ = lobby_guard.tx.send(ServerMessage::Error {
                    message: "Only the host can kick players".to_string(),
                });
                return;
            }
            match lobby_guard.remove_player(kick_id) {
                Ok(()) => {
                    let state = lobby_guard.lobby_state();
                    let _ = lobby_guard.tx.send(ServerMessage::LobbyUpdate { state });
                }
                Err(e) => {
                    let _ = lobby_guard.tx.send(ServerMessage::Error { message: e });
                }
            }
        }
    }
}
