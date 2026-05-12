import { useEffect, useRef } from "react";

interface MediaViewProps {
  videoTrack: MediaStreamTrack | null;
  audioTrack: MediaStreamTrack | null;
  muted?: boolean;
  className?: string;
}

export function MediaView({
  videoTrack,
  audioTrack,
  muted = false,
  className = "",
}: MediaViewProps) {
  const videoRef = useRef<HTMLVideoElement>(null);
  const audioRef = useRef<HTMLAudioElement>(null);

  useEffect(() => {
    if (videoRef.current && videoTrack) {
      const stream = new MediaStream([videoTrack]);
      videoRef.current.srcObject = stream;
    } else if (videoRef.current) {
      videoRef.current.srcObject = null;
    }
  }, [videoTrack]);

  useEffect(() => {
    if (audioRef.current && audioTrack) {
      const stream = new MediaStream([audioTrack]);
      audioRef.current.srcObject = stream;
    } else if (audioRef.current) {
      audioRef.current.srcObject = null;
    }
  }, [audioTrack]);

  return (
    <>
      {videoTrack && (
        <video
          ref={videoRef}
          autoPlay
          playsInline
          muted
          className={`media-video ${className}`}
        />
      )}
      {/* Always keep audio element mounted to avoid playback interruption */}
      <audio ref={audioRef} autoPlay muted={muted} style={{ display: "none" }} />
    </>
  );
}
