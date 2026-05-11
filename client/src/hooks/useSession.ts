import { useState } from "react";
import type { PlayerId } from "../types/game";

interface Session {
  lobbyCode: string;
  playerId: PlayerId;
  sessionToken: string;
}

export function useSession() {
  const [session, setSession] = useState<Session | null>(() => {
    const stored = sessionStorage.getItem("cowboy_session");
    return stored ? JSON.parse(stored) : null;
  });

  const saveSession = (s: Session) => {
    sessionStorage.setItem("cowboy_session", JSON.stringify(s));
    setSession(s);
  };

  const clearSession = () => {
    sessionStorage.removeItem("cowboy_session");
    setSession(null);
  };

  return { session, saveSession, clearSession };
}
