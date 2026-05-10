use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Suit {
    Hearts,
    Diamonds,
    Clubs,
    Spades,
}

impl fmt::Display for Suit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Suit::Hearts => write!(f, "♥"),
            Suit::Diamonds => write!(f, "♦"),
            Suit::Clubs => write!(f, "♣"),
            Suit::Spades => write!(f, "♠"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Rank {
    Ace,
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Eight,
    Nine,
    Ten,
    Jack,
    Queen,
    King,
}

impl Rank {
    /// Ranking value in a Normal round.
    /// Queen is highest (12), Ace is lowest (0).
    /// King has no normal ranking -- it triggers Cowboy.
    pub fn normal_value(self) -> Option<u8> {
        match self {
            Rank::Ace => Some(0),
            Rank::Two => Some(1),
            Rank::Three => Some(2),
            Rank::Four => Some(3),
            Rank::Five => Some(4),
            Rank::Six => Some(5),
            Rank::Seven => Some(6),
            Rank::Eight => Some(7),
            Rank::Nine => Some(8),
            Rank::Ten => Some(9),
            Rank::Jack => Some(10),
            Rank::Queen => Some(11),
            Rank::King => None, // Kings don't exist in normal ranking
        }
    }

    /// Ranking value in a Cowboy round.
    /// Ace is highest (12), 2 is lowest (0).
    pub fn cowboy_value(self) -> u8 {
        match self {
            Rank::Two => 0,
            Rank::Three => 1,
            Rank::Four => 2,
            Rank::Five => 3,
            Rank::Six => 4,
            Rank::Seven => 5,
            Rank::Eight => 6,
            Rank::Nine => 7,
            Rank::Ten => 8,
            Rank::Jack => 9,
            Rank::Queen => 10,
            Rank::King => 11,
            Rank::Ace => 12,
        }
    }
}

impl fmt::Display for Rank {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Rank::Ace => write!(f, "A"),
            Rank::Two => write!(f, "2"),
            Rank::Three => write!(f, "3"),
            Rank::Four => write!(f, "4"),
            Rank::Five => write!(f, "5"),
            Rank::Six => write!(f, "6"),
            Rank::Seven => write!(f, "7"),
            Rank::Eight => write!(f, "8"),
            Rank::Nine => write!(f, "9"),
            Rank::Ten => write!(f, "10"),
            Rank::Jack => write!(f, "J"),
            Rank::Queen => write!(f, "Q"),
            Rank::King => write!(f, "K"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Card {
    pub rank: Rank,
    pub suit: Suit,
}

impl Card {
    pub fn new(rank: Rank, suit: Suit) -> Self {
        Self { rank, suit }
    }

    pub fn is_king(self) -> bool {
        self.rank == Rank::King
    }
}

impl fmt::Display for Card {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.rank, self.suit)
    }
}

pub struct Deck {
    cards: Vec<Card>,
}

impl Deck {
    pub fn new() -> Self {
        let suits = [Suit::Hearts, Suit::Diamonds, Suit::Clubs, Suit::Spades];
        let ranks = [
            Rank::Ace,
            Rank::Two,
            Rank::Three,
            Rank::Four,
            Rank::Five,
            Rank::Six,
            Rank::Seven,
            Rank::Eight,
            Rank::Nine,
            Rank::Ten,
            Rank::Jack,
            Rank::Queen,
            Rank::King,
        ];

        let mut cards = Vec::with_capacity(52);
        for &suit in &suits {
            for &rank in &ranks {
                cards.push(Card::new(rank, suit));
            }
        }

        Self { cards }
    }

    pub fn shuffle(&mut self) {
        let mut rng = rand::rng();
        self.cards.shuffle(&mut rng);
    }

    pub fn deal_one(&mut self) -> Option<Card> {
        self.cards.pop()
    }

    pub fn remaining(&self) -> usize {
        self.cards.len()
    }

    /// Push a card onto the top of the deck (for testing).
    pub fn push(&mut self, card: Card) {
        self.cards.push(card);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deck_has_52_cards() {
        let deck = Deck::new();
        assert_eq!(deck.remaining(), 52);
    }

    #[test]
    fn deck_has_4_kings() {
        let deck = Deck::new();
        let kings = deck.cards.iter().filter(|c| c.is_king()).count();
        assert_eq!(kings, 4);
    }

    #[test]
    fn deal_one_reduces_count() {
        let mut deck = Deck::new();
        deck.deal_one();
        assert_eq!(deck.remaining(), 51);
    }

    #[test]
    fn deal_all_empties_deck() {
        let mut deck = Deck::new();
        for _ in 0..52 {
            assert!(deck.deal_one().is_some());
        }
        assert!(deck.deal_one().is_none());
        assert_eq!(deck.remaining(), 0);
    }

    #[test]
    fn normal_ranking_ace_lowest_queen_highest() {
        assert_eq!(Rank::Ace.normal_value(), Some(0));
        assert_eq!(Rank::Queen.normal_value(), Some(11));
        assert_eq!(Rank::King.normal_value(), None);
        assert!(Rank::Ace.normal_value().unwrap() < Rank::Two.normal_value().unwrap());
        assert!(Rank::Jack.normal_value().unwrap() < Rank::Queen.normal_value().unwrap());
    }

    #[test]
    fn cowboy_ranking_two_lowest_ace_highest() {
        assert_eq!(Rank::Two.cowboy_value(), 0);
        assert_eq!(Rank::Ace.cowboy_value(), 12);
        assert!(Rank::Two.cowboy_value() < Rank::Three.cowboy_value());
        assert!(Rank::King.cowboy_value() < Rank::Ace.cowboy_value());
    }

    #[test]
    fn king_detection() {
        let king = Card::new(Rank::King, Suit::Spades);
        let ace = Card::new(Rank::Ace, Suit::Hearts);
        assert!(king.is_king());
        assert!(!ace.is_king());
    }

    #[test]
    fn card_display() {
        let card = Card::new(Rank::Ace, Suit::Spades);
        assert_eq!(format!("{card}"), "A♠");

        let card = Card::new(Rank::Ten, Suit::Hearts);
        assert_eq!(format!("{card}"), "10♥");
    }

    #[test]
    fn shuffle_changes_order() {
        let deck1 = Deck::new();
        let mut deck2 = Deck::new();
        deck2.shuffle();
        // Extremely unlikely to remain identical after shuffle
        assert_ne!(deck1.cards, deck2.cards);
    }
}
