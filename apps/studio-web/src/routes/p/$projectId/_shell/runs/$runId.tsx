import { PreferenceGroup, EmptyState } from '@dazzlabs/dazzui'
import { createFileRoute } from '@tanstack/react-router'
import { ScrollTextIcon } from 'lucide-react'

import { PageBody } from '#/components/page-body'
import { PageHeader } from '#/components/page-header'
import { useI18n } from '#/lib/i18n'

export const Route = createFileRoute('/p/$projectId/_shell/runs/$runId')({
  component: RunDetail,
})

function RunDetail() {
  const { projectId, runId } = Route.useParams()
  const { t } = useI18n()

  return (
    <>
      <PageHeader
        crumbs={[
          {
            label: t('Runs'),
            link: { to: '/p/$projectId/runs', params: { projectId } },
          },
          { label: runId },
        ]}
      />
      <PageBody>
        <PreferenceGroup
          title={
            <span role="heading" aria-level={1}>
              {t('Run {id}', { id: runId })}
            </span>
          }
          description={t(
            'Step timeline, logs and the artifacts this run produced.',
          )}
        />
        <EmptyState
          title={
            <>
              <ScrollTextIcon aria-hidden />
              <span role="heading" aria-level={2}>
                {t('Run log')}
              </span>
              <p>
                {t(
                  'Streaming step output and per-step timings will render here.',
                )}
              </p>
            </>
          }
        />
      </PageBody>
    </>
  )
}
