import { useCountdown } from "../hooks/useCountdown";

interface CountdownProps {
  seconds: number;
  /** Change this to restart the countdown */
  resetKey: string;
}

export function Countdown({ seconds, resetKey }: CountdownProps) {
  const remaining = useCountdown(seconds, resetKey);
  const urgent = remaining <= 5;
  const pct = remaining / seconds;

  return (
    <div className={`countdown ${urgent ? "countdown-urgent" : ""}`}>
      <svg className="countdown-ring" viewBox="0 0 36 36">
        <circle
          className="countdown-ring-bg"
          cx="18" cy="18" r="15.5"
          fill="none"
          strokeWidth="3"
        />
        <circle
          className="countdown-ring-fill"
          cx="18" cy="18" r="15.5"
          fill="none"
          strokeWidth="3"
          strokeDasharray={`${pct * 97.4} 97.4`}
          strokeLinecap="round"
          transform="rotate(-90 18 18)"
        />
      </svg>
      <span className="countdown-num">{remaining}</span>
    </div>
  );
}
