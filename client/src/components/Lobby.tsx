import { useState } from "react";
import type { ClientMessage, LobbyState, PlayerId } from "../types/game";
import type { LiveKitState } from "../hooks/useLiveKit";
import { MediaView } from "./MediaView";

interface LobbyProps {
  lobbyState: LobbyState;
  playerId: PlayerId;
  send: (msg: ClientMessage) => void;
  livekit: LiveKitState;
}

export function Lobby({ lobbyState, playerId, send, livekit }: LobbyProps) {
  const [lives, setLives] = useState(3);
  const [blockers, setBlockers] = useState(1);
  const [exemptions, setExemptions] = useState(1);

  const isHost = lobbyState.host_id === playerId;
  const connectedCount = lobbyState.players.filter(
    (p) => p.is_connected
  ).length;

  const handleStart = () => {
    send({
      type: "StartGame",
      lives,
      blocker_tokens: blockers,
      exemption_tokens: exemptions,
    });
  };

  const handleCopyCode = () => {
    navigator.clipboard.writeText(lobbyState.code);
  };

  return (
    <div className="screen lobby-screen">
      <div className="lobby-header">
        <h2>Lobby</h2>
        <button className="invite-code" onClick={handleCopyCode}>
          {lobbyState.code}
          <span className="copy-hint">tap to copy</span>
        </button>
      </div>

      {livekit.connected && (
        <div className="lobby-media-controls">
          <button
            className={`btn-media ${livekit.isMuted ? "btn-media-off" : ""}`}
            onClick={livekit.toggleMute}
          >
            {livekit.isMuted ? "🔇 Muted" : "🎤 Mic On"}
          </button>
          <button
            className={`btn-media ${livekit.isVideoOff ? "btn-media-off" : ""}`}
            onClick={livekit.toggleVideo}
          >
            {livekit.isVideoOff ? "📷 Video Off" : "📹 Video On"}
          </button>
        </div>
      )}

      {lobbyState.game_history.length > 0 && (
        <div className="last-winner-banner">
          <span className="last-winner-trophy">🏆</span>
          <span className="last-winner-text">
            {lobbyState.game_history[lobbyState.game_history.length - 1].winner_name} won the last game!
          </span>
        </div>
      )}

      <div className="player-list">
        <h3>
          Players ({connectedCount}/{lobbyState.players.length})
        </h3>
        {lobbyState.players.map((p) => (
          <div
            key={p.id}
            className={`player-row ${!p.is_connected ? "disconnected" : ""}`}
          >
            <span className="player-name">
              {p.name}
              {p.id === playerId && " (you)"}
            </span>
            <span className="player-badges">
              {p.is_host && <span className="badge host">Host</span>}
              {!p.is_connected && (
                <span className="badge offline">Offline</span>
              )}
            </span>
          </div>
        ))}
      </div>

      {isHost && (
        <div className="game-settings">
          <h3>Game Settings</h3>
          <div className="setting-row">
            <label>Lives</label>
            <div className="stepper">
              <button onClick={() => setLives(Math.max(1, lives - 1))}>
                -
              </button>
              <span>{lives}</span>
              <button onClick={() => setLives(Math.min(5, lives + 1))}>
                +
              </button>
            </div>
          </div>
          <div className="setting-row">
            <label>Blockers</label>
            <div className="stepper">
              <button onClick={() => setBlockers(Math.max(0, blockers - 1))}>
                -
              </button>
              <span>{blockers}</span>
              <button onClick={() => setBlockers(Math.min(3, blockers + 1))}>
                +
              </button>
            </div>
          </div>
          <div className={`setting-row ${lives === 1 ? "setting-disabled" : ""}`}>
            <label>Exemptions</label>
            <div className="stepper">
              <button
                onClick={() => setExemptions(Math.max(0, exemptions - 1))}
                disabled={lives === 1}
              >
                -
              </button>
              <span>{lives === 1 ? 0 : exemptions}</span>
              <button
                onClick={() => setExemptions(Math.min(3, exemptions + 1))}
                disabled={lives === 1}
              >
                +
              </button>
            </div>
          </div>
          {lives === 1 && (
            <p className="setting-note">
              With 1 life, exemption tokens are disabled
            </p>
          )}
          <div className="start-btn-wrapper">
            <button
              className="btn btn-primary btn-large"
              onClick={handleStart}
              disabled={connectedCount < 2}
            >
              {connectedCount < 2 ? "Need 2+ players" : "Start Game"}
            </button>
          </div>
        </div>
      )}

      {!isHost && (
        <p className="waiting-text">Waiting for host to start the game...</p>
      )}

      {lobbyState.game_history.length > 0 && (
        <div className="game-history">
          <h3>Previous Games</h3>
          {lobbyState.game_history.map((g) => (
            <div key={g.game_number} className="history-row">
              Game {g.game_number}: {g.winner_name} won ({g.round_count}{" "}
              rounds, {g.player_count} players)
            </div>
          ))}
        </div>
      )}

      {/* Play remote audio in lobby */}
      {Array.from(livekit.participants.entries()).map(([pid, media]) => (
        <MediaView key={pid} videoTrack={null} audioTrack={media.audioTrack} />
      ))}
    </div>
  );
}
