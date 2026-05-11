import type { Card } from "../types/game";

interface CardDisplayProps {
  card: Card | null;
  faceDown?: boolean;
  size?: "small" | "medium" | "large";
  highlight?: boolean;
}

const SUIT_SYMBOLS: Record<string, string> = {
  Hearts: "\u2665",
  Diamonds: "\u2666",
  Clubs: "\u2663",
  Spades: "\u2660",
};

const RANK_LABELS: Record<string, string> = {
  Ace: "A",
  Two: "2",
  Three: "3",
  Four: "4",
  Five: "5",
  Six: "6",
  Seven: "7",
  Eight: "8",
  Nine: "9",
  Ten: "10",
  Jack: "J",
  Queen: "Q",
  King: "K",
};

function isRed(suit: string): boolean {
  return suit === "Hearts" || suit === "Diamonds";
}

export function CardDisplay({
  card,
  faceDown = false,
  size = "medium",
  highlight = false,
}: CardDisplayProps) {
  if (!card || faceDown) {
    return <div className={`card card-back card-${size}`} />;
  }

  const suit = SUIT_SYMBOLS[card.suit] || "?";
  const rank = RANK_LABELS[card.rank] || "?";
  const color = isRed(card.suit) ? "red" : "black";

  return (
    <div
      className={`card card-face card-${size} card-${color} ${highlight ? "card-highlight" : ""}`}
    >
      <span className="card-rank">{rank}</span>
      <span className="card-suit">{suit}</span>
    </div>
  );
}
