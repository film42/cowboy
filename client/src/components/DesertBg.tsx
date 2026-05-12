function CloudsSvg() {
  return (
    <svg viewBox="0 0 1600 120" preserveAspectRatio="none">
      <ellipse cx="200" cy="45" rx="80" ry="22" fill="white" opacity="0.35" />
      <ellipse cx="240" cy="38" rx="50" ry="16" fill="white" opacity="0.3" />
      <ellipse cx="160" cy="40" rx="40" ry="14" fill="white" opacity="0.25" />
      <ellipse cx="600" cy="55" rx="70" ry="18" fill="white" opacity="0.3" />
      <ellipse cx="640" cy="48" rx="45" ry="14" fill="white" opacity="0.2" />
      <ellipse cx="1000" cy="35" rx="60" ry="20" fill="white" opacity="0.3" />
      <ellipse cx="1040" cy="30" rx="40" ry="12" fill="white" opacity="0.25" />
      <ellipse cx="1350" cy="60" rx="55" ry="16" fill="white" opacity="0.25" />
      <ellipse cx="1380" cy="54" rx="35" ry="12" fill="white" opacity="0.2" />
    </svg>
  );
}

function FarMountainsSvg() {
  return (
    <svg viewBox="0 0 1600 200" preserveAspectRatio="xMidYMax slice">
      <defs>
        <linearGradient id="farMtn" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0%" stopColor="#c8bab5" />
          <stop offset="100%" stopColor="#d0c4b8" />
        </linearGradient>
      </defs>
      <path d="M0 140 L80 80 L140 110 L220 60 L300 100 L380 55 L440 90 L520 70 L600 105 L700 50 L780 85 L860 65 L940 95 L1020 45 L1100 80 L1180 60 L1260 100 L1340 55 L1420 85 L1500 70 L1600 95 L1600 200 L0 200Z" fill="url(#farMtn)" opacity="0.6" />
    </svg>
  );
}

function MidButtesSvg() {
  return (
    <svg viewBox="0 0 1600 280" preserveAspectRatio="xMidYMax slice">
      {/* Left mesa */}
      <path d="M60 280 L70 130 L90 105 L140 100 L160 105 L170 130 L180 280Z" fill="#b89880" />
      <path d="M85 108 L90 103 L140 99 L145 103Z" fill="#a88870" opacity="0.5" />

      {/* Tall butte */}
      <path d="M350 280 L365 100 L380 70 L400 60 L420 70 L435 100 L450 280Z" fill="#c4a088" />
      <path d="M375 75 L380 68 L400 57 L420 68 L425 75Z" fill="#a88870" opacity="0.4" />

      {/* Wide mesa */}
      <path d="M620 280 L640 140 L660 120 L760 115 L780 120 L800 140 L820 280Z" fill="#c8a890" />
      <path d="M655 123 L660 118 L760 113 L765 118Z" fill="#b09078" opacity="0.4" />

      {/* Small butte */}
      <path d="M1000 280 L1015 160 L1030 140 L1050 135 L1070 140 L1085 160 L1100 280Z" fill="#c4a088" />

      {/* Right formation */}
      <path d="M1250 280 L1260 120 L1280 90 L1320 80 L1360 90 L1380 120 L1390 280Z" fill="#b89880" />
      <path d="M1275 95 L1280 88 L1320 77 L1360 88 L1365 95Z" fill="#a08068" opacity="0.4" />

      {/* Distant small */}
      <path d="M1500 280 L1510 170 L1525 155 L1540 170 L1550 280Z" fill="#c8a890" opacity="0.6" />
    </svg>
  );
}

