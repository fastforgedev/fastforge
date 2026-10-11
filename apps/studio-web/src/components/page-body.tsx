import type { ReactNode } from 'react'

/** Shared page layout; controls and surfaces come directly from DazzUI. */
export function PageBody({ children }: { children: ReactNode }) {
  return <div className="flex flex-1 flex-col gap-6 p-6">{children}</div>
}
