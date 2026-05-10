use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::card::{Card, Deck};
use crate::player::{Player, PlayerId};

#[derive(Debug, Error)]
pub enum GameError {
    #[error("Not enough players to start (need at least 2)")]
    NotEnoughPlayers,
    #[error("Game is not in lobby state")]
    NotInLobby,
    #[error("It is not player {0}'s turn")]
    NotYourTurn(PlayerId),
    #[error("Invalid action for current game phase")]
    InvalidAction,
    #[error("Player {0} not found")]
    PlayerNotFound(PlayerId),
    #[error("Player {0} has no blocker token")]
    NoBlockerToken(PlayerId),
    #[error("Player {0} has no exemption token")]
    NoExemptionToken(PlayerId),
    #[error("Player {0} is eliminated")]
    PlayerEliminated(PlayerId),
    #[error("Game is already over")]
    GameOver,
}

/// The phase of the game state machine.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Phase {
    /// Waiting in lobby for host to start.
    Lobby,
    /// Cards have been dealt; checking for kings.
    KingCheck,
    /// Normal round: a non-dealer player is deciding to pass or trade.
    NormalTurn {
        /// Index into `alive_order` for the current actor.
        current_idx: usize,
    },
    /// A trade has been proposed; waiting for the target to block or accept.
    WaitingForBlock {
        /// The player proposing the trade.
        trader_idx: usize,
        /// The player who can block (to the left of trader).
        target_idx: usize,
    },
    /// Dealer's turn: pass or take off the top.
    DealerTurn,
    /// Cowboy round: collecting exemption votes (staged, not revealed).
    /// Votes are validated on submission but only processed on resolve.
    CowboyVote {
        /// Staged votes: player_id -> wants to exempt.
        votes: Vec<(PlayerId, bool)>,
    },
    /// All active players reveal cards and lowest loses a life.
    Showdown,
    /// Round is over, processing eliminations and preparing next round.
    RoundEnd,
    /// Game is over -- we have a winner (or resurrection triggered).
    GameOver { winner: Option<PlayerId> },
}

/// Events emitted by the game engine for the frontend to animate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GameEvent {
    GameStarted {
        dealer_id: PlayerId,
    },
    RoundStarted {
        round_number: u32,
        dealer_id: PlayerId,
    },
    CardDealt {
        player_id: PlayerId,
    },
    CowboyTriggered {
        king_holders: Vec<PlayerId>,
        source: CowboySource,
    },
    NormalTurnStarted {
        player_id: PlayerId,
    },
    PlayerPassed {
        player_id: PlayerId,
    },
    TradeProposed {
        from_id: PlayerId,
        to_id: PlayerId,
    },
    TradeBlocked {
        blocker_id: PlayerId,
    },
    TradeCompleted {
        from_id: PlayerId,
        to_id: PlayerId,
    },
    DealerTurnStarted {
        dealer_id: PlayerId,
    },
    DealerPassed {
        dealer_id: PlayerId,
    },
    DealerDrewCard {
        dealer_id: PlayerId,
        card: Card,
    },
    CowboyVoteStarted,
    PlayerExempted {
        player_id: PlayerId,
    },
    PlayerStayedIn {
        player_id: PlayerId,
    },
    ShowdownResult {
        reveals: Vec<(PlayerId, Card)>,
        losers: Vec<PlayerId>,
        is_cowboy_round: bool,
    },
    PlayerEliminated {
        player_id: PlayerId,
    },
    EveryoneExempted,
    ResurrectionTriggered,
    GameWon {
        winner_id: PlayerId,
    },
    DealerChipPassed {
        new_dealer_id: PlayerId,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CowboySource {
    Deal,
    DealerDraw,
}

/// Actions that clients can send to the game.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Action {
    /// Host starts the game with configuration.
    StartGame {
        lives: u8,
        blocker_tokens: u8,
        exemption_tokens: u8,
    },
    /// Player passes (keeps their card).
    Pass,
    /// Player proposes a trade with the player to their left.
    Trade,
    /// Target player blocks a proposed trade.
    Block,
    /// Target player accepts a proposed trade.
    AcceptTrade,
    /// Dealer passes (keeps their card).
    DealerPass,
    /// Dealer draws from the top of the deck.
    TakeOffTop,
    /// Player submits their cowboy exemption vote (staged, not yet applied).
    CowboyVote { exempt: bool },
    /// Server resolves the cowboy vote (called when timer expires).
    /// Players who haven't voted get auto-defaulted.
    ResolveCowboyVote,
    /// Advance from showdown to next round (after UI has shown results).
    NextRound,
}

pub struct Game {
    pub players: Vec<Player>,
    pub phase: Phase,
    pub deck: Deck,
    pub round_number: u32,
    /// Index into `players` for who has the dealer chip.
    pub dealer_idx: usize,
    /// Order of alive players for the current round (indices into `players`).
    pub alive_order: Vec<usize>,
    /// Whether the current round is a cowboy round.
    pub is_cowboy_round: bool,
    /// Players who exempted this cowboy round (indices into `players`).
    pub exempted: Vec<usize>,
    /// Accumulated events from the last action.
    pub events: Vec<GameEvent>,
    /// All players who originally joined (for resurrection).
    pub original_player_count: usize,
}

impl Game {
    pub fn new() -> Self {
        Self {
            players: Vec::new(),
            phase: Phase::Lobby,
            deck: Deck::new(),
            round_number: 0,
            dealer_idx: 0,
            alive_order: Vec::new(),
            is_cowboy_round: false,
            exempted: Vec::new(),
            events: Vec::new(),
            original_player_count: 0,
        }
    }

    pub fn add_player(&mut self, id: PlayerId, name: String) -> Result<(), GameError> {
        if self.phase != Phase::Lobby {
            return Err(GameError::NotInLobby);
        }
        self.players.push(Player::new(id, name, 0, 0, 0));
        Ok(())
    }

    /// Take accumulated events, clearing the internal buffer.
    pub fn take_events(&mut self) -> Vec<GameEvent> {
        std::mem::take(&mut self.events)
    }

    /// Find a player's index by their ID.
    fn player_idx(&self, id: PlayerId) -> Result<usize, GameError> {
        self.players
            .iter()
            .position(|p| p.id == id)
            .ok_or(GameError::PlayerNotFound(id))
    }

