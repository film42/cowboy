import { useEffect, useRef, useState } from "react";
import type { LobbyState } from "./types/game";
import { Home } from "./components/Home";
import { Lobby } from "./components/Lobby";
import { GameView } from "./components/GameView";
import { CameraPreview } from "./components/CameraPreview";
import type { MediaPrefs as CameraMediaPrefs } from "./components/CameraPreview";
import { useWebSocket } from "./hooks/useWebSocket";
import { useSession } from "./hooks/useSession";
import { useLiveKit } from "./hooks/useLiveKit";

interface PendingAction {
  type: "create" | "join";
  name: string;
  code?: string;
}

function App() {
  const { session, saveSession, clearSession } = useSession();
  const [joining, setJoining] = useState(false);
  const [holdGameView, setHoldGameView] = useState(false);
  const holdTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const [initialLobbyState, setInitialLobbyState] = useState<LobbyState | null>(null);
  const wasGameActive = useRef(false);
  const [pendingAction, setPendingAction] = useState<PendingAction | null>(null);
  const [mediaPrefs, setMediaPrefs] = useState<CameraMediaPrefs>({ camera: true, mic: true });

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

  const currentPlayerName = effectiveLobbyState?.players.find(
    (p) => p.id === currentPlayerId
  )?.name ?? null;

  const livekit = useLiveKit(
    session?.lobbyCode ?? null,
    currentPlayerId,
    currentPlayerName,
    mediaPrefs
  );

  useEffect(() => {
    if (lobbyState && initialLobbyState) {
      setInitialLobbyState(null);
    }
  }, [lobbyState, initialLobbyState]);

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

  // Home screen handlers just set the pending action → show camera preview
  const handleCreateLobby = (name: string) => {
    setPendingAction({ type: "create", name });
  };

  const handleJoinLobby = (code: string, name: string) => {
    setPendingAction({ type: "join", name, code });
  };

  // After camera preview confirmation, actually create/join
  const handlePreviewConfirm = async (prefs: CameraMediaPrefs) => {
    if (!pendingAction) return;
    setMediaPrefs(prefs);
    setJoining(true);
    setPendingAction(null);

    try {
      if (pendingAction.type === "create") {
        const res = await fetch("/api/lobby", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ host_name: pendingAction.name }),
        });
        if (!res.ok) throw new Error("Failed to create lobby");
        const data = await res.json();
        saveSession({
          lobbyCode: data.code,
          playerId: data.player_id,
          sessionToken: data.session_token,
        });
        await fetchLobbyState(data.code);
      } else {
        const res = await fetch(`/api/lobby/${pendingAction.code}/join`, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ name: pendingAction.name }),
        });
        if (!res.ok) throw new Error("Failed to join lobby");
        const data = await res.json();
        saveSession({
          lobbyCode: pendingAction.code!,
          playerId: data.player_id,
          sessionToken: data.session_token,
        });
        await fetchLobbyState(pendingAction.code!);
      }
    } catch {
      clearError();
    } finally {
      setJoining(false);
    }
  };

  const handleLeave = async () => {
    if (session) {
      await fetch(`/api/lobby/${session.lobbyCode}/leave`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ player_id: session.playerId }),
      }).catch(() => {});
    }
    setInitialLobbyState(null);
    clearSession();
  };

  // Camera preview step
  if (pendingAction) {
    return (
      <div className="app">
        <CameraPreview
          name={pendingAction.name}
          onConfirm={handlePreviewConfirm}
          onBack={() => setPendingAction(null)}
        />
        {joining && <div className="loading-overlay">Joining...</div>}
      </div>
    );
  }

  // No session: home screen
  if (!session) {
    return (
      <div className="app">
        <Home
          onCreateLobby={handleCreateLobby}
          onJoinLobby={handleJoinLobby}
        />
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
          livekit={livekit}
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
          livekit={livekit}
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
