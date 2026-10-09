import { Link } from '@tanstack/react-router'
import { Button } from '@dazzlabs/dazzui'
import { PlusIcon } from 'lucide-react'

import type { LinkComponentProps } from '@tanstack/react-router'
import type { Target } from '#/lib/targets'
import { useI18n } from '#/lib/i18n'

/**
 * The list that replaces per-integration sidebar entries. Connected targets are
 * links; the rest are an inline catalog, so adding a target never changes nav.
 */
export function TargetList({
  targets,
  detailLink,
}: {
  targets: Array<Target>
  detailLink: (target: Target) => LinkComponentProps
}) {
  const { t } = useI18n()
  const connected = targets.filter((target) => target.connected)
  const available = targets.filter((target) => !target.connected)

  return (
    <div className="space-y-6">
      <section className="space-y-2">
        <h2 className="text-sm font-medium text-muted-foreground">
          {t('Connected')}
        </h2>
        {connected.length === 0 ? (
          <p className="rounded-xl border border-dashed p-6 text-center text-sm text-muted-foreground">
            {t('Nothing connected yet.')}
          </p>
        ) : (
          <ul className="divide-y rounded-xl border bg-card">
            {connected.map((target) => (
              <li key={target.id}>
                <Link
                  {...detailLink(target)}
                  className="flex items-center gap-4 p-4 transition-colors hover:bg-muted/40"
                >
                  <div className="min-w-0 flex-1">
                    <p className="font-medium">{t(target.name)}</p>
                    <p className="truncate text-sm text-muted-foreground">
                      {target.detail ?? t(target.description)}
                    </p>
                  </div>
                  <span className="shrink-0 rounded-full border px-2.5 py-0.5 text-xs font-medium">
                    {t('Connected')}
                  </span>
                </Link>
              </li>
            ))}
          </ul>
        )}
      </section>

      <section className="space-y-2">
        <h2 className="text-sm font-medium text-muted-foreground">
          {t('Available')}
        </h2>
        <ul className="grid gap-3 sm:grid-cols-2">
          {available.map((target) => (
            <li
              key={target.id}
              className="flex items-start gap-3 rounded-xl border p-4"
            >
              <div className="min-w-0 flex-1">
                <p className="font-medium">{t(target.name)}</p>
                <p className="text-sm text-muted-foreground">
                  {t(target.description)}
                </p>
              </div>
              <Button variant="outlined" size="small" className="shrink-0">
                <PlusIcon data-icon="inline-start" />
                {t('Connect')}
              </Button>
            </li>
          ))}
        </ul>
      </section>
    </div>
  )
}
