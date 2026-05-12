// Mirrors the Rust types from lobby.rs and game.rs

export type PlayerId = number;

export interface Card {
  rank: string;
  suit: string;
}

// --- Lobby ---

export interface LobbyPlayerView {
  id: PlayerId;
  name: string;
  is_connected: boolean;
  is_host: boolean;
}

export interface GameResult {
  game_number: number;
  player_count: number;
  round_count: number;
  winner_name: string;
}

export interface LobbyState {
  code: string;
  players: LobbyPlayerView[];
  host_id: PlayerId;
  game_active: boolean;
  game_history: GameResult[];
}

// --- Game State (per-player view) ---

export interface PlayerPublicState {
  id: PlayerId;
  name: string;
  lives: number;
  has_blocker: boolean;
  has_exemption: boolean;
  is_eliminated: boolean;
  revealed_card: Card | null;
}

export interface PlayerGameState {
  round_number: number;
  your_card: Card | null;
  your_lives: number;
  your_exemption_tokens: number;
  your_blocker_tokens: number;
  players: PlayerPublicState[];
  phase: string;
  is_cowboy_round: boolean;
  dealer_id: PlayerId;
  current_actor: PlayerId | null;
}

// --- Server Messages ---

export type ServerMessage =
  | { type: "LobbyUpdate"; state: LobbyState }
  | { type: "GameState"; state: PlayerGameState }
  | { type: "GameEvents"; events: GameEvent[] }
  | { type: "Error"; message: string }
  | { type: "LobbyClosed" }
  | { type: "Welcome"; player_id: PlayerId; session_token: string }
  | {
      type: "ActionRequired";
      player_id: PlayerId;
      valid_actions: string[];
      timeout_secs: number | null;
    };

// --- Game Events ---

export type GameEvent =
  | { type: "GameStarted"; dealer_id: PlayerId }
  | { type: "RoundStarted"; round_number: number; dealer_id: PlayerId }
  | { type: "CardDealt"; player_id: PlayerId }
  | {
      type: "CowboyTriggered";
      king_holders: PlayerId[];
      source: "Deal" | "DealerDraw";
    }
  | { type: "NormalTurnStarted"; player_id: PlayerId }
  | { type: "PlayerPassed"; player_id: PlayerId }
  | { type: "TradeProposed"; from_id: PlayerId; to_id: PlayerId }
  | { type: "TradeBlocked"; blocker_id: PlayerId }
  | { type: "TradeCompleted"; from_id: PlayerId; to_id: PlayerId }
  | { type: "DealerTurnStarted"; dealer_id: PlayerId }
  | { type: "DealerPassed"; dealer_id: PlayerId }
  | { type: "DealerDrewCard"; dealer_id: PlayerId; card: Card }
  | { type: "CowboyVoteStarted" }
  | { type: "PlayerExempted"; player_id: PlayerId }
  | { type: "PlayerStayedIn"; player_id: PlayerId }
  | {
      type: "ShowdownResult";
      reveals: [PlayerId, Card][];
      losers: PlayerId[];
      is_cowboy_round: boolean;
    }
  | { type: "PlayerEliminated"; player_id: PlayerId }
  | { type: "EveryoneExempted" }
  | { type: "ResurrectionTriggered" }
  | { type: "GameWon"; winner_id: PlayerId }
  | { type: "DealerChipPassed"; new_dealer_id: PlayerId };

// --- Client Messages ---

export type ClientMessage =
  | {
      type: "StartGame";
      lives: number;
      blocker_tokens: number;
      exemption_tokens: number;
    }
  | { type: "GameAction"; action: GameAction }
  | { type: "TransferHost"; to_player_id: PlayerId }
  | { type: "KickPlayer"; player_id: PlayerId }
  | { type: "EndGame" }
  | { type: "EndLobby" };

export type GameAction =
  | { Pass: null }
  | { Trade: null }
  | { Block: null }
  | { AcceptTrade: null }
  | { DealerPass: null }
  | { TakeOffTop: null }
  | { CowboyVote: { exempt: boolean } }
  | { ResolveCowboyVote: null }
  | { NextRound: null };
