use serde::{Deserialize, Serialize};

use crate::card::Card;

pub type PlayerId = u64;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Player {
    pub id: PlayerId,
    pub name: String,
    pub lives: u8,
    pub exemption_tokens: u8,
    pub blocker_tokens: u8,
    pub hand: Option<Card>,
    pub is_eliminated: bool,
}

impl Player {
    pub fn new(
        id: PlayerId,
        name: String,
        lives: u8,
        exemption_tokens: u8,
        blocker_tokens: u8,
    ) -> Self {
        Self {
            id,
            name,
            lives,
            exemption_tokens,
            blocker_tokens,
            hand: None,
            is_eliminated: false,
        }
    }

    pub fn is_alive(&self) -> bool {
        !self.is_eliminated && self.lives > 0
    }

    pub fn lose_life(&mut self) {
        if self.lives > 0 {
            self.lives -= 1;
        }
        if self.lives == 0 {
            self.is_eliminated = true;
        }
    }

    pub fn can_block(&self) -> bool {
        self.blocker_tokens > 0
    }

    pub fn use_blocker(&mut self) {
        assert!(self.blocker_tokens > 0, "No blocker tokens remaining");
        self.blocker_tokens -= 1;
    }

    pub fn can_exempt(&self) -> bool {
        self.exemption_tokens > 0
    }

    pub fn use_exemption(&mut self) {
        assert!(self.exemption_tokens > 0, "No exemption tokens remaining");
        self.exemption_tokens -= 1;
    }

    /// Resurrect a player with 1 life and no tokens.
    pub fn resurrect(&mut self) {
        self.lives = 1;
        self.is_eliminated = false;
        // Tokens are NOT returned on resurrection
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_player() -> Player {
        Player::new(1, "Test".to_string(), 3, 1, 1)
    }

    #[test]
    fn new_player_is_alive() {
        let p = make_player();
        assert!(p.is_alive());
        assert_eq!(p.lives, 3);
    }

    #[test]
    fn lose_life_decrements() {
        let mut p = make_player();
        p.lose_life();
        assert_eq!(p.lives, 2);
        assert!(p.is_alive());
    }

    #[test]
    fn lose_all_lives_eliminates() {
        let mut p = make_player();
        p.lose_life();
        p.lose_life();
        p.lose_life();
        assert_eq!(p.lives, 0);
        assert!(p.is_eliminated);
        assert!(!p.is_alive());
    }

    #[test]
    fn use_blocker_decrements() {
        let mut p = make_player();
        assert!(p.can_block());
        p.use_blocker();
        assert!(!p.can_block());
    }

    #[test]
    #[should_panic(expected = "No blocker tokens")]
    fn use_blocker_when_empty_panics() {
        let mut p = make_player();
        p.use_blocker();
        p.use_blocker(); // should panic
    }

    #[test]
    fn use_exemption_decrements() {
        let mut p = make_player();
        assert!(p.can_exempt());
        p.use_exemption();
        assert!(!p.can_exempt());
    }

    #[test]
    fn resurrect_gives_one_life_no_tokens() {
        let mut p = make_player();
        p.use_blocker();
        p.use_exemption();
        p.lose_life();
        p.lose_life();
        p.lose_life();
        assert!(p.is_eliminated);

        p.resurrect();
        assert!(p.is_alive());
        assert_eq!(p.lives, 1);
        assert_eq!(p.exemption_tokens, 0);
        assert_eq!(p.blocker_tokens, 0);
    }

    #[test]
    fn no_hand_by_default() {
        let p = make_player();
        assert!(p.hand.is_none());
    }
}
