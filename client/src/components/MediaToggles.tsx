import type { LiveKitState } from "../hooks/useLiveKit";

interface MediaTogglesProps {
  livekit: LiveKitState;
  layout?: "row" | "compact";
}

export function MediaToggles({ livekit, layout = "row" }: MediaTogglesProps) {
  if (!livekit.connected) return null;

  return (
    <div className={`media-toggles media-toggles-${layout}`}>
      <div className="toggle-control" onClick={livekit.toggleMute}>
        <span className="toggle-icon">{livekit.isMuted ? "🔇" : "🎤"}</span>
        {layout === "row" && <span className="toggle-label">Mic</span>}
        <div className={`toggle-switch ${!livekit.isMuted ? "toggle-on" : "toggle-off"}`}>
          <div className="toggle-knob" />
        </div>
      </div>
      <div className="toggle-control" onClick={livekit.toggleVideo}>
        <span className="toggle-icon">{livekit.isVideoOff ? "📷" : "📹"}</span>
        {layout === "row" && <span className="toggle-label">Camera</span>}
        <div className={`toggle-switch ${!livekit.isVideoOff ? "toggle-on" : "toggle-off"}`}>
          <div className="toggle-knob" />
        </div>
      </div>
    </div>
  );
}