    /// Get the alive player order starting left of dealer, ending with dealer.
    fn compute_alive_order(&self) -> Vec<usize> {
        let n = self.players.len();
        let mut order = Vec::new();
        for i in 1..=n {
            let idx = (self.dealer_idx + i) % n;
            if self.players[idx].is_alive() {
                order.push(idx);
            }
        }
        // Dealer should be last if alive
        if self.players[self.dealer_idx].is_alive() {
            // Remove dealer from wherever they are and push to end
            order.retain(|&idx| idx != self.dealer_idx);
            order.push(self.dealer_idx);
        }
        order
    }

    /// Process an action from a player.
    pub fn act(&mut self, player_id: PlayerId, action: Action) -> Result<(), GameError> {
        self.events.clear();
        match action {
            Action::StartGame {
                lives,
                blocker_tokens,
                exemption_tokens,
            } => self.start_game(lives, blocker_tokens, exemption_tokens),
            Action::Pass => self.handle_pass(player_id),
            Action::Trade => self.handle_trade(player_id),
            Action::Block => self.handle_block(player_id),
            Action::AcceptTrade => self.handle_accept_trade(player_id),
            Action::DealerPass => self.handle_dealer_pass(player_id),
            Action::TakeOffTop => self.handle_take_off_top(player_id),
            Action::CowboyVote { exempt } => self.handle_cowboy_vote(player_id, exempt),
            Action::ResolveCowboyVote => self.handle_resolve_cowboy_vote(),
            Action::NextRound => self.handle_next_round(),
        }
    }

    fn start_game(
        &mut self,
        lives: u8,
        blocker_tokens: u8,
        exemption_tokens: u8,
    ) -> Result<(), GameError> {
        if self.phase != Phase::Lobby {
            return Err(GameError::NotInLobby);
        }
        if self.players.len() < 2 {
            return Err(GameError::NotEnoughPlayers);
        }

        // If 1 life, no exemption tokens
        let actual_exemptions = if lives == 1 { 0 } else { exemption_tokens };

        for player in &mut self.players {
            player.lives = lives;
            player.blocker_tokens = blocker_tokens;
            player.exemption_tokens = actual_exemptions;
            player.is_eliminated = false;
        }

        self.original_player_count = self.players.len();

        // Random dealer
        use rand::Rng;
        self.dealer_idx = rand::rng().random_range(0..self.players.len());

        self.events.push(GameEvent::GameStarted {
            dealer_id: self.players[self.dealer_idx].id,
        });

        self.start_round();
        Ok(())
    }

    fn start_round(&mut self) {
        self.round_number += 1;
        self.is_cowboy_round = false;
        self.exempted.clear();
        self.alive_order = self.compute_alive_order();

        // Fresh deck and deal
        self.deck = Deck::new();
        self.deck.shuffle();

        self.events.push(GameEvent::RoundStarted {
            round_number: self.round_number,
            dealer_id: self.players[self.dealer_idx].id,
        });

        // Deal one card to each alive player in order (left of dealer first)
        for &player_idx in &self.alive_order {
            let card = self.deck.deal_one().expect("Deck should have enough cards");
            self.players[player_idx].hand = Some(card);
            self.events.push(GameEvent::CardDealt {
                player_id: self.players[player_idx].id,
            });
        }

        // Check for kings
        self.phase = Phase::KingCheck;
        self.check_for_kings();
    }

    fn check_for_kings(&mut self) {
        let king_holders: Vec<usize> = self
            .alive_order
            .iter()
            .filter(|&&idx| self.players[idx].hand.map(|c| c.is_king()).unwrap_or(false))
            .copied()
            .collect();

        if !king_holders.is_empty() {
            self.is_cowboy_round = true;
            let king_ids: Vec<PlayerId> = king_holders
                .iter()
                .map(|&idx| self.players[idx].id)
                .collect();

            self.events.push(GameEvent::CowboyTriggered {
                king_holders: king_ids,
                source: CowboySource::Deal,
            });

            self.start_cowboy_vote();
        } else {
            // Normal round: first non-dealer player acts
            self.start_normal_turn(0);
        }
    }

    fn start_normal_turn(&mut self, alive_order_idx: usize) {
        // The last player in alive_order is the dealer
        let dealer_alive_idx = self.alive_order.len() - 1;

        if alive_order_idx >= dealer_alive_idx {
            // It's the dealer's turn
            let dealer_player_idx = self.alive_order[dealer_alive_idx];
            self.phase = Phase::DealerTurn;
            self.events.push(GameEvent::DealerTurnStarted {
                dealer_id: self.players[dealer_player_idx].id,
            });
        } else {
            let player_idx = self.alive_order[alive_order_idx];
            self.phase = Phase::NormalTurn {
                current_idx: alive_order_idx,
            };
            self.events.push(GameEvent::NormalTurnStarted {
                player_id: self.players[player_idx].id,
            });
        }
    }

    fn handle_pass(&mut self, player_id: PlayerId) -> Result<(), GameError> {
        let current_idx = match &self.phase {
            Phase::NormalTurn { current_idx } => *current_idx,
            _ => return Err(GameError::InvalidAction),
        };

        let player_idx = self.alive_order[current_idx];
        if self.players[player_idx].id != player_id {
            return Err(GameError::NotYourTurn(player_id));
        }

        self.events.push(GameEvent::PlayerPassed { player_id });

        self.start_normal_turn(current_idx + 1);
        Ok(())
    }

    fn handle_trade(&mut self, player_id: PlayerId) -> Result<(), GameError> {
        let current_idx = match &self.phase {
            Phase::NormalTurn { current_idx } => *current_idx,
            _ => return Err(GameError::InvalidAction),
        };

        let player_idx = self.alive_order[current_idx];
        if self.players[player_idx].id != player_id {
            return Err(GameError::NotYourTurn(player_id));
        }

        // Target is the next player in alive_order
        let target_idx = current_idx + 1;
        let target_player_idx = self.alive_order[target_idx];

        self.events.push(GameEvent::TradeProposed {
            from_id: player_id,
            to_id: self.players[target_player_idx].id,
        });

        // If target can block, wait for their decision
        if self.players[target_player_idx].can_block() {
            self.phase = Phase::WaitingForBlock {
                trader_idx: current_idx,
                target_idx,
            };
        } else {
            // Forced trade
            self.execute_trade(player_idx, target_player_idx);
            self.start_normal_turn(current_idx + 1);
        }

        Ok(())
    }

    fn execute_trade(&mut self, from_idx: usize, to_idx: usize) {
        let from_card = self.players[from_idx].hand.take();
        let to_card = self.players[to_idx].hand.take();
        self.players[from_idx].hand = to_card;
        self.players[to_idx].hand = from_card;

        self.events.push(GameEvent::TradeCompleted {
            from_id: self.players[from_idx].id,
            to_id: self.players[to_idx].id,
        });
    }

