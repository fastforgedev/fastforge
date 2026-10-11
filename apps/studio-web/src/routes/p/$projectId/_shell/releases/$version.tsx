import { PreferenceGroup, EmptyState } from '@dazzlabs/dazzui'
import { createFileRoute } from '@tanstack/react-router'
import { RocketIcon } from 'lucide-react'

import { PageBody } from '#/components/page-body'
import { PageHeader } from '#/components/page-header'
import { useI18n } from '#/lib/i18n'

export const Route = createFileRoute('/p/$projectId/_shell/releases/$version')({
  component: ReleaseDetail,
})

function ReleaseDetail() {
  const { projectId, version } = Route.useParams()
  const { t } = useI18n()

  return (
    <>
      <PageHeader
        crumbs={[
          {
            label: t('Releases'),
            link: { to: '/p/$projectId/releases', params: { projectId } },
          },
          { label: version },
        ]}
      />
      <PageBody>
        <PreferenceGroup
          title={<span role="heading" aria-level={1}>{`${version}`}</span>}
          description={t(
            'Artifacts, changelog and per-channel rollout state for this version.',
          )}
        />
        <EmptyState
          title={
            <>
              <RocketIcon aria-hidden />
              <span role="heading" aria-level={2}>
                {t('Release detail')}
              </span>
              <p>
                {t(
                  'Channel-by-channel status, the artifacts attached to this version and its release notes.',
                )}
              </p>
            </>
          }
        />
      </PageBody>
    </>
  )
}