function NearGroundSvg() {
  return (
    <svg viewBox="0 0 1600 180" preserveAspectRatio="xMidYMax slice">
      {/* Ground */}
      <path d="M0 60 Q200 50 400 58 Q600 66 800 55 Q1000 48 1200 58 Q1400 65 1600 52 L1600 180 L0 180Z" fill="#d8c8a8" />
      <path d="M0 80 Q300 72 600 78 Q900 85 1200 75 Q1400 70 1600 76 L1600 180 L0 180Z" fill="#ccb898" opacity="0.6" />

      {/* Saguaro 1 */}
      <g transform="translate(120, 20)">
        <rect x="8" y="10" width="8" height="50" rx="4" fill="#8a9a78" />
        <rect x="0" y="25" width="8" height="20" rx="4" fill="#7a8a68" transform="rotate(-15 4 35)" />
        <rect x="16" y="20" width="7" height="18" rx="3.5" fill="#7a8a68" transform="rotate(12 19 29)" />
      </g>

      {/* Small cactus */}
      <g transform="translate(320, 45)">
        <rect x="3" y="5" width="6" height="22" rx="3" fill="#8a9a78" />
        <rect x="0" y="12" width="5" height="10" rx="2.5" fill="#7a8a68" transform="rotate(-20 2 17)" />
      </g>

      {/* Saguaro 2 */}
      <g transform="translate(550, 15)">
        <rect x="8" y="8" width="9" height="55" rx="4.5" fill="#8a9a78" />
        <rect x="0" y="20" width="8" height="22" rx="4" fill="#7a8a68" transform="rotate(-12 4 31)" />
        <rect x="17" y="28" width="7" height="16" rx="3.5" fill="#7a8a68" transform="rotate(18 20 36)" />
      </g>

      {/* Bushes */}
      <ellipse cx="220" cy="70" rx="12" ry="8" fill="#9aa888" opacity="0.6" />
      <ellipse cx="460" cy="65" rx="10" ry="6" fill="#8a9878" opacity="0.5" />
      <ellipse cx="700" cy="72" rx="14" ry="7" fill="#9aa888" opacity="0.5" />

      {/* Small cactus */}
      <g transform="translate(830, 40)">
        <rect x="3" y="5" width="5" height="20" rx="2.5" fill="#8a9a78" />
        <rect x="8" y="10" width="4" height="10" rx="2" fill="#7a8a68" transform="rotate(15 10 15)" />
      </g>

      {/* Saguaro 3 */}
      <g transform="translate(1050, 18)">
        <rect x="7" y="10" width="8" height="48" rx="4" fill="#8a9a78" />
        <rect x="0" y="22" width="7" height="18" rx="3.5" fill="#7a8a68" transform="rotate(-10 3 31)" />
        <rect x="15" y="18" width="7" height="22" rx="3.5" fill="#7a8a68" transform="rotate(14 18 29)" />
      </g>

      {/* Rocks */}
      <ellipse cx="180" cy="75" rx="8" ry="4" fill="#b8a890" opacity="0.5" />
      <ellipse cx="400" cy="68" rx="6" ry="3" fill="#b0a088" opacity="0.4" />
      <ellipse cx="650" cy="74" rx="10" ry="4" fill="#b8a890" opacity="0.45" />
      <ellipse cx="950" cy="70" rx="7" ry="3.5" fill="#b0a088" opacity="0.4" />
      <ellipse cx="1200" cy="72" rx="9" ry="4" fill="#b8a890" opacity="0.5" />

      {/* Tumbleweeds */}
      <circle cx="280" cy="73" r="6" fill="none" stroke="#b8a898" strokeWidth="1" opacity="0.3" />
      <circle cx="900" cy="68" r="5" fill="none" stroke="#b8a898" strokeWidth="0.8" opacity="0.25" />

      {/* More bushes */}
      <ellipse cx="1150" cy="68" rx="11" ry="6" fill="#9aa888" opacity="0.5" />
      <ellipse cx="1380" cy="72" rx="8" ry="5" fill="#8a9878" opacity="0.45" />

      {/* Grass tufts */}
      <g opacity="0.4">
        <line x1="150" y1="78" x2="148" y2="68" stroke="#a0a888" strokeWidth="1.5" />
        <line x1="153" y1="78" x2="155" y2="69" stroke="#a0a888" strokeWidth="1.5" />
        <line x1="500" y1="72" x2="498" y2="63" stroke="#a0a888" strokeWidth="1.5" />
        <line x1="503" y1="72" x2="506" y2="64" stroke="#a0a888" strokeWidth="1.5" />
        <line x1="780" y1="76" x2="778" y2="67" stroke="#a0a888" strokeWidth="1.5" />
        <line x1="1100" y1="70" x2="1098" y2="61" stroke="#a0a888" strokeWidth="1.5" />
        <line x1="1103" y1="70" x2="1106" y2="62" stroke="#a0a888" strokeWidth="1.5" />
        <line x1="1450" y1="74" x2="1448" y2="65" stroke="#a0a888" strokeWidth="1.5" />
      </g>
    </svg>
  );
}

export function DesertBg() {
  return (
    <div className="desert-bg" aria-hidden="true">
      <div className="desert-sky" />
      <div className="desert-sun" />

      <div className="desert-clouds">
        <CloudsSvg />
        <CloudsSvg />
      </div>

      <div className="desert-layer desert-far">
        <FarMountainsSvg />
        <FarMountainsSvg />
      </div>

      <div className="desert-layer desert-mid">
        <MidButtesSvg />
        <MidButtesSvg />
      </div>

      <div className="desert-layer desert-near">
        <NearGroundSvg />
        <NearGroundSvg />
      </div>
    </div>
  );
}