    fn handle_block(&mut self, player_id: PlayerId) -> Result<(), GameError> {
        let (trader_idx, target_idx) = match &self.phase {
            Phase::WaitingForBlock {
                trader_idx,
                target_idx,
            } => (*trader_idx, *target_idx),
            _ => return Err(GameError::InvalidAction),
        };

        let target_player_idx = self.alive_order[target_idx];
        if self.players[target_player_idx].id != player_id {
            return Err(GameError::NotYourTurn(player_id));
        }

        if !self.players[target_player_idx].can_block() {
            return Err(GameError::NoBlockerToken(player_id));
        }

        self.players[target_player_idx].use_blocker();
        self.events.push(GameEvent::TradeBlocked {
            blocker_id: player_id,
        });

        // Trade canceled, move to next player's turn
        self.start_normal_turn(trader_idx + 1);
        Ok(())
    }

    fn handle_accept_trade(&mut self, player_id: PlayerId) -> Result<(), GameError> {
        let (trader_idx, target_idx) = match &self.phase {
            Phase::WaitingForBlock {
                trader_idx,
                target_idx,
            } => (*trader_idx, *target_idx),
            _ => return Err(GameError::InvalidAction),
        };

        let target_player_idx = self.alive_order[target_idx];
        if self.players[target_player_idx].id != player_id {
            return Err(GameError::NotYourTurn(player_id));
        }

        let trader_player_idx = self.alive_order[trader_idx];
        self.execute_trade(trader_player_idx, target_player_idx);
        self.start_normal_turn(trader_idx + 1);
        Ok(())
    }

    fn handle_dealer_pass(&mut self, player_id: PlayerId) -> Result<(), GameError> {
        if self.phase != Phase::DealerTurn {
            return Err(GameError::InvalidAction);
        }

        let dealer_player_idx = *self.alive_order.last().unwrap();
        if self.players[dealer_player_idx].id != player_id {
            return Err(GameError::NotYourTurn(player_id));
        }

        self.events.push(GameEvent::DealerPassed {
            dealer_id: player_id,
        });

        self.begin_showdown();
        Ok(())
    }

    fn handle_take_off_top(&mut self, player_id: PlayerId) -> Result<(), GameError> {
        if self.phase != Phase::DealerTurn {
            return Err(GameError::InvalidAction);
        }

        let dealer_player_idx = *self.alive_order.last().unwrap();
        if self.players[dealer_player_idx].id != player_id {
            return Err(GameError::NotYourTurn(player_id));
        }

        // Dealer discards their card and draws from deck
        self.players[dealer_player_idx].hand = None;
        let drawn = self
            .deck
            .deal_one()
            .expect("Deck should have cards remaining");
        self.players[dealer_player_idx].hand = Some(drawn);

        self.events.push(GameEvent::DealerDrewCard {
            dealer_id: player_id,
            card: drawn,
        });

        if drawn.is_king() {
            // Cowboy triggered by dealer draw!
            self.is_cowboy_round = true;
            self.events.push(GameEvent::CowboyTriggered {
                king_holders: vec![player_id],
                source: CowboySource::DealerDraw,
            });
            self.start_cowboy_vote();
        } else {
            self.begin_showdown();
        }

        Ok(())
    }

    fn start_cowboy_vote(&mut self) {
        self.events.push(GameEvent::CowboyVoteStarted);
        self.phase = Phase::CowboyVote { votes: Vec::new() };
    }

    fn handle_cowboy_vote(&mut self, player_id: PlayerId, exempt: bool) -> Result<(), GameError> {
        if !matches!(self.phase, Phase::CowboyVote { .. }) {
            return Err(GameError::InvalidAction);
        }

        let player_idx = self.player_idx(player_id)?;

        if !self.players[player_idx].is_alive() {
            return Err(GameError::PlayerEliminated(player_id));
        }

        // Check for duplicate vote
        if let Phase::CowboyVote { votes } = &self.phase {
            if votes.iter().any(|(id, _)| *id == player_id) {
                return Err(GameError::InvalidAction);
            }
        }

        // Validate they CAN exempt before staging
        if exempt && !self.players[player_idx].can_exempt() {
            return Err(GameError::NoExemptionToken(player_id));
        }

        // Stage the vote -- no events emitted, no tokens consumed yet
        if let Phase::CowboyVote { votes } = &mut self.phase {
            votes.push((player_id, exempt));
        }

        Ok(())
    }

    /// Resolve the cowboy vote: apply defaults for missing voters, consume tokens,
    /// emit all events at once, then advance to showdown.
    fn handle_resolve_cowboy_vote(&mut self) -> Result<(), GameError> {
        if !matches!(self.phase, Phase::CowboyVote { .. }) {
            return Err(GameError::InvalidAction);
        }

        // Extract staged votes
        let staged_votes = if let Phase::CowboyVote { votes } = &self.phase {
            votes.clone()
        } else {
            unreachable!()
        };

        // Build final votes: fill in defaults for players who didn't vote
        let mut final_votes: Vec<(PlayerId, bool)> = staged_votes;
        for &player_idx in &self.alive_order {
            let pid = self.players[player_idx].id;
            if !final_votes.iter().any(|(id, _)| *id == pid) {
                // Default: exempt if they have a token, otherwise stay in
                let default_exempt = self.players[player_idx].can_exempt();
                final_votes.push((pid, default_exempt));
            }
        }

        // Now apply all votes: consume tokens, build exempted list, emit events
        for &(pid, exempt) in &final_votes {
            let player_idx = self.player_idx(pid).unwrap();
            if exempt {
                self.players[player_idx].use_exemption();
                self.exempted.push(player_idx);
                self.events
                    .push(GameEvent::PlayerExempted { player_id: pid });
            } else {
                self.events
                    .push(GameEvent::PlayerStayedIn { player_id: pid });
            }
        }

        self.begin_showdown();
        Ok(())
    }

