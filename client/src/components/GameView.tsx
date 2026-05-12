import { useEffect, useRef, useState } from "react";
import type {
  Card,
  ClientMessage,
  GameEvent,
  PlayerGameState,
  PlayerPublicState,
  PlayerId,
} from "../types/game";
import { CardDisplay } from "./CardDisplay";
import { MediaView } from "./MediaView";
import { MediaToggles } from "./MediaToggles";
import { Countdown } from "./Countdown";
import { DesertBg } from "./DesertBg";
import type { LiveKitState } from "../hooks/useLiveKit";

interface GameViewProps {
  gameState: PlayerGameState;
  playerId: PlayerId;
  events: GameEvent[];
  send: (msg: ClientMessage) => void;
  clearEvents: () => void;
  isHost: boolean;
  livekit: LiveKitState;
}

function sendAction(send: (msg: ClientMessage) => void, action: object) {
  send({ type: "GameAction", action } as ClientMessage);
}

// --- Player Profile Card (reused across scenes) ---

function PlayerProfile({
  player,
  isYou,
  size = "medium",
  glow,
  label,
  videoTrack,
  audioTrack,
  children,
}: {
  player: PlayerPublicState;
  isYou: boolean;
  size?: "small" | "medium" | "large";
  glow?: "gold" | "red" | "green" | "accent";
  label?: string;
  videoTrack?: MediaStreamTrack | null;
  audioTrack?: MediaStreamTrack | null;
  children?: React.ReactNode;
}) {
  const hasVideo = !!videoTrack;

  return (
    <div className={`profile profile-${size} ${glow ? `profile-glow-${glow}` : ""} ${player.is_eliminated ? "profile-dead" : ""}`}>
      <div className={`profile-avatar ${hasVideo ? "profile-avatar-video" : ""}`}>
        {hasVideo ? (
          <MediaView
            videoTrack={videoTrack ?? null}
            audioTrack={null}
            muted
            className="profile-video-feed"
          />
        ) : (
          <span className="profile-initial">{player.name[0]}</span>
        )}
        {/* Play remote audio even without video */}
      </div>
      <div className="profile-info">
        <div className="profile-name">
          {player.name}
          {isYou && <span className="profile-you">you</span>}
        </div>
        <div className="profile-stats">
          <span className="stat-lives" title="Lives">
            {"♥".repeat(player.lives)}
            {player.lives === 0 && <span className="stat-dead">OUT</span>}
          </span>
          <span className="stat-tokens">
            {player.has_blocker && <span className="token blocker" title="Has blocker">🛡</span>}
            {player.has_exemption && <span className="token exemption" title="Has exemption">⭐</span>}
          </span>
        </div>
        {label && <div className="profile-label">{label}</div>}
      </div>
      {player.revealed_card && (
        <div className="profile-card">
          <CardDisplay card={player.revealed_card} size="small" />
        </div>
      )}
      {children}
    </div>
  );
}

// --- Battle Profile (compact horizontal, big face for banter) ---

function BattleProfile({
  player,
  getMedia,
  isYou = false,
  label,
}: {
  player: PlayerPublicState;
  getMedia: (id: PlayerId) => { audioTrack: MediaStreamTrack | null; videoTrack: MediaStreamTrack | null };
  isYou?: boolean;
  label?: string;
}) {
  const media = getMedia(player.id);
  const hasVideo = !!media.videoTrack;

  return (
    <div className={`bp ${isYou ? "bp-you" : "bp-opponent"}`}>
      <div className={`bp-avatar ${hasVideo ? "bp-avatar-video" : ""}`}>
        {hasVideo ? (
          <MediaView
            videoTrack={media.videoTrack}
            audioTrack={null}
            muted
            className="bp-video"
          />
        ) : (
          <span className="bp-initial">{player.name[0]}</span>
        )}
      </div>
      <div className="bp-info">
        <div className="bp-name">{player.name}{isYou && <span className="bp-you-tag"> (you)</span>}</div>
        <div className="bp-stats">
          <span className="bp-lives">{"♥".repeat(player.lives)}</span>
          {player.has_blocker && <span>🛡</span>}
          {player.has_exemption && <span>⭐</span>}
        </div>
        {label && <div className="bp-label">{label}</div>}
      </div>
    </div>
  );
}

// --- Showdown Result Tracking ---

