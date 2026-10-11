import { PreferenceGroup, EmptyState } from '@dazzlabs/dazzui'
import { createFileRoute } from '@tanstack/react-router'
import { FileSearchIcon } from 'lucide-react'

import { PageBody } from '#/components/page-body'
import { PageHeader } from '#/components/page-header'
import { useI18n } from '#/lib/i18n'

export const Route = createFileRoute(
  '/p/$projectId/_shell/artifacts/$artifactId',
)({
  component: ArtifactDetail,
})

function ArtifactDetail() {
  const { projectId, artifactId } = Route.useParams()
  const { t } = useI18n()

  return (
    <>
      <PageHeader
        crumbs={[
          {
            label: t('Artifacts'),
            link: { to: '/p/$projectId/artifacts', params: { projectId } },
          },
          { label: artifactId },
        ]}
      />
      <PageBody>
        <PreferenceGroup
          title={<span role="heading" aria-level={1}>{`${artifactId}`}</span>}
          description={t(
            'Contents, signing and size breakdown, plus the publish targets it can go to.',
          )}
        />
        <EmptyState
          title={
            <>
              <FileSearchIcon aria-hidden />
              <span role="heading" aria-level={2}>
                {t('Package analysis')}
              </span>
              <p>
                {t(
                  'The output of `fastforge analyze` — manifest, permissions, signing and size — belongs on this page.',
                )}
              </p>
            </>
          }
        />
      </PageBody>
    </>
  )
}
