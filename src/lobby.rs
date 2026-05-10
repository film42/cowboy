use std::collections::HashMap;
use std::sync::Arc;

use rand::Rng;
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, RwLock};

use crate::game::Game;
use crate::player::PlayerId;

/// A session token that allows reconnecting to a lobby.
pub type SessionToken = String;

/// A short invite code like "ABCD" for joining lobbies.
pub type InviteCode = String;

/// Generates a random 4-character uppercase invite code.
fn generate_invite_code() -> InviteCode {
    let mut rng = rand::rng();
    (0..4)
        .map(|_| (rng.random_range(b'A'..=b'Z')) as char)
        .collect()
}

/// Generates a UUID session token.
fn generate_session_token() -> SessionToken {
    uuid::Uuid::new_v4().to_string()
}

/// Info about a connected (or disconnected) player in a lobby.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LobbyPlayer {
    pub id: PlayerId,
    pub name: String,
    pub session_token: SessionToken,
    pub is_connected: bool,
    pub is_host: bool,
}

/// Snapshot of lobby state sent to clients.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LobbyState {
    pub code: InviteCode,
    pub players: Vec<LobbyPlayerView>,
    pub host_id: PlayerId,
    pub game_active: bool,
    pub game_history: Vec<GameResult>,
}

/// What clients see about other players (no session tokens).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LobbyPlayerView {
    pub id: PlayerId,
    pub name: String,
    pub is_connected: bool,
    pub is_host: bool,
}

/// Result of a completed game.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameResult {
    pub game_number: u32,
    pub player_count: usize,
    pub round_count: u32,
    pub winner_name: String,
}

/// Messages broadcast from the lobby to all connected clients.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerMessage {
    /// Lobby state update (player joined, left, settings changed).
    LobbyUpdate { state: LobbyState },
    /// Game state update for a specific player (their hand, etc.).
    GameState { state: PlayerGameState },
    /// Game events for animation (trade proposed, cowboy triggered, etc.).
    GameEvents {
        events: Vec<crate::game::GameEvent>,
    },
    /// Error message.
    Error { message: String },
    /// Welcome message with session info on connect/reconnect.
    Welcome {
        player_id: PlayerId,
        session_token: SessionToken,
    },
    /// A player's action prompt (it's their turn).
    ActionRequired {
        player_id: PlayerId,
        valid_actions: Vec<String>,
        timeout_secs: Option<u8>,
    },
}

/// What each player sees about the game state (filtered view).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerGameState {
    pub round_number: u32,
    pub your_card: Option<crate::card::Card>,
    pub your_lives: u8,
    pub your_exemption_tokens: u8,
    pub your_blocker_tokens: u8,
    pub players: Vec<PlayerPublicState>,
    pub phase: String,
    pub is_cowboy_round: bool,
    pub dealer_id: PlayerId,
    pub current_actor: Option<PlayerId>,
}

/// What everyone can see about a player.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerPublicState {
    pub id: PlayerId,
    pub name: String,
    pub lives: u8,
    pub has_blocker: bool,
    pub has_exemption: bool,
    pub is_eliminated: bool,
    pub revealed_card: Option<crate::card::Card>,
}

/// Messages clients send to the server.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    /// Start the game (host only).
    StartGame {
        lives: u8,
        blocker_tokens: u8,
        exemption_tokens: u8,
    },
    /// Game action.
    GameAction { action: crate::game::Action },
    /// Transfer host to another player.
    TransferHost { to_player_id: PlayerId },
    /// Kick a player (host only).
    KickPlayer { player_id: PlayerId },
}

pub struct Lobby {
    pub code: InviteCode,
    pub players: Vec<LobbyPlayer>,
    pub host_id: PlayerId,
    pub game: Option<Game>,
    pub game_history: Vec<GameResult>,
    pub game_count: u32,
    next_player_id: PlayerId,
    /// Broadcast channel for sending messages to all connected clients.
    pub tx: broadcast::Sender<ServerMessage>,
}

