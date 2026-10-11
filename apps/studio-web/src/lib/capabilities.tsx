import { Callout, Code, EmptyState } from '@dazzlabs/dazzui'
import * as React from 'react'
import { api } from 'studio-api-client'
import { useI18n } from '#/lib/i18n'

import type { Capabilities, StudioMode } from 'studio-api-client'

export type { Capabilities, StudioMode }

/**
 * Studio runs in two shapes against this one client:
 *
 * - `local` — a companion to the CLI on a developer machine. It reads and
 *   writes `.fastforge/` directly.
 * - `cloud` — the hosted service.
 *
 * The server reports which, and what it can do, through `GET /v1/capabilities`.
 * Surfaces the current mode cannot serve are hidden rather than disabled, so
 * navigation never contains a dead link.
 *
 * This is fetched here rather than in the root route's loader on purpose: the
 * client ships as a prerendered SPA shell, and a root loader's result is baked
 * into that shell at build time — when there is no server to ask.
 */
const CapabilitiesContext = React.createContext<Capabilities | null>(null)

type State =
  | { status: 'loading' }
  | { status: 'ready'; capabilities: Capabilities }
  | { status: 'failed'; error: Error }

export function CapabilitiesProvider({
  children,
}: {
  children: React.ReactNode
}) {
  const [state, setState] = React.useState<State>({ status: 'loading' })

  React.useEffect(() => {
    let current = true
    api
      .getCapabilities()
      .then(
        (capabilities) =>
          current && setState({ status: 'ready', capabilities }),
      )
      .catch((error: Error) => current && setState({ status: 'failed', error }))
    return () => {
      current = false
    }
  }, [])

  if (state.status === 'failed') {
    return <ServerUnreachable error={state.error} />
  }
  if (state.status === 'loading') {
    // Route loaders are already in flight by now — the router runs them
    // independently of what renders — so this only covers the gap before the
    // first paint, not a waterfall.
    return null
  }

  return (
    <CapabilitiesContext.Provider value={state.capabilities}>
      {children}
    </CapabilitiesContext.Provider>
  )
}

export function useCapabilities(): Capabilities {
  const capabilities = React.useContext(CapabilitiesContext)
  if (!capabilities) {
    // Reaching here means a component rendered outside the provider, which is a
    // wiring mistake rather than a state a user can get into.
    throw new Error('useCapabilities must be used inside CapabilitiesProvider')
  }
  return capabilities
}

/**
 * Shown when the API cannot be reached at all. In local mode that almost always
 * means the server is not running, so the page says how to start it rather than
 * printing a fetch error on its own.
 */
function ServerUnreachable({ error }: { error: Error }) {
  const { t } = useI18n()
  return (
    <main className="flex min-h-svh flex-col items-center justify-center gap-4 p-6 text-center">
      <EmptyState
        title={
          <>
            {t('Can’t reach the Studio server')}
            <p>
              {t('Start it with')} <Code>fastforge-studio serve</Code>{' '}
              {t('and reload.')}
            </p>
          </>
        }
      />
      <Callout tint="danger" role="alert">
        {error.message}
      </Callout>
    </main>
  )
}
