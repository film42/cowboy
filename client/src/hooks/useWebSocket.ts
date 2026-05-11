import { useCallback, useEffect, useRef, useState } from "react";
import type {
  ClientMessage,
  GameEvent,
  LobbyState,
  PlayerGameState,
  PlayerId,
  ServerMessage,
} from "../types/game";

interface UseWebSocketReturn {
  lobbyState: LobbyState | null;
  gameState: PlayerGameState | null;
  playerId: PlayerId | null;
  events: GameEvent[];
  error: string | null;
  connected: boolean;
  send: (msg: ClientMessage) => void;
  clearEvents: () => void;
  clearError: () => void;
}

export function useWebSocket(
  lobbyCode: string | null,
  sessionToken: string | null
): UseWebSocketReturn {
  const wsRef = useRef<WebSocket | null>(null);
  const [lobbyState, setLobbyState] = useState<LobbyState | null>(null);
  const [gameState, setGameState] = useState<PlayerGameState | null>(null);
  const [playerId, setPlayerId] = useState<PlayerId | null>(null);
  const [events, setEvents] = useState<GameEvent[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [connected, setConnected] = useState(false);
  const reconnectTimeout = useRef<ReturnType<typeof setTimeout> | null>(null);

  const connect = useCallback(() => {
    if (!lobbyCode || !sessionToken) return;

    const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
    const host = window.location.host;
    const url = `${protocol}//${host}/ws/${lobbyCode}?session_token=${sessionToken}`;

    const ws = new WebSocket(url);
    wsRef.current = ws;

    ws.onopen = () => {
      setConnected(true);
      setError(null);
    };

    ws.onclose = () => {
      setConnected(false);
      reconnectTimeout.current = setTimeout(connect, 2000);
    };

    ws.onerror = () => {
      setError("Connection error");
    };

    ws.onmessage = (event) => {
      const msg: ServerMessage = JSON.parse(event.data);

      switch (msg.type) {
        case "Welcome":
          setPlayerId(msg.player_id);
          break;
        case "LobbyUpdate":
          setLobbyState(msg.state);
          break;
        case "GameState":
          setGameState(msg.state);
          break;
        case "GameEvents":
          setEvents((prev) => [...prev, ...msg.events]);
          break;
        case "Error":
          setError(msg.message);
          break;
      }
    };
  }, [lobbyCode, sessionToken]);

  useEffect(() => {
    connect();
    return () => {
      if (reconnectTimeout.current) {
        clearTimeout(reconnectTimeout.current);
      }
      wsRef.current?.close();
    };
  }, [connect]);

  const send = useCallback((msg: ClientMessage) => {
    if (wsRef.current?.readyState === WebSocket.OPEN) {
      wsRef.current.send(JSON.stringify(msg));
    }
  }, []);

  const clearEvents = useCallback(() => setEvents([]), []);
  const clearError = useCallback(() => setError(null), []);

  return {
    lobbyState,
    gameState,
    playerId,
    events,
    error,
    connected,
    send,
    clearEvents,
    clearError,
  };
}