    fn begin_showdown(&mut self) {
        self.phase = Phase::Showdown;

        // Determine active players (alive and not exempted)
        let active_indices: Vec<usize> = self
            .alive_order
            .iter()
            .filter(|idx| !self.exempted.contains(idx))
            .copied()
            .collect();

        // If everyone exempted in a cowboy round, no one loses
        if active_indices.is_empty() {
            self.events.push(GameEvent::EveryoneExempted);
            self.phase = Phase::RoundEnd;
            return;
        }

        // Reveal cards and find the lowest
        let mut reveals: Vec<(PlayerId, Card)> = Vec::new();
        for &idx in &active_indices {
            if let Some(card) = self.players[idx].hand {
                reveals.push((self.players[idx].id, card));
            }
        }

        // Find lowest value
        let min_value = if self.is_cowboy_round {
            reveals
                .iter()
                .map(|(_, card)| card.rank.cowboy_value())
                .min()
                .unwrap()
        } else {
            reveals
                .iter()
                .map(|(_, card)| card.rank.normal_value().unwrap())
                .min()
                .unwrap()
        };

        // Find all players with the lowest value
        let losers: Vec<PlayerId> = reveals
            .iter()
            .filter(|(_, card)| {
                if self.is_cowboy_round {
                    card.rank.cowboy_value() == min_value
                } else {
                    card.rank.normal_value().unwrap() == min_value
                }
            })
            .map(|(id, _)| *id)
            .collect();

        self.events.push(GameEvent::ShowdownResult {
            reveals,
            losers: losers.clone(),
            is_cowboy_round: self.is_cowboy_round,
        });

        // Apply life loss
        for &loser_id in &losers {
            let idx = self.player_idx(loser_id).unwrap();
            self.players[idx].lose_life();
            if !self.players[idx].is_alive() {
                self.events.push(GameEvent::PlayerEliminated {
                    player_id: loser_id,
                });
            }
        }

        self.phase = Phase::RoundEnd;
    }

    fn handle_next_round(&mut self) -> Result<(), GameError> {
        if self.phase != Phase::RoundEnd {
            return Err(GameError::InvalidAction);
        }

        // Clear hands
        for player in &mut self.players {
            player.hand = None;
        }

        let alive: Vec<usize> = self
            .players
            .iter()
            .enumerate()
            .filter(|(_, p)| p.is_alive())
            .map(|(i, _)| i)
            .collect();

        if alive.len() == 1 {
            let winner_id = self.players[alive[0]].id;
            self.events.push(GameEvent::GameWon { winner_id });
            self.phase = Phase::GameOver {
                winner: Some(winner_id),
            };
            return Ok(());
        }

        if alive.is_empty() {
            // Resurrection! All players come back with 1 life.
            // Players who were alive this round (the ones who tied) keep their tokens.
            // Previously eliminated players come back with 0 tokens.
            self.events.push(GameEvent::ResurrectionTriggered);

            // alive_order contains the players who were alive at the start of this round
            let was_alive_this_round: Vec<usize> = self.alive_order.clone();

            for (i, player) in self.players.iter_mut().enumerate() {
                if was_alive_this_round.contains(&i) {
                    // They just died in the tie -- give them 1 life, keep tokens
                    player.lives = 1;
                    player.is_eliminated = false;
                } else {
                    // Previously eliminated -- come back with 1 life, 0 tokens
                    player.resurrect();
                }
            }
        }

        // Advance dealer chip to next alive player
        self.advance_dealer();
        self.events.push(GameEvent::DealerChipPassed {
            new_dealer_id: self.players[self.dealer_idx].id,
        });

        self.start_round();
        Ok(())
    }

    fn advance_dealer(&mut self) {
        let n = self.players.len();
        let mut next = (self.dealer_idx + 1) % n;
        while !self.players[next].is_alive() {
            next = (next + 1) % n;
        }
        self.dealer_idx = next;
    }

    // --- Public query helpers ---

    /// Who is currently expected to act?
    pub fn current_actor(&self) -> Option<PlayerId> {
        match &self.phase {
            Phase::NormalTurn { current_idx } => {
                let player_idx = self.alive_order[*current_idx];
                Some(self.players[player_idx].id)
            }
            Phase::WaitingForBlock { target_idx, .. } => {
                let player_idx = self.alive_order[*target_idx];
                Some(self.players[player_idx].id)
            }
            Phase::DealerTurn => {
                let dealer_player_idx = *self.alive_order.last().unwrap();
                Some(self.players[dealer_player_idx].id)
            }
            _ => None,
        }
    }

    pub fn alive_players(&self) -> Vec<&Player> {
        self.players.iter().filter(|p| p.is_alive()).collect()
    }

