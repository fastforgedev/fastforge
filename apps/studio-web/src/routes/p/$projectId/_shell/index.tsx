import {
  PreferenceGroup,
  Card,
  PreferenceSection,
  Badge,
  Button,
  EmptyState,
  PreferenceRow,
} from '@dazzlabs/dazzui'
import { RouterLink } from '#/components/router-link'
import { createFileRoute } from '@tanstack/react-router'
import {
  ActivityIcon,
  PackageIcon,
  PlayIcon,
  RocketIcon,
  StoreIcon,
} from 'lucide-react'

import { PageBody } from '#/components/page-body'
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
        <PreferenceGroup
          title={
            <span role="heading" aria-level={1}>
              {t('Overview')}
            </span>
          }
          description={t(
            'What this project has shipped, and what is on the way.',
          )}
          action={
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
          <Card>
            <PreferenceSection label={t('Latest release')}>
              <div className="space-y-3">
                {[
                  { channel: 'App Store', status: 'Published' },
                  { channel: 'Google Play', status: 'Published' },
                  { channel: 'Firebase', status: 'In review' },
                ].map((row) => (
                  <PreferenceRow
                    key={row.channel}
                    title="v2.3.0+45"
                    subtitle={row.channel}
                  >
                    <Badge
                      tint={row.status === 'Published' ? 'success' : 'warning'}
                    >
                      {t(row.status)}
                    </Badge>
                  </PreferenceRow>
                ))}
              </div>
              <RouterLink
                className="mt-4 inline-flex"
                to="/p/$projectId/releases"
                params={{ projectId }}
              >
                {t('All releases')}
              </RouterLink>
            </PreferenceSection>
          </Card>

          <Card>
            <PreferenceSection label={t('Recent runs')}>
              <EmptyState
                title={
                  <>
                    {t('No runs yet')}
                    <p>{t('Trigger a workflow to see it appear here.')}</p>
                  </>
                }
              />
              <RouterLink
                className="mt-4 inline-flex"
                to="/p/$projectId/runs"
                params={{ projectId }}
              >
                {t('All runs')}
              </RouterLink>
            </PreferenceSection>
          </Card>
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
    <Card>
      <div className="flex items-center gap-2 text-muted-foreground">
        <Icon className="size-4" />
        <span className="text-xs font-medium">{label}</span>
      </div>
      <p className="mt-2 text-3xl font-bold">{value}</p>
    </Card>
  )
}
