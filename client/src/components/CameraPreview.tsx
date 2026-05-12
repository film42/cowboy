import { useEffect, useRef, useState } from "react";

export interface MediaPrefs {
  camera: boolean;
  mic: boolean;
}

interface CameraPreviewProps {
  name: string;
  onConfirm: (prefs: MediaPrefs) => void;
  onBack: () => void;
}

export function CameraPreview({ name, onConfirm, onBack }: CameraPreviewProps) {
  const videoRef = useRef<HTMLVideoElement>(null);
  const streamRef = useRef<MediaStream | null>(null);
  const [ready, setReady] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [cameraOn, setCameraOn] = useState(true);
  const [micOn, setMicOn] = useState(true);

  useEffect(() => {
    let cancelled = false;

    navigator.mediaDevices
      .getUserMedia({ video: true, audio: true })
      .then((stream) => {
        if (cancelled) {
          stream.getTracks().forEach((t) => t.stop());
          return;
        }
        streamRef.current = stream;
        if (videoRef.current) {
          videoRef.current.srcObject = stream;
        }
        setReady(true);
      })
      .catch(() => {
        if (!cancelled) {
          setError("Camera access denied. You can still join without video.");
          setCameraOn(false);
          setMicOn(false);
          setReady(true);
        }
      });

    return () => {
      cancelled = true;
      streamRef.current?.getTracks().forEach((t) => t.stop());
    };
  }, []);

  const toggleCamera = () => {
    const stream = streamRef.current;
    if (!stream) return;
    const videoTrack = stream.getVideoTracks()[0];
    if (videoTrack) {
      const next = !cameraOn;
      videoTrack.enabled = next;
      setCameraOn(next);
    }
  };

  const toggleMic = () => {
    const stream = streamRef.current;
    if (!stream) return;
    const audioTrack = stream.getAudioTracks()[0];
    if (audioTrack) {
      const next = !micOn;
      audioTrack.enabled = next;
      setMicOn(next);
    }
  };

  const handleJoin = () => {
    streamRef.current?.getTracks().forEach((t) => t.stop());
    onConfirm({ camera: cameraOn, mic: micOn });
  };

  return (
    <div className="screen preview-screen">
      <h2 className="preview-title">You're about to join as</h2>
      <div className="preview-name">{name}</div>

      <div className={`preview-video-container ${!cameraOn ? "preview-video-off" : ""}`}>
        {/* Always mounted so srcObject persists */}
        <video
          ref={videoRef}
          autoPlay
          playsInline
          muted
          className="preview-video"
          style={{ display: cameraOn ? "block" : "none" }}
        />
        {!cameraOn && !error && (
          <div className="preview-avatar">
            <span>{name[0]}</span>
          </div>
        )}
        {!ready && !error && (
          <div className="preview-loading">Requesting camera...</div>
        )}
        {error && <div className="preview-error">{error}</div>}
      </div>

      {ready && !error && (
        <div className="preview-controls">
          <div className="toggle-control" onClick={toggleMic}>
            <span className="toggle-icon">{micOn ? "🎤" : "🔇"}</span>
            <span className="toggle-label">{micOn ? "Mic" : "Mic"}</span>
            <div className={`toggle-switch ${micOn ? "toggle-on" : "toggle-off"}`}>
              <div className="toggle-knob" />
            </div>
          </div>
          <div className="toggle-control" onClick={toggleCamera}>
            <span className="toggle-icon">{cameraOn ? "📹" : "📷"}</span>
            <span className="toggle-label">Camera</span>
            <div className={`toggle-switch ${cameraOn ? "toggle-on" : "toggle-off"}`}>
              <div className="toggle-knob" />
            </div>
          </div>
        </div>
      )}

      <p className="preview-hint">
        {cameraOn || micOn
          ? "Other players will see and hear you"
          : "You can turn these on later in the lobby"}
      </p>

      <div className="button-group">
        <button
          className="btn btn-primary btn-large"
          onClick={handleJoin}
          disabled={!ready}
        >
          Join Lobby
        </button>
        <button className="btn btn-ghost" onClick={onBack}>
          Back
        </button>
      </div>
    </div>
  );
}