interface ShowdownInfo {
  reveals: [PlayerId, Card][];
  losers: PlayerId[];
  isCowboy: boolean;
}

export function GameView({
  gameState,
  playerId,
  events,
  send,
  clearEvents,
  isHost,
  livekit,
}: GameViewProps) {
  const [announcement, setAnnouncement] = useState<string | null>(null);
  const [announcementSub, setAnnouncementSub] = useState<string | null>(null);
  const [showdownInfo, setShowdownInfo] = useState<ShowdownInfo | null>(null);
  const [cowboyVoted, setCowboyVoted] = useState(false);
  const [deckReveal, setDeckReveal] = useState<{ card: Card; dealerName: string } | null>(null);
  const [traderId, setTraderId] = useState<PlayerId | null>(null);
  const announcementTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  const myPlayer = gameState.players.find((p) => p.id === playerId);
  const phase = gameState.phase;
  const isMyTurn = gameState.current_actor === playerId;

  const getPlayer = (id: PlayerId) =>
    gameState.players.find((p) => p.id === id);
  const getPlayerName = (id: PlayerId) => getPlayer(id)?.name || "Unknown";

  // Get media tracks for a player
  const getMedia = (id: PlayerId) => {
    if (id === playerId) {
      return livekit.localMedia;
    }
    return livekit.participants.get(id) ?? { audioTrack: null, videoTrack: null };
  };

  // Wrapper that auto-injects media tracks into PlayerProfile
  function P(props: Omit<Parameters<typeof PlayerProfile>[0], "videoTrack" | "audioTrack">) {
    const media = getMedia(props.player.id);
    return <PlayerProfile {...props} videoTrack={media.videoTrack} audioTrack={media.audioTrack} />;
  }

  // Reset vote state when phase changes away from cowboy_vote
  useEffect(() => {
    if (phase !== "cowboy_vote") {
      setCowboyVoted(false);
    }
    if (phase !== "waiting_for_block") {
      setTraderId(null);
    }
    // Clear stale announcements when a new round starts or new game begins
    if (phase === "normal_turn" || phase === "dealer_turn" || phase === "cowboy_vote") {
      setAnnouncement(null);
      setAnnouncementSub(null);
    }
  }, [phase, gameState.round_number]);

  // Process events for announcements and showdown info
  useEffect(() => {
    for (const event of events) {
      switch (event.type) {
        case "CowboyTriggered": {
          const names = event.king_holders.map(getPlayerName).join(", ");
          showAnnouncement("🤠 COWBOY!", `${names} drew a King`, 2500);
          break;
        }
        case "ResurrectionTriggered":
          showAnnouncement("💀 RESURRECTION!", "All players return with 1 life", 3000);
          break;
        case "GameWon": {
          // Only show if we're actually in game_over phase (not a stale event from last game)
          if (gameState.phase === "game_over") {
            const winner = getPlayer(event.winner_id);
            showAnnouncement(`🏆 ${winner?.name || "Someone"} wins!`, null, 0);
          }
          break;
        }
        case "ShowdownResult":
          setShowdownInfo({
            reveals: event.reveals,
            losers: event.losers,
            isCowboy: event.is_cowboy_round,
          });
          break;
        case "EveryoneExempted":
          showAnnouncement("Everyone exempted!", "No one loses a life", 2500);
          break;
        case "TradeProposed":
          setTraderId(event.from_id);
          break;
        case "TradeBlocked": {
          const blocker = getPlayer(event.blocker_id);
          showAnnouncement("BLOCKED!", `${blocker?.name} used their blocker`, 1500);
          break;
        }
        case "DealerDrewCard": {
          const dealer = getPlayer(event.dealer_id);
          setDeckReveal({ card: event.card, dealerName: dealer?.name || "Dealer" });
          setTimeout(() => setDeckReveal(null), 2500);
          break;
        }
      }
    }
    if (events.length > 0) {
      clearEvents();
    }
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [events]);

  // Clear showdown info when we leave round_end
  useEffect(() => {
    if (phase !== "round_end" && phase !== "showdown") {
      setShowdownInfo(null);
    }
  }, [phase]);

  function showAnnouncement(text: string, sub: string | null, duration: number) {
    if (announcementTimer.current) clearTimeout(announcementTimer.current);
    setAnnouncement(text);
    setAnnouncementSub(sub);
    if (duration > 0) {
      announcementTimer.current = setTimeout(() => {
        setAnnouncement(null);
        setAnnouncementSub(null);
      }, duration);
    }
  }

  // Determine alive count label
  const aliveCount = gameState.players.filter((p) => !p.is_eliminated).length;
  const totalPlayers = gameState.players.length;
  const aliveLabel =
    aliveCount === 2
      ? "Final 2!"
      : aliveCount === 3
        ? "Final 3!"
        : `${aliveCount}/${totalPlayers}`;

  return (
    <div className={`screen game-screen game-phase-${phase}`}>
      <DesertBg />

      {/* Announcement overlay */}
      {announcement && (
        <div className="announcement-overlay" onClick={() => { setAnnouncement(null); setAnnouncementSub(null); }}>
          <div className="announcement-content">
            <div className="announcement-text">{announcement}</div>
            {announcementSub && <div className="announcement-sub">{announcementSub}</div>}
          </div>
        </div>
      )}

      {/* Deck reveal overlay */}
      {deckReveal && (
        <div className="deck-reveal-overlay">
          <div className="deck-reveal-content">
            <div className="deck-reveal-label">{deckReveal.dealerName} takes off the top...</div>
            <div className="deck-reveal-stage">
              <div className="deck-reveal-deck">
                <CardDisplay card={null} faceDown size="large" />
              </div>
              <div className="deck-reveal-card">
                <CardDisplay card={deckReveal.card} size="large" highlight />
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Top bar */}
      <div className="round-bar">
        <span className="round-number">Round {gameState.round_number}</span>
        {gameState.is_cowboy_round
          ? <span className="cowboy-badge">🤠 COWBOY</span>
          : <span className="alive-count">{aliveLabel}</span>
        }
        <div className="round-bar-controls">
          <MediaToggles livekit={livekit} layout="compact" />
          {isHost && (
            <button
              className="btn-end-game"
              onClick={() => {
                if (window.confirm("End the game and return to lobby?")) {
                  send({ type: "EndGame" });
                }
              }}
            >
              End
            </button>
          )}
        </div>
      </div>

      {/* Player strip - always visible at top */}
      <div className="player-strip">
        {gameState.players.map((p) => {
          const isLoser = showdownInfo?.losers.includes(p.id);
          return (
            <div
              key={p.id}
              className={[
                "player-chip",
                p.is_eliminated && "eliminated",
                p.id === gameState.current_actor && "active",
                p.id === playerId && "you",
                p.id === gameState.dealer_id && "dealer",
                isLoser && "loser",
              ]
                .filter(Boolean)
                .join(" ")}
            >
              <span className="chip-name">{p.name}</span>
              <span className="chip-lives">
                {"♥".repeat(p.lives)}
                {p.lives === 0 && "✕"}
              </span>
              <span className="chip-tokens">
                {p.has_blocker && <span className="token blocker">🛡</span>}
                {p.has_exemption && <span className="token exemption">⭐</span>}
              </span>
            </div>
          );
        })}
      </div>

      {/* Always play all remote audio */}
      {Array.from(livekit.participants.entries()).map(([pid, media]) => (
        media.audioTrack ? <MediaView key={`audio-${pid}`} videoTrack={null} audioTrack={media.audioTrack} /> : null
      ))}

      {/* Main scene area */}
      <div className="scene">
        {renderScene()}
      </div>
    </div>
  );

  function renderScene() {
    // --- TRADING SCENE (normal_turn, waiting_for_block) ---
    if (phase === "normal_turn" || phase === "waiting_for_block") {
      return <TradingScene />;
    }

    // --- DEALER SCENE ---
    if (phase === "dealer_turn") {
      return <DealerScene />;
    }

    // --- COWBOY VOTE SCENE ---
    if (phase === "cowboy_vote") {
      return <CowboyVoteScene />;
    }

    // --- ROUND END / SHOWDOWN SCENE ---
    if (phase === "round_end" || phase === "showdown") {
      return <RoundEndScene />;
    }

    // --- GAME OVER ---
    if (phase === "game_over") {
      return <GameOverScene />;
    }

    return (
      <div className="scene-waiting">
        <div className="scene-waiting-text">Waiting...</div>
      </div>
    );
  }

  // ==========================================
  // TRADING SCENE
  // ==========================================
  function TradingScene() {
    const currentActor = gameState.current_actor;
    if (!currentActor || !myPlayer) return null;

    const actorPlayer = getPlayer(currentActor);
    if (!actorPlayer) return null;

    // The trade target is the person to the actor's left (next in alive order)
    // For the blocker: the target is the person being asked
    const isTradeProposed = phase === "waiting_for_block";

    // If it's MY turn to trade or pass
    if (phase === "normal_turn" && isMyTurn) {
      const aliveNonEliminated = gameState.players.filter((p) => !p.is_eliminated);
      const myIdx = aliveNonEliminated.findIndex((p) => p.id === playerId);
      const targetIdx = (myIdx + 1) % aliveNonEliminated.length;
      const target = aliveNonEliminated[targetIdx];

      return (
        <div className="battle">
          <div className="battle-banner-prompt">
            <span>Want to trade with {target.name}?</span>
            <Countdown seconds={30} resetKey={`turn-${gameState.round_number}-${playerId}`} />
          </div>

          <div className="battle-opponent">
            <BattleProfile player={target} getMedia={getMedia} />
            <CardDisplay card={null} faceDown size="small" />
          </div>

          <div className="battle-you">
            <CardDisplay card={gameState.your_card} size="medium" highlight />
            <BattleProfile player={myPlayer} getMedia={getMedia} isYou />
          </div>

          <div className="scene-actions battle-actions">
            <div className="battle-actions-row">
              <button
                className="btn btn-secondary btn-battle"
                onClick={() => sendAction(send, { Pass: null })}
              >
                Keep!
              </button>
              <button
                className="btn btn-accent btn-battle"
                onClick={() => sendAction(send, { Trade: null })}
              >
                Trade
              </button>
            </div>
          </div>
        </div>
      );
    }

    if (isTradeProposed && isMyTurn) {
      const traderPlayer = traderId ? getPlayer(traderId) : null;
      if (!traderPlayer) return null;

      return (
        <div className="battle">
          <div className="battle-banner-alert">
            <span>Trade Incoming!</span>
            <Countdown seconds={30} resetKey={`block-${gameState.round_number}-${playerId}`} />
          </div>

          <div className="battle-opponent">
            <BattleProfile player={traderPlayer} getMedia={getMedia} label="wants your card" />
            <CardDisplay card={null} faceDown size="small" />
          </div>

          <div className="battle-you">
            <CardDisplay card={gameState.your_card} size="medium" highlight />
            <BattleProfile player={myPlayer} getMedia={getMedia} isYou />
          </div>

          <div className="scene-actions battle-actions">
            <div className="battle-actions-row">
              <button
                className="btn btn-danger btn-battle"
                onClick={() => sendAction(send, { AcceptTrade: null })}
              >
                Accept
              </button>
              {myPlayer.has_blocker && (
                <button
                  className="btn btn-warning btn-battle"
                  onClick={() => sendAction(send, { Block: null })}
                >
                  🛡 Block
                </button>
              )}
            </div>
          </div>
        </div>
      );
    }

    // I proposed the trade, watching them decide to block or accept
    if (isTradeProposed && !isMyTurn) {
      const targetPlayer = getPlayer(gameState.current_actor!);
      if (targetPlayer) {
        return (
          <div className="battle">
            <div className="battle-staredown">
              <BattleProfile player={targetPlayer} getMedia={getMedia} label="deciding..." />
            </div>

            <div className="battle-waiting-text">Will they block?</div>

            <div className="battle-you">
              <CardDisplay card={gameState.your_card} size="small" />
              <BattleProfile player={myPlayer} getMedia={getMedia} isYou />
            </div>
          </div>
        );
      }
    }

    // Watching someone else's turn (bystander)
    const watchTarget = gameState.players.filter((p) => !p.is_eliminated);
    const actorIdx = watchTarget.findIndex((p) => p.id === currentActor);
    const nextIdx = (actorIdx + 1) % watchTarget.length;
    const nextPlayer = watchTarget[nextIdx];

    const bannerText = isTradeProposed
      ? `${actorPlayer.name} wants to trade with ${nextPlayer.name}`
      : `${actorPlayer.name} is deciding...`;

    return (
      <div className="battle">
        <div className="battle-banner-prompt">
          <span>{bannerText}</span>
        </div>

        <div className="battle-watch">
          <BattleProfile player={actorPlayer} getMedia={getMedia} />
          <div className="battle-watch-vs">vs</div>
          <BattleProfile player={nextPlayer} getMedia={getMedia} />
        </div>

        <div className="battle-you">
          <CardDisplay card={gameState.your_card} size="small" />
          <BattleProfile player={myPlayer!} getMedia={getMedia} isYou />
        </div>
      </div>
    );
  }

  // ==========================================
  // DEALER SCENE
  // ==========================================
  function DealerScene() {
    if (!myPlayer) return null;

    const dealerPlayer = getPlayer(gameState.dealer_id);
    if (!dealerPlayer) return null;

    if (isMyTurn) {
      return (
        <div className="battle">
          <div className="battle-banner-prompt">
            <span>Keep or take off the top?</span>
            <Countdown seconds={30} resetKey={`dealer-${gameState.round_number}`} />
          </div>

          <div className="battle-deck-area">
            <div className="deck-stack">
              <CardDisplay card={null} faceDown size="medium" />
            </div>
            <div className="deck-label">The Deck</div>
          </div>

          <div className="battle-you">
            <CardDisplay card={gameState.your_card} size="medium" highlight />
            <BattleProfile player={myPlayer} getMedia={getMedia} isYou label="Dealer" />
          </div>

          <div className="scene-actions battle-actions">
            <div className="battle-actions-row">
              <button
                className="btn btn-secondary btn-battle"
                onClick={() => sendAction(send, { DealerPass: null })}
              >
                Keep Card
              </button>
              <button
                className="btn btn-accent btn-battle"
                onClick={() => sendAction(send, { TakeOffTop: null })}
              >
                Off the Top
              </button>
            </div>
          </div>
        </div>
      );
    }

    // Watching the dealer
    return (
      <div className="battle">
        <div className="battle-banner-prompt">
          <span>Will {dealerPlayer.name} take off the top?</span>
        </div>

        <div className="battle-opponent">
          <BattleProfile player={dealerPlayer} getMedia={getMedia} label="Dealer" />
          <div className="deck-stack" style={{ alignSelf: "center" }}>
            <CardDisplay card={null} faceDown size="medium" />
          </div>
        </div>

        <div className="battle-you">
          <CardDisplay card={gameState.your_card} size="small" />
          <BattleProfile player={myPlayer!} getMedia={getMedia} isYou />
        </div>
      </div>
    );
  }

  // ==========================================
  // COWBOY VOTE SCENE
  // ==========================================
  function CowboyVoteScene() {
    if (!myPlayer) return null;

    const canExempt = gameState.your_exemption_tokens > 0;
    const isEliminated = myPlayer.is_eliminated;

    return (
      <div className="scene-cowboy">
        <div className="cowboy-header">
          <div className="scene-label scene-label-cowboy">🤠 Cowboy Round</div>
          {!cowboyVoted && !isEliminated && (
            <Countdown seconds={30} resetKey={`cowboy-${gameState.round_number}`} />
          )}
        </div>
        <div className="cowboy-prompt">
          {gameState.is_cowboy_round && (
            <p className="cowboy-explain">
              Ace is HIGH, 2 is LOW.
              {canExempt ? " Use your exemption to fold and stay safe." : ""}
            </p>
          )}
        </div>

        <div className="cowboy-your-card">
          <div className="your-card-label">Your Card</div>
          <CardDisplay card={gameState.your_card} size="large" highlight={!cowboyVoted} />
        </div>

        {!isEliminated && !cowboyVoted && (
          <div className="scene-actions">
            <button
              className="btn btn-accent btn-large"
              onClick={() => {
                sendAction(send, { CowboyVote: { exempt: false } });
                setCowboyVoted(true);
              }}
            >
              Stay In
            </button>
            {canExempt && (
              <button
                className="btn btn-warning btn-large"
                onClick={() => {
                  sendAction(send, { CowboyVote: { exempt: true } });
                  setCowboyVoted(true);
                }}
              >
                ⭐ Exempt (fold)
              </button>
            )}
          </div>
        )}

        {cowboyVoted && (
          <div className="cowboy-voted">
            <div className="voted-text">Vote submitted</div>
            <div className="voted-sub">Waiting for other players...</div>
          </div>
        )}

        {isEliminated && (
          <div className="cowboy-voted">
            <div className="voted-text">You're out</div>
            <div className="voted-sub">Watching the showdown...</div>
          </div>
        )}
      </div>
    );
  }

  // ==========================================
  // ROUND END SCENE
  // ==========================================
  function RoundEndScene() {
    const everyoneExempted = !showdownInfo && gameState.is_cowboy_round;

    // Figure out what happened to YOU this round
    const youLost = showdownInfo?.losers.includes(playerId) ?? false;
    const youExempted = showdownInfo
      ? !showdownInfo.reveals.some(([id]) => id === playerId) && !myPlayer?.is_eliminated
      : false;
    const youSafe = showdownInfo && !youLost && !youExempted;
    const youEliminated = myPlayer?.is_eliminated && youLost;

    return (
      <div className="scene-roundend">
        <div className="scene-label">Round {gameState.round_number} Results</div>

        {/* Personal outcome banner */}
        {showdownInfo && !everyoneExempted && (
          <div className={`your-outcome ${youLost ? "your-outcome-lost" : "your-outcome-safe"}`}>
            {youEliminated && (
              <>
                <span className="your-outcome-icon">💀</span>
                <span className="your-outcome-text">You're out!</span>
              </>
            )}
            {youLost && !youEliminated && (
              <>
                <span className="your-outcome-icon">💔</span>
                <span className="your-outcome-text">You lost a life!</span>
              </>
            )}
            {youExempted && (
              <>
                <span className="your-outcome-icon">⭐</span>
                <span className="your-outcome-text">You exempted — safe!</span>
              </>
            )}
            {youSafe && (
              <>
                <span className="your-outcome-icon">😎</span>
                <span className="your-outcome-text">You survived!</span>
              </>
            )}
          </div>
        )}

        {everyoneExempted && (
          <div className="everyone-safe">
            <div className="safe-icon">🤠✨</div>
            <div className="safe-title">All Safe!</div>
            <div className="safe-sub">Everyone exempted. No one loses a life this round.</div>
          </div>
        )}

        {showdownInfo && (
          <div className="showdown-results">
            <div className="showdown-mode">
              {showdownInfo.isCowboy ? "🤠 Cowboy Rankings (Ace high)" : "Normal Rankings (Ace low)"}
            </div>

            <div className="showdown-reveals">
              {showdownInfo.reveals.map(([pid, card]) => {
                const p = getPlayer(pid);
                if (!p) return null;
                const isLoser = showdownInfo.losers.includes(pid);
                return (
                  <div
                    key={pid}
                    className={`showdown-player ${isLoser ? "showdown-loser" : "showdown-safe"}`}
                  >
                    <P
                      player={p}
                      isYou={pid === playerId}
                      size="medium"
                      glow={isLoser ? "red" : "green"}
                      label={isLoser ? "Lost a life!" : "Safe"}
                    />
                    <CardDisplay card={card} size="medium" highlight={isLoser} />
                  </div>
                );
              })}
            </div>

            {/* Show who was exempted */}
            {gameState.players.some(
              (p) =>
                !p.is_eliminated &&
                !showdownInfo.reveals.some(([id]) => id === p.id)
            ) && (
              <div className="showdown-exempted">
                <div className="exempted-label">Exempted (safe)</div>
                <div className="exempted-list">
                  {gameState.players
                    .filter(
                      (p) =>
                        !p.is_eliminated &&
                        !showdownInfo.reveals.some(([id]) => id === p.id)
                    )
                    .map((p) => (
                      <span key={p.id} className="exempted-name">
                        {p.name}
                        {p.id === playerId && " (you)"}
                      </span>
                    ))}
                </div>
              </div>
            )}
          </div>
        )}

        <div className="scene-actions">
          <button
            className="btn btn-primary btn-large"
            onClick={() => sendAction(send, { NextRound: null })}
          >
            Next Round →
          </button>
        </div>
      </div>
    );
  }

  // ==========================================
  // GAME OVER SCENE
  // ==========================================
  function GameOverScene() {
    const winner = gameState.players.find((p) => !p.is_eliminated);

    return (
      <div className="scene-gameover">
        <div className="gameover-trophy">🏆</div>
        <div className="gameover-title">
          {winner?.id === playerId ? "You Win!" : `${winner?.name || "Someone"} Wins!`}
        </div>
        {winner && (
          <P player={winner} isYou={winner.id === playerId} size="large" glow="gold" />
        )}
        <div className="gameover-rounds">
          {gameState.round_number} rounds played
        </div>
      </div>
    );
  }
}
