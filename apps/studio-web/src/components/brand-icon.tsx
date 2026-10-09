import { cx } from '@dazzlabs/dazzui'

export function BrandIcon({ className }: { className?: string }) {
  return (
    <svg viewBox="0 0 128 128" className={cx(className)} aria-hidden="true">
      <defs>
        <linearGradient id="ff-gradient" x1="0" y1="0" x2="1" y2="1">
          <stop stopColor="#1E40B0" />
          <stop offset=".35" stopColor="#2563EB" />
          <stop offset=".7" stopColor="#1688F5" />
          <stop offset="1" stopColor="#22D3EE" />
        </linearGradient>
        <symbol id="ff-mark" viewBox="0 0 128 128">
          <path d="M19,23 L37,23 L75,64 L37,105 L19,105 L41,64 Z" />
          <path d="M66,23 L84,23 L122,64 L84,105 L66,105 L88,64 Z" />
        </symbol>
      </defs>
      <use href="#ff-mark" fill="url(#ff-gradient)" />
    </svg>
  )
}
