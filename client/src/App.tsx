import { useEffect, useRef, useState } from "react";
import type { LobbyState } from "./types/game";
import { Home } from "./components/Home";
import { Lobby } from "./components/Lobby";
import { GameView } from "./components/GameView";
import { useWebSocket } from "./hooks/useWebSocket";
import { useSession } from "./hooks/useSession";

function App() {
  const { session, saveSession, clearSession } = useSession();
  const [joining, setJoining] = useState(false);
  const [holdGameView, setHoldGameView] = useState(false);
  const holdTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const [initialLobbyState, setInitialLobbyState] = useState<LobbyState | null>(null);
  const wasGameActive = useRef(false);

  const {
    lobbyState,
    gameState,
    playerId,
    events,
    error,
    connected,
    send,
    clearEvents,
    clearError,
  } = useWebSocket(
    session?.lobbyCode ?? null,
    session?.sessionToken ?? null
  );

  const currentPlayerId = playerId ?? session?.playerId ?? null;
  const effectiveLobbyState = lobbyState ?? initialLobbyState;

  // Clear initialLobbyState once WS delivers real data
  useEffect(() => {
    if (lobbyState && initialLobbyState) {
      setInitialLobbyState(null);
    }
  }, [lobbyState, initialLobbyState]);

  // Hold the game over screen for 5 seconds before returning to lobby
  useEffect(() => {
    if (effectiveLobbyState?.game_active) {
      wasGameActive.current = true;
    } else if (wasGameActive.current && gameState?.phase === "game_over") {
      wasGameActive.current = false;
      setHoldGameView(true);
      holdTimer.current = setTimeout(() => setHoldGameView(false), 5000);
    }
    return () => {
      if (holdTimer.current) clearTimeout(holdTimer.current);
    };
  }, [effectiveLobbyState?.game_active, gameState?.phase]);

  const fetchLobbyState = async (code: string) => {
    const res = await fetch(`/api/lobby/${code}`);
    if (res.ok) {
      const state = await res.json();
      setInitialLobbyState(state);
    }
  };

  const handleCreateLobby = async (name: string) => {
    setJoining(true);
    try {
      const res = await fetch("/api/lobby", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ host_name: name }),
      });
      if (!res.ok) throw new Error("Failed to create lobby");
      const data = await res.json();
      saveSession({
        lobbyCode: data.code,
        playerId: data.player_id,
        sessionToken: data.session_token,
      });
      await fetchLobbyState(data.code);
    } catch {
      clearError();
    } finally {
      setJoining(false);
    }
  };

  const handleJoinLobby = async (code: string, name: string) => {
    setJoining(true);
    try {
      const res = await fetch(`/api/lobby/${code}/join`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ name }),
      });
      if (!res.ok) throw new Error("Failed to join lobby");
      const data = await res.json();
      saveSession({
        lobbyCode: code,
        playerId: data.player_id,
        sessionToken: data.session_token,
      });
      await fetchLobbyState(code);
    } catch {
      clearError();
    } finally {
      setJoining(false);
    }
  };

  const handleLeave = () => {
    setInitialLobbyState(null);
    clearSession();
  };

  // No session: home screen
  if (!session) {
    return (
      <div className="app">
        <Home
          onCreateLobby={handleCreateLobby}
          onJoinLobby={handleJoinLobby}
        />
        {joining && <div className="loading-overlay">Connecting...</div>}
      </div>
    );
  }

  // In a game (or holding the win screen)
  if (currentPlayerId && gameState && (effectiveLobbyState?.game_active || holdGameView)) {
    return (
      <div className="app">
        <GameView
          gameState={gameState}
          playerId={currentPlayerId}
          events={events}
          send={send}
          clearEvents={clearEvents}
          isHost={effectiveLobbyState?.host_id === currentPlayerId}
        />
        {error && (
          <div className="error-toast" onClick={clearError}>
            {error}
          </div>
        )}
        {!connected && (
          <div className="reconnecting-bar">Reconnecting...</div>
        )}
      </div>
    );
  }

  // In lobby
  if (currentPlayerId && effectiveLobbyState) {
    return (
      <div className="app">
        <Lobby
          lobbyState={effectiveLobbyState}
          playerId={currentPlayerId}
          send={send}
        />
        <button className="btn btn-ghost leave-btn" onClick={handleLeave}>
          Leave Lobby
        </button>
        {error && (
          <div className="error-toast" onClick={clearError}>
            {error}
          </div>
        )}
        {!connected && (
          <div className="reconnecting-bar">Reconnecting...</div>
        )}
      </div>
    );
  }

  // Connecting...
  return (
    <div className="app">
      <div className="screen">
        <div className="loading-text">Connecting...</div>
      </div>
    </div>
  );
}

export default App;
