import { PreferenceGroup, EmptyState } from '@dazzlabs/dazzui'
import { createFileRoute } from '@tanstack/react-router'
import { CloudUploadIcon } from 'lucide-react'

import { PageBody } from '#/components/page-body'
import { useI18n } from '#/lib/i18n'

export const Route = createFileRoute(
  '/p/$projectId/_shell/settings/publishers/$publisherId',
)({
  component: PublisherDetail,
})

function PublisherDetail() {
  const { publisherId } = Route.useParams()
  const { t } = useI18n()

  return (
    <>
      <PageBody>
        <PreferenceGroup
          title={
            <span role="heading" aria-level={1}>
              {publisherId}
            </span>
          }
          description={t(
            'Credentials and target options for this publisher, plus what has been uploaded to it.',
          )}
        />
        <EmptyState
          title={
            <>
              <CloudUploadIcon aria-hidden />
              <span role="heading" aria-level={2}>
                {t('Publisher target')}
              </span>
              <p>
                {t(
                  'Target options, the credential it authenticates with, and its upload history.',
                )}
              </p>
            </>
          }
        />
      </PageBody>
    </>
  )
}
