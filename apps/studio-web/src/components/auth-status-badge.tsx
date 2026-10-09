import { cx } from '@dazzlabs/dazzui'

import type { AuthStatus } from 'studio-api-client'
import { useI18n } from '#/lib/i18n'

/**
 * Whether a store's credentials resolve.
 *
 * Studio never reads credential values — only whether each field resolves and
 * from where — so this reports a state, and the detail below it names the
 * fields rather than showing anything secret.
 */
export function AuthStatusBadge({ status }: { status: AuthStatus }) {
  const { t } = useI18n()
  const { label, tone } = describe(status, t)

  return (
    <span
      className={cx(
        'shrink-0 rounded-full border px-2.5 py-0.5 text-xs font-medium',
        tone,
      )}
    >
      {label}
    </span>
  )
}

function describe(
  status: AuthStatus,
  t: (key: string) => string,
): { label: string; tone: string } {
  switch (status.state) {
    case 'ready':
      return { label: t('Ready'), tone: '' }
    case 'incomplete':
      return {
        label: t('Credentials missing'),
        tone: 'border-amber-500/40 text-amber-600 dark:text-amber-400',
      }
    case 'notConfigured':
      return { label: t('Not configured'), tone: 'text-muted-foreground' }
  }
}

/**
 * The per-field breakdown: where each credential came from, or which
 * environment variables would supply the ones that are missing.
 */
export function AuthFieldList({ status }: { status: AuthStatus }) {
  const { t } = useI18n()
  if (status.state === 'notConfigured') {
    return null
  }

  const fields =
    status.state === 'ready' ? (status.sources ?? []) : status.missing

  return (
    <dl className="mt-3 grid gap-1 text-xs">
      {fields.map((field) => (
        <div key={field.field} className="flex gap-2">
          <dt className="min-w-36 font-mono text-muted-foreground">
            {field.field}
          </dt>
          <dd className="font-mono">
            {status.state === 'ready' ? (
              field.source
            ) : (
              <span className="text-amber-600 dark:text-amber-400">
                {field.candidates?.length
                  ? t('set {names}', { names: field.candidates.join(' / ') })
                  : t('unset')}
              </span>
            )}
          </dd>
        </div>
      ))}
    </dl>
  )
}