    pub fn alive_count(&self) -> usize {
        self.players.iter().filter(|p| p.is_alive()).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{Card, Rank, Suit};

    /// Helper to create a game with N players in lobby.
    fn setup_lobby(n: usize) -> Game {
        let mut game = Game::new();
        for i in 0..n {
            game.add_player(i as u64 + 1, format!("Player{}", i + 1))
                .unwrap();
        }
        game
    }

    /// Helper to start a game with default settings.
    fn start_default(game: &mut Game) {
        game.act(
            0,
            Action::StartGame {
                lives: 3,
                blocker_tokens: 1,
                exemption_tokens: 1,
            },
        )
        .unwrap();
    }

    #[test]
    fn cannot_start_with_one_player() {
        let mut game = setup_lobby(1);
        let result = game.act(
            0,
            Action::StartGame {
                lives: 3,
                blocker_tokens: 1,
                exemption_tokens: 1,
            },
        );
        assert!(result.is_err());
    }

    #[test]
    fn cannot_add_player_after_start() {
        let mut game = setup_lobby(3);
        start_default(&mut game);
        let result = game.add_player(99, "Late".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn game_starts_and_deals_cards() {
        let mut game = setup_lobby(4);
        start_default(&mut game);

        // All alive players should have a card
        for player in &game.players {
            assert!(player.hand.is_some());
            assert_eq!(player.lives, 3);
        }

        // Phase should be either KingCheck resolved to NormalTurn/CowboyVote
        assert!(
            matches!(game.phase, Phase::NormalTurn { .. })
                || matches!(game.phase, Phase::CowboyVote { .. })
                || matches!(game.phase, Phase::DealerTurn)
        );
    }

    #[test]
    fn one_life_removes_exemption_tokens() {
        let mut game = setup_lobby(3);
        game.act(
            0,
            Action::StartGame {
                lives: 1,
                blocker_tokens: 1,
                exemption_tokens: 1,
            },
        )
        .unwrap();

        for player in &game.players {
            assert_eq!(player.exemption_tokens, 0);
            assert_eq!(player.lives, 1);
        }
    }

    #[test]
    fn normal_round_pass_advances_turn() {
        let mut game = setup_lobby(3);
        start_default(&mut game);

        // Force a normal round by rigging no kings
        for player in &mut game.players {
            player.hand = Some(Card::new(Rank::Five, Suit::Hearts));
        }
        game.is_cowboy_round = false;
        // Set to first player's turn
        game.phase = Phase::NormalTurn { current_idx: 0 };

        let first_player_id = game.players[game.alive_order[0]].id;
        let second_player_id = game.players[game.alive_order[1]].id;

        game.act(first_player_id, Action::Pass).unwrap();

        // Should now be second player's turn (or dealer turn if only 2 non-dealer)
        match &game.phase {
            Phase::NormalTurn { current_idx } => {
                let next_id = game.players[game.alive_order[*current_idx]].id;
                assert_eq!(next_id, second_player_id);
            }
            Phase::DealerTurn => {
                // If there were only 2 alive + dealer, second player IS dealer
                assert_eq!(
                    second_player_id,
                    game.players[*game.alive_order.last().unwrap()].id
                );
            }
            _ => panic!("Unexpected phase: {:?}", game.phase),
        }
    }

    #[test]
    fn wrong_player_cannot_act() {
        let mut game = setup_lobby(3);
        start_default(&mut game);

        // Rig normal round
        for player in &mut game.players {
            player.hand = Some(Card::new(Rank::Five, Suit::Hearts));
        }
        game.is_cowboy_round = false;
        game.phase = Phase::NormalTurn { current_idx: 0 };

        let wrong_player_id = game.players[game.alive_order[1]].id;
        let result = game.act(wrong_player_id, Action::Pass);
        assert!(result.is_err());
    }

    #[test]
    fn trade_without_blocker_auto_completes() {
        let mut game = setup_lobby(3);
        start_default(&mut game);

        // Remove all blocker tokens
        for player in &mut game.players {
            player.blocker_tokens = 0;
        }

        // Rig hands for a normal round
        let card_a = Card::new(Rank::Two, Suit::Hearts);
        let card_b = Card::new(Rank::Queen, Suit::Spades);
        let card_c = Card::new(Rank::Five, Suit::Clubs);

        let idx_0 = game.alive_order[0];
        let idx_1 = game.alive_order[1];
        let idx_2 = game.alive_order[2]; // dealer

        game.players[idx_0].hand = Some(card_a);
        game.players[idx_1].hand = Some(card_b);
        game.players[idx_2].hand = Some(card_c);
        game.is_cowboy_round = false;
        game.phase = Phase::NormalTurn { current_idx: 0 };

        let trader_id = game.players[idx_0].id;
        game.act(trader_id, Action::Trade).unwrap();

        // Cards should have swapped
        assert_eq!(game.players[idx_0].hand.unwrap().rank, Rank::Queen);
        assert_eq!(game.players[idx_1].hand.unwrap().rank, Rank::Two);
    }

    #[test]
    fn trade_with_blocker_goes_to_waiting() {
        let mut game = setup_lobby(3);
        start_default(&mut game);

        // Rig normal round
        for player in &mut game.players {
            player.hand = Some(Card::new(Rank::Five, Suit::Hearts));
            player.blocker_tokens = 1;
        }
        game.is_cowboy_round = false;
        game.phase = Phase::NormalTurn { current_idx: 0 };

        let trader_id = game.players[game.alive_order[0]].id;
        game.act(trader_id, Action::Trade).unwrap();

        assert!(matches!(game.phase, Phase::WaitingForBlock { .. }));
    }

    #[test]
    fn block_cancels_trade() {
        let mut game = setup_lobby(3);
        start_default(&mut game);

        let card_a = Card::new(Rank::Two, Suit::Hearts);
        let card_b = Card::new(Rank::Queen, Suit::Spades);

        let idx_0 = game.alive_order[0];
        let idx_1 = game.alive_order[1];

        game.players[idx_0].hand = Some(card_a);
        game.players[idx_1].hand = Some(card_b);
        game.players[idx_1].blocker_tokens = 1;
        game.is_cowboy_round = false;
        game.phase = Phase::NormalTurn { current_idx: 0 };

        let trader_id = game.players[idx_0].id;
        let blocker_id = game.players[idx_1].id;

        game.act(trader_id, Action::Trade).unwrap();
        assert!(matches!(game.phase, Phase::WaitingForBlock { .. }));

        game.act(blocker_id, Action::Block).unwrap();

        // Cards should NOT have swapped
        assert_eq!(game.players[idx_0].hand.unwrap().rank, Rank::Two);
        assert_eq!(game.players[idx_1].hand.unwrap().rank, Rank::Queen);
        assert_eq!(game.players[idx_1].blocker_tokens, 0);
    }

    #[test]
    fn accept_trade_swaps_cards() {
        let mut game = setup_lobby(3);
        start_default(&mut game);

        let card_a = Card::new(Rank::Two, Suit::Hearts);
        let card_b = Card::new(Rank::Queen, Suit::Spades);

        let idx_0 = game.alive_order[0];
        let idx_1 = game.alive_order[1];

        game.players[idx_0].hand = Some(card_a);
        game.players[idx_1].hand = Some(card_b);
        game.players[idx_1].blocker_tokens = 1;
        game.is_cowboy_round = false;
        game.phase = Phase::NormalTurn { current_idx: 0 };

        let trader_id = game.players[idx_0].id;
        let target_id = game.players[idx_1].id;

        game.act(trader_id, Action::Trade).unwrap();
        game.act(target_id, Action::AcceptTrade).unwrap();

        assert_eq!(game.players[idx_0].hand.unwrap().rank, Rank::Queen);
        assert_eq!(game.players[idx_1].hand.unwrap().rank, Rank::Two);
    }

    #[test]
    fn dealer_pass_triggers_showdown() {
        let mut game = setup_lobby(3);
        start_default(&mut game);

        // Rig normal round
        let idx_0 = game.alive_order[0];
        let idx_1 = game.alive_order[1];
        let idx_2 = game.alive_order[2];

        game.players[idx_0].hand = Some(Card::new(Rank::Queen, Suit::Hearts));
        game.players[idx_1].hand = Some(Card::new(Rank::Five, Suit::Spades));
        game.players[idx_2].hand = Some(Card::new(Rank::Ace, Suit::Clubs));
        game.is_cowboy_round = false;
        game.phase = Phase::DealerTurn;

        let dealer_id = game.players[idx_2].id;
        game.act(dealer_id, Action::DealerPass).unwrap();

        assert_eq!(game.phase, Phase::RoundEnd);

        // idx_2 (dealer) has Ace, which is lowest in normal mode
        assert_eq!(game.players[idx_2].lives, 2);
        assert_eq!(game.players[idx_0].lives, 3);
        assert_eq!(game.players[idx_1].lives, 3);
    }

    #[test]
    fn cowboy_vote_exempt_saves_player() {
        let mut game = setup_lobby(3);
        start_default(&mut game);

        let idx_0 = game.alive_order[0];
        let idx_1 = game.alive_order[1];
        let idx_2 = game.alive_order[2];

        // Player 0 has king, others have low cards
        game.players[idx_0].hand = Some(Card::new(Rank::King, Suit::Spades));
        game.players[idx_1].hand = Some(Card::new(Rank::Two, Suit::Hearts));
        game.players[idx_2].hand = Some(Card::new(Rank::Five, Suit::Clubs));
        game.is_cowboy_round = true;
        game.phase = Phase::CowboyVote { votes: Vec::new() };
        game.exempted.clear();

        let id_0 = game.players[idx_0].id;
        let id_1 = game.players[idx_1].id;
        let id_2 = game.players[idx_2].id;

        // Player 1 exempts (has token), player 0 and 2 stay in
        game.act(id_1, Action::CowboyVote { exempt: true }).unwrap();
        game.act(id_0, Action::CowboyVote { exempt: false })
            .unwrap();
        game.act(id_2, Action::CowboyVote { exempt: false })
            .unwrap();
        game.act(0, Action::ResolveCowboyVote).unwrap();

        // Showdown: in cowboy mode, 2 is lowest.
        // Active: idx_0 (King=11), idx_2 (Five=3). idx_2 has lowest.
        assert_eq!(game.phase, Phase::RoundEnd);
        assert_eq!(game.players[idx_2].lives, 2); // lost a life
        assert_eq!(game.players[idx_0].lives, 3); // king holder safe
        assert_eq!(game.players[idx_1].lives, 3); // exempted, safe
    }

    #[test]
    fn lone_cowboy_rule() {
        let mut game = setup_lobby(3);
        start_default(&mut game);

        let idx_0 = game.alive_order[0];
        let idx_1 = game.alive_order[1];
        let idx_2 = game.alive_order[2];

        game.players[idx_0].hand = Some(Card::new(Rank::King, Suit::Spades));
        game.players[idx_1].hand = Some(Card::new(Rank::Two, Suit::Hearts));
        game.players[idx_2].hand = Some(Card::new(Rank::Five, Suit::Clubs));
        game.is_cowboy_round = true;
        game.phase = Phase::CowboyVote { votes: Vec::new() };
        game.exempted.clear();

        let id_0 = game.players[idx_0].id;
        let id_1 = game.players[idx_1].id;
        let id_2 = game.players[idx_2].id;

        // Everyone exempts except the king holder
        game.act(id_1, Action::CowboyVote { exempt: true }).unwrap();
        game.act(id_2, Action::CowboyVote { exempt: true }).unwrap();
        game.act(id_0, Action::CowboyVote { exempt: false })
            .unwrap();
        game.act(0, Action::ResolveCowboyVote).unwrap();

        // King is the only active card -- it's the lowest by default
        assert_eq!(game.phase, Phase::RoundEnd);
        assert_eq!(game.players[idx_0].lives, 2); // king holder loses!
        assert_eq!(game.players[idx_1].lives, 3); // safe
        assert_eq!(game.players[idx_2].lives, 3); // safe
    }

    #[test]
    fn everyone_exempts_no_one_loses() {
        let mut game = setup_lobby(3);
        start_default(&mut game);

        let idx_0 = game.alive_order[0];
        let idx_1 = game.alive_order[1];
        let idx_2 = game.alive_order[2];

        game.players[idx_0].hand = Some(Card::new(Rank::King, Suit::Spades));
        game.players[idx_1].hand = Some(Card::new(Rank::Two, Suit::Hearts));
        game.players[idx_2].hand = Some(Card::new(Rank::Five, Suit::Clubs));
        game.is_cowboy_round = true;
        game.phase = Phase::CowboyVote { votes: Vec::new() };
        game.exempted.clear();

        // Give everyone exemption tokens
        for p in &mut game.players {
            p.exemption_tokens = 1;
        }

        let id_0 = game.players[idx_0].id;
        let id_1 = game.players[idx_1].id;
        let id_2 = game.players[idx_2].id;

        game.act(id_0, Action::CowboyVote { exempt: true }).unwrap();
        game.act(id_1, Action::CowboyVote { exempt: true }).unwrap();
        game.act(id_2, Action::CowboyVote { exempt: true }).unwrap();
        game.act(0, Action::ResolveCowboyVote).unwrap();

        assert_eq!(game.phase, Phase::RoundEnd);
        // No one lost a life
        for p in &game.players {
            assert_eq!(p.lives, 3);
        }
    }

    #[test]
    fn elimination_and_game_over() {
        let mut game = setup_lobby(2);
        game.act(
            0,
            Action::StartGame {
                lives: 1,
                blocker_tokens: 0,
                exemption_tokens: 0,
            },
        )
        .unwrap();

        let idx_0 = game.alive_order[0];
        let idx_1 = game.alive_order[1];

        // Rig: player 0 has Ace (lowest), player 1 has Queen (highest)
        game.players[idx_0].hand = Some(Card::new(Rank::Ace, Suit::Hearts));
        game.players[idx_1].hand = Some(Card::new(Rank::Queen, Suit::Spades));
        game.is_cowboy_round = false;
        game.phase = Phase::DealerTurn;

        let dealer_id = game.players[*game.alive_order.last().unwrap()].id;
        game.act(dealer_id, Action::DealerPass).unwrap();

        // The player with Ace should be eliminated
        let ace_holder = &game.players[idx_0];
        assert!(!ace_holder.is_alive());

        game.act(0, Action::NextRound).unwrap();

        // Game should be over
        assert!(matches!(game.phase, Phase::GameOver { .. }));
    }

    #[test]
    fn resurrection_when_all_die() {
        let mut game = setup_lobby(2);
        game.act(
            0,
            Action::StartGame {
                lives: 1,
                blocker_tokens: 0,
                exemption_tokens: 0,
            },
        )
        .unwrap();

        let idx_0 = game.alive_order[0];
        let idx_1 = game.alive_order[1];

        // Both have the same card -- they both lose
        game.players[idx_0].hand = Some(Card::new(Rank::Five, Suit::Hearts));
        game.players[idx_1].hand = Some(Card::new(Rank::Five, Suit::Spades));
        game.is_cowboy_round = false;
        game.phase = Phase::DealerTurn;

        let dealer_id = game.players[*game.alive_order.last().unwrap()].id;
        game.act(dealer_id, Action::DealerPass).unwrap();

        // Both should have lost their life
        assert_eq!(game.players[idx_0].lives, 0);
        assert_eq!(game.players[idx_1].lives, 0);

        game.act(0, Action::NextRound).unwrap();

        // Resurrection! Both should be back with 1 life
        let events = game.take_events();
        assert!(
            events
                .iter()
                .any(|e| matches!(e, GameEvent::ResurrectionTriggered))
        );

        for player in &game.players {
            assert!(player.is_alive());
            assert_eq!(player.lives, 1);
            assert_eq!(player.exemption_tokens, 0);
            assert_eq!(player.blocker_tokens, 0);
        }
    }

    #[test]
    fn multiple_losers_with_tied_lowest() {
        let mut game = setup_lobby(4);
        start_default(&mut game);

        let idx_0 = game.alive_order[0];
        let idx_1 = game.alive_order[1];
        let idx_2 = game.alive_order[2];
        let idx_3 = game.alive_order[3];

        // Two players tie for lowest
        game.players[idx_0].hand = Some(Card::new(Rank::Ace, Suit::Hearts));
        game.players[idx_1].hand = Some(Card::new(Rank::Ace, Suit::Spades));
        game.players[idx_2].hand = Some(Card::new(Rank::Queen, Suit::Clubs));
        game.players[idx_3].hand = Some(Card::new(Rank::Jack, Suit::Diamonds));
        game.is_cowboy_round = false;
        game.phase = Phase::DealerTurn;

        let dealer_id = game.players[*game.alive_order.last().unwrap()].id;
        game.act(dealer_id, Action::DealerPass).unwrap();

        // Both Ace holders should lose a life
        assert_eq!(game.players[idx_0].lives, 2);
        assert_eq!(game.players[idx_1].lives, 2);
        assert_eq!(game.players[idx_2].lives, 3);
        assert_eq!(game.players[idx_3].lives, 3);
    }

    #[test]
    fn cannot_exempt_without_token() {
        let mut game = setup_lobby(3);
        start_default(&mut game);

        let idx_0 = game.alive_order[0];
        game.players[idx_0].exemption_tokens = 0;
        game.is_cowboy_round = true;
        game.phase = Phase::CowboyVote { votes: Vec::new() };

        let id_0 = game.players[idx_0].id;
        let result = game.act(id_0, Action::CowboyVote { exempt: true });
        assert!(result.is_err());
    }

    #[test]
    fn cannot_vote_twice() {
        let mut game = setup_lobby(3);
        start_default(&mut game);

        for p in &mut game.players {
            p.exemption_tokens = 1;
        }
        game.is_cowboy_round = true;
        game.phase = Phase::CowboyVote { votes: Vec::new() };

        let id_0 = game.players[game.alive_order[0]].id;
        game.act(id_0, Action::CowboyVote { exempt: false })
            .unwrap();
        let result = game.act(id_0, Action::CowboyVote { exempt: false });
        assert!(result.is_err());
    }

    #[test]
    fn dealer_draw_king_forces_cowboy_round() {
        // Deterministic test: make deck cards accessible
        let mut game = setup_lobby(3);
        start_default(&mut game);

        let idx_0 = game.alive_order[0];
        let idx_1 = game.alive_order[1];
        let idx_2 = game.alive_order[2];

        game.players[idx_0].hand = Some(Card::new(Rank::Five, Suit::Hearts));
        game.players[idx_1].hand = Some(Card::new(Rank::Jack, Suit::Spades));
        game.players[idx_2].hand = Some(Card::new(Rank::Three, Suit::Clubs));
        game.is_cowboy_round = false;
        game.phase = Phase::DealerTurn;

        // Drain the deck and put only a King in it
        while game.deck.deal_one().is_some() {}
        game.deck.push(Card::new(Rank::King, Suit::Diamonds));

        let dealer_id = game.players[idx_2].id;
        game.act(dealer_id, Action::TakeOffTop).unwrap();

        assert!(game.is_cowboy_round);
        assert!(matches!(game.phase, Phase::CowboyVote { .. }));
        assert_eq!(game.players[idx_2].hand.unwrap().rank, Rank::King);
    }

    #[test]
    fn full_game_two_rounds() {
        // Play a 3-player game through 2 rounds
        let mut game = setup_lobby(3);
        game.act(
            0,
            Action::StartGame {
                lives: 2,
                blocker_tokens: 0,
                exemption_tokens: 0,
            },
        )
        .unwrap();

        // Round 1: rig a normal round
        let idx_0 = game.alive_order[0];
        let idx_1 = game.alive_order[1];
        let idx_2 = game.alive_order[2];

        game.players[idx_0].hand = Some(Card::new(Rank::Queen, Suit::Hearts));
        game.players[idx_1].hand = Some(Card::new(Rank::Jack, Suit::Spades));
        game.players[idx_2].hand = Some(Card::new(Rank::Ace, Suit::Clubs));
        game.is_cowboy_round = false;

        // All pass to dealer
        game.phase = Phase::NormalTurn { current_idx: 0 };
        let id_0 = game.players[idx_0].id;
        let id_1 = game.players[idx_1].id;
        let id_2 = game.players[idx_2].id;

        game.act(id_0, Action::Pass).unwrap();
        // After player 0 passes, it should be player 1's turn or dealer
        if matches!(game.phase, Phase::NormalTurn { .. }) {
            game.act(id_1, Action::Pass).unwrap();
        }
        // Now dealer's turn
        assert_eq!(game.phase, Phase::DealerTurn);

        // Dealer draws, but let's just have them pass
        game.act(id_2, Action::DealerPass).unwrap();
        assert_eq!(game.phase, Phase::RoundEnd);

        // Ace holder lost a life
        assert_eq!(game.players[idx_2].lives, 1);

        // Advance to round 2
        game.act(0, Action::NextRound).unwrap();
        assert_eq!(game.round_number, 2);

        // Should be in a new round
        assert!(
            matches!(game.phase, Phase::NormalTurn { .. })
                || matches!(game.phase, Phase::CowboyVote { .. })
                || matches!(game.phase, Phase::DealerTurn)
        );
    }

    #[test]
    fn resurrection_preserves_tokens_for_survivors() {
        // 4 players: P1 and P2 die early, P3 and P4 tie at the end.
        // P3 still has a blocker token. On resurrection, P3 keeps it, P1/P2 don't get tokens.
        let mut game = setup_lobby(4);
        game.act(
            0,
            Action::StartGame {
                lives: 2,
                blocker_tokens: 1,
                exemption_tokens: 1,
            },
        )
        .unwrap();

        // Eliminate players 1 and 2 manually (simulating prior rounds)
        // Find the actual player indices
        let p0_idx = 0;
        let p1_idx = 1;
        let p2_idx = 2;
        let p3_idx = 3;

        game.players[p0_idx].lives = 0;
        game.players[p0_idx].is_eliminated = true;
        game.players[p0_idx].blocker_tokens = 0;
        game.players[p0_idx].exemption_tokens = 0;

        game.players[p1_idx].lives = 0;
        game.players[p1_idx].is_eliminated = true;
        game.players[p1_idx].blocker_tokens = 0;
        game.players[p1_idx].exemption_tokens = 0;

        // P3 and P4 are alive with 1 life each, P3 has a blocker still
        game.players[p2_idx].lives = 1;
        game.players[p2_idx].blocker_tokens = 1;
        game.players[p2_idx].exemption_tokens = 0;

        game.players[p3_idx].lives = 1;
        game.players[p3_idx].blocker_tokens = 0;
        game.players[p3_idx].exemption_tokens = 1;

        // Set up round with just P3 and P4 alive
        game.dealer_idx = p2_idx;
        game.alive_order = game.compute_alive_order();

        // Rig a tie: both have a 5
        for &idx in &game.alive_order {
            game.players[idx].hand = Some(Card::new(Rank::Five, Suit::Hearts));
        }
        game.is_cowboy_round = false;
        game.phase = Phase::DealerTurn;

        let dealer_id = game.players[*game.alive_order.last().unwrap()].id;
        game.act(dealer_id, Action::DealerPass).unwrap();

        // Both died
        assert_eq!(game.players[p2_idx].lives, 0);
        assert_eq!(game.players[p3_idx].lives, 0);

        game.act(0, Action::NextRound).unwrap();

        // All 4 should be alive again
        for p in &game.players {
            assert!(p.is_alive());
            assert_eq!(p.lives, 1);
        }

        // P3 (idx 2) should keep their blocker token
        assert_eq!(game.players[p2_idx].blocker_tokens, 1);
        assert_eq!(game.players[p2_idx].exemption_tokens, 0);

        // P4 (idx 3) should keep their exemption token
        assert_eq!(game.players[p3_idx].exemption_tokens, 1);
        assert_eq!(game.players[p3_idx].blocker_tokens, 0);

        // P1 and P2 (previously eliminated) come back with 0 tokens
        assert_eq!(game.players[p0_idx].blocker_tokens, 0);
        assert_eq!(game.players[p0_idx].exemption_tokens, 0);
        assert_eq!(game.players[p1_idx].blocker_tokens, 0);
        assert_eq!(game.players[p1_idx].exemption_tokens, 0);
    }

    #[test]
    fn cowboy_vote_default_for_missing_voters() {
        // Test that players who don't vote get auto-defaulted
        let mut game = setup_lobby(3);
        start_default(&mut game);

        let idx_0 = game.alive_order[0];
        let idx_1 = game.alive_order[1];
        let idx_2 = game.alive_order[2];

        game.players[idx_0].hand = Some(Card::new(Rank::King, Suit::Spades));
        game.players[idx_1].hand = Some(Card::new(Rank::Two, Suit::Hearts));
        game.players[idx_2].hand = Some(Card::new(Rank::Five, Suit::Clubs));
        game.is_cowboy_round = true;
        game.phase = Phase::CowboyVote { votes: Vec::new() };
        game.exempted.clear();

        let id_0 = game.players[idx_0].id;

        // Only player 0 votes. Player 1 and 2 don't vote (timed out).
        game.act(id_0, Action::CowboyVote { exempt: false })
            .unwrap();

        // Resolve: player 1 and 2 both have exemption tokens, so they auto-exempt
        game.act(0, Action::ResolveCowboyVote).unwrap();

        assert_eq!(game.phase, Phase::RoundEnd);
        // Player 0 (king) is the only active player -- lone cowboy rule
        assert_eq!(game.players[idx_0].lives, 2);
        // Player 1 and 2 auto-exempted, safe
        assert_eq!(game.players[idx_1].lives, 3);
        assert_eq!(game.players[idx_2].lives, 3);
    }

    #[test]
    fn cowboy_vote_no_events_until_resolve() {
        let mut game = setup_lobby(3);
        start_default(&mut game);

        let idx_0 = game.alive_order[0];
        let idx_1 = game.alive_order[1];
        let idx_2 = game.alive_order[2];

        game.players[idx_0].hand = Some(Card::new(Rank::King, Suit::Spades));
        game.players[idx_1].hand = Some(Card::new(Rank::Two, Suit::Hearts));
        game.players[idx_2].hand = Some(Card::new(Rank::Five, Suit::Clubs));
        game.is_cowboy_round = true;
        game.phase = Phase::CowboyVote { votes: Vec::new() };
        game.exempted.clear();

        let id_0 = game.players[idx_0].id;
        let id_1 = game.players[idx_1].id;

        game.events.clear();
        game.act(id_0, Action::CowboyVote { exempt: false })
            .unwrap();
        // No events should have been emitted from staging
        assert!(game.events.is_empty());

        game.act(id_1, Action::CowboyVote { exempt: true }).unwrap();
        assert!(game.events.is_empty());

        // Only on resolve do events appear
        game.act(0, Action::ResolveCowboyVote).unwrap();
        assert!(!game.events.is_empty());
    }

    #[test]
    fn dealer_chip_advances_after_round() {
        let mut game = setup_lobby(3);
        start_default(&mut game);

        let initial_dealer = game.dealer_idx;

        // Rig a quick normal round
        let idx_0 = game.alive_order[0];
        let idx_1 = game.alive_order[1];
        let idx_2 = game.alive_order[2];

        game.players[idx_0].hand = Some(Card::new(Rank::Queen, Suit::Hearts));
        game.players[idx_1].hand = Some(Card::new(Rank::Jack, Suit::Spades));
        game.players[idx_2].hand = Some(Card::new(Rank::Ace, Suit::Clubs));
        game.is_cowboy_round = false;
        game.phase = Phase::DealerTurn;

        let dealer_id = game.players[idx_2].id;
        game.act(dealer_id, Action::DealerPass).unwrap();
        game.act(0, Action::NextRound).unwrap();

        // Dealer should have advanced
        assert_ne!(game.dealer_idx, initial_dealer);
    }
}
