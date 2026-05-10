Cowboy Game -- Rule Clarifications & Design Decisions
======================================================

Collected during game engine development. These supplement game_rules.md
with answers to ambiguities and edge cases discovered during implementation.

## Suits don't matter

Card suit is never relevant to game outcome. Ties are determined purely by
rank. Two players holding a 5 of Hearts and a 5 of Spades are tied.

## King holder can exempt from their own cowboy round

A player dealt a King triggers the cowboy round, but they are allowed to
exempt from it. This is strategic: if they suspect everyone else will also
exempt, they'd be the lone active player and lose a life despite holding
the second-highest card. Exempting protects them from that scenario.

## Cowboy vote is simultaneous

In the physical game, players hold their hands up and drop tokens on
"1...2...3...Drop!" -- no one can react to others' choices. The game
engine enforces this by staging votes silently (no events emitted, no
tokens consumed) and only resolving them all at once when the server
calls ResolveCowboyVote (triggered by a timer). No information leaks
during the voting window.

## Vote timeout defaults

If a player doesn't vote before the timer expires, they are auto-defaulted:
- If they have an exemption token: they exempt.
- If they don't: they stay in.

## Resurrection rules

Resurrection triggers ONLY when all remaining players die simultaneously
from a tied outcome. If even one player survives the round, no resurrection.

Example that does NOT trigger resurrection:
- 3 players left with 1, 1, and 3 lives. All tie with a 5.
- All lose a life. Two die, one survives with 2 lives.
- Survivor wins. No resurrection.

Example that DOES trigger resurrection:
- 2 players left with 1 life each. Both hold a 5. Both die.
- Zero survivors triggers resurrection.

On resurrection:
- ALL players who started the game come back with 1 life.
- Players who were alive in the final round (the ones who tied) KEEP
  their remaining blocker and exemption tokens. This rewards them for
  surviving as long as they did.
- Previously eliminated players come back with 0 tokens.
- Resurrection can chain: if resurrected players tie again, it triggers
  another resurrection.

## Two-player trading

With 2 players, the non-dealer can trade with the dealer (they are the
person to the left). Then it becomes the dealer's turn and they decide
to pass or draw off the top. The dealer can also block the trade if they
have a blocker token. This is a high-tension moment in the game.

## Kings can never appear in trades

Kings trigger a cowboy round immediately during the deal. If no king is
dealt, there are no kings in play during the normal trading phase. The
only way a king appears mid-round is the dealer drawing one off the top,
at which point all trades are already locked.

## Server enforces king revelation

In the digital version, hiding a king is impossible -- the server detects
kings automatically during the deal and triggers the cowboy round. No
player action required.

## Multiple kings on deal

Multiple players can be dealt kings simultaneously. All are revealed, and
the cowboy round proceeds. Non-king holders may be stuck with terrible
cards (e.g., a 2, which is the lowest in cowboy mode) and have no choice
but to stay in if they've already used their exemption token.