impl Lobby {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(256);
        Self {
            code: generate_invite_code(),
            players: Vec::new(),
            host_id: 0,
            game: None,
            game_history: Vec::new(),
            game_count: 0,
            next_player_id: 1,
            tx,
        }
    }

    /// Add a player to the lobby. Returns (player_id, session_token).
    pub fn add_player(&mut self, name: String) -> Result<(PlayerId, SessionToken), String> {
        if self.game.is_some() {
            return Err("Game is already in progress".to_string());
        }

        let id = self.next_player_id;
        self.next_player_id += 1;
        let token = generate_session_token();

        let is_host = self.players.is_empty();
        if is_host {
            self.host_id = id;
        }

        self.players.push(LobbyPlayer {
            id,
            name,
            session_token: token.clone(),
            is_connected: true,
            is_host,
        });

        Ok((id, token))
    }

    /// Reconnect a player using their session token.
    pub fn reconnect(&mut self, session_token: &str) -> Result<PlayerId, String> {
        let player = self
            .players
            .iter_mut()
            .find(|p| p.session_token == session_token)
            .ok_or_else(|| "Invalid session token".to_string())?;

        player.is_connected = true;
        Ok(player.id)
    }

    /// Mark a player as disconnected.
    pub fn disconnect(&mut self, player_id: PlayerId) {
        if let Some(player) = self.players.iter_mut().find(|p| p.id == player_id) {
            player.is_connected = false;
        }
    }

    /// Remove a player entirely (kick or leave while in lobby).
    pub fn remove_player(&mut self, player_id: PlayerId) -> Result<(), String> {
        if self.game.is_some() {
            return Err("Cannot remove player during active game".to_string());
        }

        self.players.retain(|p| p.id != player_id);

        // If host left, transfer to next connected player
        if self.host_id == player_id {
            if let Some(new_host) = self.players.iter_mut().find(|p| p.is_connected) {
                new_host.is_host = true;
                self.host_id = new_host.id;
            }
        }

        Ok(())
    }

    /// Transfer host to another player.
    pub fn transfer_host(
        &mut self,
        from_id: PlayerId,
        to_id: PlayerId,
    ) -> Result<(), String> {
        if self.host_id != from_id {
            return Err("Only the host can transfer host".to_string());
        }

        let to_exists = self.players.iter().any(|p| p.id == to_id);
        if !to_exists {
            return Err("Target player not found".to_string());
        }

        for p in &mut self.players {
            p.is_host = p.id == to_id;
        }
        self.host_id = to_id;
        Ok(())
    }

    /// Start a new game.
    pub fn start_game(
        &mut self,
        host_id: PlayerId,
        lives: u8,
        blocker_tokens: u8,
        exemption_tokens: u8,
    ) -> Result<(), String> {
        if self.host_id != host_id {
            return Err("Only the host can start the game".to_string());
        }

        let connected: Vec<&LobbyPlayer> =
            self.players.iter().filter(|p| p.is_connected).collect();
        if connected.len() < 2 {
            return Err("Need at least 2 connected players".to_string());
        }

        let mut game = Game::new();
        for player in &connected {
            game.add_player(player.id, player.name.clone())
                .map_err(|e| e.to_string())?;
        }

        game.act(
            0,
            crate::game::Action::StartGame {
                lives,
                blocker_tokens,
                exemption_tokens,
            },
        )
        .map_err(|e| e.to_string())?;

        self.game_count += 1;
        self.game = Some(game);
        Ok(())
    }

    /// Build the lobby state view (no session tokens).
    pub fn lobby_state(&self) -> LobbyState {
        LobbyState {
            code: self.code.clone(),
            players: self
                .players
                .iter()
                .map(|p| LobbyPlayerView {
                    id: p.id,
                    name: p.name.clone(),
                    is_connected: p.is_connected,
                    is_host: p.is_host,
                })
                .collect(),
            host_id: self.host_id,
            game_active: self.game.is_some(),
            game_history: self.game_history.clone(),
        }
    }

    /// Build the game state view for a specific player.
    pub fn player_game_state(&self, player_id: PlayerId) -> Option<PlayerGameState> {
        let game = self.game.as_ref()?;

        let player = game.players.iter().find(|p| p.id == player_id)?;

        let phase_str = match &game.phase {
            crate::game::Phase::Lobby => "lobby",
            crate::game::Phase::KingCheck => "king_check",
            crate::game::Phase::NormalTurn { .. } => "normal_turn",
            crate::game::Phase::WaitingForBlock { .. } => "waiting_for_block",
            crate::game::Phase::DealerTurn => "dealer_turn",
            crate::game::Phase::CowboyVote { .. } => "cowboy_vote",
            crate::game::Phase::Showdown => "showdown",
            crate::game::Phase::RoundEnd => "round_end",
            crate::game::Phase::GameOver { .. } => "game_over",
        };

        let players_public: Vec<PlayerPublicState> = game
            .players
            .iter()
            .map(|p| {
                // Only show revealed cards during showdown/round_end for active (non-exempted) players
                let revealed = match &game.phase {
                    crate::game::Phase::RoundEnd | crate::game::Phase::GameOver { .. } => {
                        let p_idx = game
                            .players
                            .iter()
                            .position(|pp| pp.id == p.id)
                            .unwrap();
                        if !game.exempted.contains(&p_idx) {
                            p.hand
                        } else {
                            None
                        }
                    }
                    _ => None,
                };

                PlayerPublicState {
                    id: p.id,
                    name: p.name.clone(),
                    lives: p.lives,
                    has_blocker: p.blocker_tokens > 0,
                    has_exemption: p.exemption_tokens > 0,
                    is_eliminated: p.is_eliminated,
                    revealed_card: revealed,
                }
            })
            .collect();

        Some(PlayerGameState {
            round_number: game.round_number,
            your_card: player.hand,
            your_lives: player.lives,
            your_exemption_tokens: player.exemption_tokens,
            your_blocker_tokens: player.blocker_tokens,
            players: players_public,
            phase: phase_str.to_string(),
            is_cowboy_round: game.is_cowboy_round,
            dealer_id: game.players[game.dealer_idx].id,
            current_actor: game.current_actor(),
        })
    }

    /// Record a game result when the game ends.
    pub fn record_game_result(&mut self, winner_id: PlayerId) {
        let winner_name = self
            .players
            .iter()
            .find(|p| p.id == winner_id)
            .map(|p| p.name.clone())
            .unwrap_or_else(|| "Unknown".to_string());

        let round_count = self
            .game
            .as_ref()
            .map(|g| g.round_number)
            .unwrap_or(0);

        self.game_history.push(GameResult {
            game_number: self.game_count,
            player_count: self
                .game
                .as_ref()
                .map(|g| g.players.len())
                .unwrap_or(0),
            round_count,
            winner_name,
        });

        self.game = None;
    }
}

