import { RouterLink } from '#/components/router-link'
import { createFileRoute } from '@tanstack/react-router'
import { Button } from '@dazzlabs/dazzui'
import {
  ActivityIcon,
  PackageIcon,
  PlayIcon,
  RocketIcon,
  StoreIcon,
} from 'lucide-react'

import { PageBody, PageTitle, Panel } from '#/components/page-body'
import { PageHeader } from '#/components/page-header'
import { useI18n } from '#/lib/i18n'

import type { LucideIcon } from 'lucide-react'

export const Route = createFileRoute('/p/$projectId/_shell/')({
  component: Overview,
})

function Overview() {
  const { projectId } = Route.useParams()
  const { t } = useI18n()

  return (
    <>
      <PageHeader crumbs={[{ label: t('Overview') }]} />
      <PageBody>
        <PageTitle
          title={t('Overview')}
          description={t(
            'What this project has shipped, and what is on the way.',
          )}
          actions={
            <Button variant="filled">
              <PlayIcon data-icon="inline-start" />
              {t('Run workflow')}
            </Button>
          }
        />

        <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
          <StatCard icon={ActivityIcon} label={t('Runs this week')} value="3" />
          <StatCard icon={PackageIcon} label={t('Artifacts')} value="24" />
          <StatCard icon={RocketIcon} label={t('Releases')} value="12" />
          <StatCard icon={StoreIcon} label={t('Connected stores')} value="2" />
        </div>

        <div className="grid gap-4 md:grid-cols-2">
          <Panel title={t('Latest release')}>
            <div className="space-y-3">
              {[
                { channel: 'App Store', status: 'Published' },
                { channel: 'Google Play', status: 'Published' },
                { channel: 'Firebase', status: 'In review' },
              ].map((row) => (
                <div
                  key={row.channel}
                  className="flex items-center justify-between rounded-lg border p-3"
                >
                  <div>
                    <p className="text-sm font-medium">v2.3.0+45</p>
                    <p className="text-xs text-muted-foreground">
                      {row.channel}
                    </p>
                  </div>
                  <span className="rounded-full border px-2.5 py-0.5 text-xs font-medium">
                    {t(row.status)}
                  </span>
                </div>
              ))}
            </div>
            <RouterLink
              className="mt-4 inline-flex"
              to="/p/$projectId/releases"
              params={{ projectId }}
            >
              {t('All releases')}
            </RouterLink>
          </Panel>

          <Panel title={t('Recent runs')}>
            <div className="flex h-40 flex-col items-center justify-center text-muted-foreground">
              <p className="text-sm">{t('No runs yet')}</p>
              <p className="mt-1 text-xs">
                {t('Trigger a workflow to see it appear here.')}
              </p>
            </div>
            <RouterLink
              className="mt-4 inline-flex"
              to="/p/$projectId/runs"
              params={{ projectId }}
            >
              {t('All runs')}
            </RouterLink>
          </Panel>
        </div>
      </PageBody>
    </>
  )
}

function StatCard({
  icon: Icon,
  label,
  value,
}: {
  icon: LucideIcon
  label: string
  value: string
}) {
  return (
    <div className="rounded-xl border bg-card p-6">
      <div className="flex items-center gap-2 text-muted-foreground">
        <Icon className="size-4" />
        <span className="text-xs font-medium">{label}</span>
      </div>
      <p className="mt-2 text-3xl font-bold">{value}</p>
    </div>
  )
}
