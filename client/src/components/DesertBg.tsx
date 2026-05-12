export function DesertBg() {
  return (
    <svg
      className="desert-bg"
      viewBox="0 0 400 260"
      preserveAspectRatio="xMidYMax slice"
      xmlns="http://www.w3.org/2000/svg"
    >
      {/* Sky */}
      <defs>
        <linearGradient id="sky" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0%" stopColor="#c8ddf0" />
          <stop offset="100%" stopColor="#e8ddd0" />
        </linearGradient>
      </defs>
      <rect width="400" height="260" fill="url(#sky)" />

      {/* Clouds */}
      <ellipse cx="80" cy="50" rx="40" ry="14" fill="white" opacity="0.45" />
      <ellipse cx="100" cy="46" rx="28" ry="10" fill="white" opacity="0.35" />
      <ellipse cx="300" cy="35" rx="35" ry="11" fill="white" opacity="0.3" />
      <ellipse cx="320" cy="32" rx="20" ry="8" fill="white" opacity="0.25" />
      <ellipse cx="190" cy="65" rx="22" ry="8" fill="white" opacity="0.2" />

      {/* Far mountains */}
      <path d="M0 180 L60 130 L100 155 L160 110 L200 145 L240 120 L300 150 L360 125 L400 160 L400 260 L0 260Z" fill="#d4c4ae" opacity="0.4" />

      {/* Monument left */}
      <path d="M70 190 L80 120 L86 118 L92 120 L102 190Z" fill="#c4a882" opacity="0.5" />
      <path d="M78 190 L83 140 L87 138 L91 140 L96 190Z" fill="#b89870" opacity="0.45" />

      {/* Monument right */}
      <path d="M310 195 L322 105 L330 100 L338 105 L350 195Z" fill="#c4a882" opacity="0.45" />
      <path d="M316 195 L325 130 L330 127 L335 130 L344 195Z" fill="#b89870" opacity="0.4" />

      {/* Small butte */}
      <path d="M210 200 L220 165 L240 160 L260 165 L270 200Z" fill="#cbb898" opacity="0.35" />

      {/* Desert floor */}
      <path d="M0 200 Q100 190 200 195 Q300 200 400 192 L400 260 L0 260Z" fill="#ddd0bc" opacity="0.5" />
      <path d="M0 215 Q80 208 180 212 Q280 216 400 210 L400 260 L0 260Z" fill="#d8cab4" opacity="0.4" />

      {/* Cacti */}
      {/* Left cactus */}
      <rect x="48" y="195" width="4" height="20" rx="2" fill="#9aac82" opacity="0.4" />
      <rect x="44" y="200" width="4" height="8" rx="2" fill="#9aac82" opacity="0.35" transform="rotate(-30 46 204)" />
      <rect x="52" y="198" width="3" height="7" rx="1.5" fill="#9aac82" opacity="0.35" transform="rotate(25 53 201)" />

      {/* Right cactus */}
      <rect x="355" y="198" width="3.5" height="16" rx="1.75" fill="#9aac82" opacity="0.35" />
      <rect x="351" y="202" width="3" height="6" rx="1.5" fill="#9aac82" opacity="0.3" transform="rotate(-25 352 205)" />

      {/* Tumbleweed hint */}
      <circle cx="145" cy="218" r="5" fill="none" stroke="#b8a888" strokeWidth="0.8" opacity="0.25" />
      <circle cx="147" cy="217" r="3" fill="none" stroke="#b8a888" strokeWidth="0.6" opacity="0.2" />
    </svg>
  );
}