/// Shared state: all active lobbies.
pub type LobbyStore = Arc<RwLock<HashMap<InviteCode, Arc<RwLock<Lobby>>>>>;

pub fn new_lobby_store() -> LobbyStore {
    Arc::new(RwLock::new(HashMap::new()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_lobby_and_add_players() {
        let mut lobby = Lobby::new();
        assert_eq!(lobby.code.len(), 4);

        let (id1, token1) = lobby.add_player("Alice".to_string()).unwrap();
        assert_eq!(id1, 1);
        assert!(lobby.players[0].is_host);
        assert_eq!(lobby.host_id, 1);

        let (id2, _token2) = lobby.add_player("Bob".to_string()).unwrap();
        assert_eq!(id2, 2);
        assert!(!lobby.players[1].is_host);

        assert!(!token1.is_empty());
    }

    #[test]
    fn first_player_is_host() {
        let mut lobby = Lobby::new();
        lobby.add_player("Alice".to_string()).unwrap();
        lobby.add_player("Bob".to_string()).unwrap();
        assert_eq!(lobby.host_id, 1);
    }

    #[test]
    fn cannot_join_during_game() {
        let mut lobby = Lobby::new();
        lobby.add_player("Alice".to_string()).unwrap();
        lobby.add_player("Bob".to_string()).unwrap();
        lobby.start_game(1, 3, 1, 1).unwrap();

        let result = lobby.add_player("Charlie".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn reconnect_with_session_token() {
        let mut lobby = Lobby::new();
        let (id1, token1) = lobby.add_player("Alice".to_string()).unwrap();

        lobby.disconnect(id1);
        assert!(!lobby.players[0].is_connected);

        let reconnected_id = lobby.reconnect(&token1).unwrap();
        assert_eq!(reconnected_id, id1);
        assert!(lobby.players[0].is_connected);
    }

    #[test]
    fn invalid_session_token_rejected() {
        let mut lobby = Lobby::new();
        lobby.add_player("Alice".to_string()).unwrap();

        let result = lobby.reconnect("bogus-token");
        assert!(result.is_err());
    }

    #[test]
    fn host_transfers_on_leave() {
        let mut lobby = Lobby::new();
        lobby.add_player("Alice".to_string()).unwrap();
        lobby.add_player("Bob".to_string()).unwrap();

        lobby.remove_player(1).unwrap();
        assert_eq!(lobby.host_id, 2);
        assert!(lobby.players[0].is_host);
    }

    #[test]
    fn transfer_host_explicitly() {
        let mut lobby = Lobby::new();
        lobby.add_player("Alice".to_string()).unwrap();
        lobby.add_player("Bob".to_string()).unwrap();

        lobby.transfer_host(1, 2).unwrap();
        assert_eq!(lobby.host_id, 2);
    }

    #[test]
    fn non_host_cannot_transfer() {
        let mut lobby = Lobby::new();
        lobby.add_player("Alice".to_string()).unwrap();
        lobby.add_player("Bob".to_string()).unwrap();

        let result = lobby.transfer_host(2, 1);
        assert!(result.is_err());
    }

    #[test]
    fn start_game_requires_host() {
        let mut lobby = Lobby::new();
        lobby.add_player("Alice".to_string()).unwrap();
        lobby.add_player("Bob".to_string()).unwrap();

        let result = lobby.start_game(2, 3, 1, 1);
        assert!(result.is_err());
    }

    #[test]
    fn start_game_needs_two_players() {
        let mut lobby = Lobby::new();
        lobby.add_player("Alice".to_string()).unwrap();

        let result = lobby.start_game(1, 3, 1, 1);
        assert!(result.is_err());
    }

    #[test]
    fn start_game_success() {
        let mut lobby = Lobby::new();
        lobby.add_player("Alice".to_string()).unwrap();
        lobby.add_player("Bob".to_string()).unwrap();

        lobby.start_game(1, 3, 1, 1).unwrap();
        assert!(lobby.game.is_some());
        assert!(lobby.lobby_state().game_active);
    }

    #[test]
    fn lobby_state_hides_tokens() {
        let mut lobby = Lobby::new();
        lobby.add_player("Alice".to_string()).unwrap();

        let state = lobby.lobby_state();
        assert_eq!(state.players.len(), 1);
        assert_eq!(state.players[0].name, "Alice");
        // LobbyPlayerView has no session_token field
    }

    #[test]
    fn game_result_recorded() {
        let mut lobby = Lobby::new();
        lobby.add_player("Alice".to_string()).unwrap();
        lobby.add_player("Bob".to_string()).unwrap();
        lobby.start_game(1, 3, 1, 1).unwrap();

        lobby.record_game_result(1);
        assert!(lobby.game.is_none());
        assert_eq!(lobby.game_history.len(), 1);
        assert_eq!(lobby.game_history[0].winner_name, "Alice");
        assert_eq!(lobby.game_history[0].game_number, 1);
    }

    #[test]
    fn player_game_state_hides_other_hands() {
        let mut lobby = Lobby::new();
        lobby.add_player("Alice".to_string()).unwrap();
        lobby.add_player("Bob".to_string()).unwrap();
        lobby.start_game(1, 3, 1, 1).unwrap();

        let state = lobby.player_game_state(1).unwrap();
        // Alice can see her own card
        assert!(state.your_card.is_some());
        // Other players' cards are not revealed during play
        for p in &state.players {
            if p.id != 1 {
                assert!(p.revealed_card.is_none());
            }
        }
    }
}
