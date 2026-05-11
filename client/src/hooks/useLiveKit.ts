import { useCallback, useEffect, useRef, useState } from "react";
import {
  Room,
  RoomEvent,
  Track,
  RemoteTrackPublication,
  RemoteParticipant,
  LocalParticipant,
  ConnectionState,
} from "livekit-client";
import type { PlayerId } from "../types/game";

interface ParticipantMedia {
  audioTrack: MediaStreamTrack | null;
  videoTrack: MediaStreamTrack | null;
}

export interface LiveKitState {
  connected: boolean;
  participants: Map<PlayerId, ParticipantMedia>;
  localMedia: ParticipantMedia;
  toggleMute: () => void;
  toggleVideo: () => void;
  isMuted: boolean;
  isVideoOff: boolean;
}

export function useLiveKit(
  lobbyCode: string | null,
  playerId: PlayerId | null,
  playerName: string | null
): LiveKitState {
  const roomRef = useRef<Room | null>(null);
  const [connected, setConnected] = useState(false);
  const [participants, setParticipants] = useState<Map<PlayerId, ParticipantMedia>>(new Map());
  const [localMedia, setLocalMedia] = useState<ParticipantMedia>({ audioTrack: null, videoTrack: null });
  const [isMuted, setIsMuted] = useState(false);
  const [isVideoOff, setIsVideoOff] = useState(false);

  const updateParticipants = useCallback((room: Room) => {
    const map = new Map<PlayerId, ParticipantMedia>();

    for (const [, participant] of room.remoteParticipants) {
      const pid = parsePlayerId(participant.identity);
      if (pid === null) continue;

      let audioTrack: MediaStreamTrack | null = null;
      let videoTrack: MediaStreamTrack | null = null;

      for (const [, pub_] of participant.trackPublications) {
        if (pub_.track && pub_.isSubscribed) {
          if (pub_.track.kind === Track.Kind.Audio) {
            audioTrack = pub_.track.mediaStreamTrack;
          } else if (pub_.track.kind === Track.Kind.Video) {
            videoTrack = pub_.track.mediaStreamTrack;
          }
        }
      }

      map.set(pid, { audioTrack, videoTrack });
    }

    setParticipants(new Map(map));
  }, []);

  const updateLocalMedia = useCallback((room: Room) => {
    const local = room.localParticipant;
    let audioTrack: MediaStreamTrack | null = null;
    let videoTrack: MediaStreamTrack | null = null;

    for (const [, pub_] of local.trackPublications) {
      if (pub_.track) {
        if (pub_.track.kind === Track.Kind.Audio) {
          audioTrack = pub_.track.mediaStreamTrack;
        } else if (pub_.track.kind === Track.Kind.Video) {
          videoTrack = pub_.track.mediaStreamTrack;
        }
      }
    }

    setLocalMedia({ audioTrack, videoTrack });
  }, []);

  useEffect(() => {
    if (!lobbyCode || !playerId || !playerName) return;

    let cancelled = false;
    const room = new Room({
      adaptiveStream: true,
      dynacast: true,
      videoCaptureDefaults: {
        resolution: { width: 320, height: 240, frameRate: 15 },
      },
    });
    roomRef.current = room;

    const onTrackSubscribed = () => updateParticipants(room);
    const onTrackUnsubscribed = () => updateParticipants(room);
    const onLocalTrackPublished = () => updateLocalMedia(room);
    const onConnected = () => {
      if (!cancelled) setConnected(true);
    };
    const onDisconnected = () => {
      if (!cancelled) setConnected(false);
    };

    room.on(RoomEvent.TrackSubscribed, onTrackSubscribed);
    room.on(RoomEvent.TrackUnsubscribed, onTrackUnsubscribed);
    room.on(RoomEvent.LocalTrackPublished, onLocalTrackPublished);
    room.on(RoomEvent.Connected, onConnected);
    room.on(RoomEvent.Disconnected, onDisconnected);
    room.on(RoomEvent.ParticipantConnected, () => updateParticipants(room));
    room.on(RoomEvent.ParticipantDisconnected, () => updateParticipants(room));

    // Fetch token and connect
    (async () => {
      try {
        const res = await fetch("/api/livekit/token", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            lobby_code: lobbyCode,
            player_name: playerName,
            player_id: playerId,
          }),
        });
        if (!res.ok) return;
        const { token } = await res.json();

        if (cancelled) return;

        // Connect through Vite's proxy to avoid mixed content issues on HTTPS
        const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
        const livekitUrl = `${protocol}//${window.location.host}/livekit-ws`;

        await room.connect(livekitUrl, token);

        await room.localParticipant.setMicrophoneEnabled(true);
        await room.localParticipant.setCameraEnabled(true);
        updateLocalMedia(room);
      } catch (err) {
        console.error("LiveKit connection failed:", err);
      }
    })();

    return () => {
      cancelled = true;
      room.disconnect();
      roomRef.current = null;
    };
  }, [lobbyCode, playerId, playerName, updateParticipants, updateLocalMedia]);

  const toggleMute = useCallback(() => {
    const room = roomRef.current;
    if (!room) return;
    const newMuted = !isMuted;
    room.localParticipant.setMicrophoneEnabled(!newMuted);
    setIsMuted(newMuted);
  }, [isMuted]);

  const toggleVideo = useCallback(() => {
    const room = roomRef.current;
    if (!room) return;
    const newOff = !isVideoOff;
    room.localParticipant.setCameraEnabled(!newOff);
    setIsVideoOff(newOff);
    if (!newOff) {
      // Small delay for track to publish
      setTimeout(() => updateLocalMedia(room), 500);
    }
  }, [isVideoOff, updateLocalMedia]);

  return {
    connected,
    participants,
    localMedia,
    toggleMute,
    toggleVideo,
    isMuted,
    isVideoOff,
  };
}

function parsePlayerId(identity: string): PlayerId | null {
  // identity format: "player-{id}"
  const match = identity.match(/^player-(\d+)$/);
  return match ? parseInt(match[1], 10) : null;
}
