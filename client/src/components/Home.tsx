import { useState } from "react";

interface HomeProps {
  onCreateLobby: (name: string) => void;
  onJoinLobby: (code: string, name: string) => void;
}

export function Home({ onCreateLobby, onJoinLobby }: HomeProps) {
  const [name, setName] = useState("");
  const [joinCode, setJoinCode] = useState("");
  const [mode, setMode] = useState<"home" | "join">("home");

  const handleCreate = () => {
    if (name.trim()) {
      onCreateLobby(name.trim());
    }
  };

  const handleJoin = () => {
    if (name.trim() && joinCode.trim()) {
      onJoinLobby(joinCode.trim().toUpperCase(), name.trim());
    }
  };

  return (
    <div className="screen home-screen">
      <h1 className="title">COWBOY</h1>
      <p className="subtitle">The Card Game</p>

      <div className="input-group">
        <input
          type="text"
          placeholder="Your name"
          value={name}
          onChange={(e) => setName(e.target.value)}
          maxLength={20}
          autoFocus
        />
      </div>

      {mode === "home" ? (
        <div className="button-group">
          <button
            className="btn btn-primary"
            onClick={handleCreate}
            disabled={!name.trim()}
          >
            Create Lobby
          </button>
          <button
            className="btn btn-secondary"
            onClick={() => setMode("join")}
            disabled={!name.trim()}
          >
            Join Game
          </button>
        </div>
      ) : (
        <div className="button-group">
          <input
            type="text"
            placeholder="Invite code"
            value={joinCode}
            onChange={(e) => setJoinCode(e.target.value.toUpperCase())}
            maxLength={4}
            className="code-input"
            autoFocus
          />
          <button
            className="btn btn-primary"
            onClick={handleJoin}
            disabled={!joinCode.trim() || !name.trim()}
          >
            Join
          </button>
          <button className="btn btn-ghost" onClick={() => setMode("home")}>
            Back
          </button>
        </div>
      )}
    </div>
  );
}
