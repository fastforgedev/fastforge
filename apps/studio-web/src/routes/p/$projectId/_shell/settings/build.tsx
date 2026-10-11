import { PreferenceGroup, EmptyState } from '@dazzlabs/dazzui'
import { createFileRoute } from '@tanstack/react-router'
import { HammerIcon } from 'lucide-react'

import { PageBody } from '#/components/page-body'
import { PackagingForms } from '#/components/packaging-forms'
import { useCapabilities } from '#/lib/capabilities'
import { useI18n } from '#/lib/i18n'

export const Route = createFileRoute('/p/$projectId/_shell/settings/build')({
  component: SettingsBuild,
})

function SettingsBuild() {
  const { t } = useI18n()
  const { projectId } = Route.useParams()
  const { localFs } = useCapabilities()
  return (
    <>
      <PageBody>
        <PreferenceGroup
          title={
            <span role="heading" aria-level={1}>
              {t('Build & Package')}
            </span>
          }
          description={t(
            'Configure native packaging files for your application.',
          )}
        />
        {localFs ? (
          <PackagingForms key={projectId} projectId={projectId} />
        ) : (
          <EmptyState
            title={
              <>
                <HammerIcon aria-hidden />
                <span role="heading" aria-level={2}>
                  {t('Local packaging files')}
                </span>
                <p>
                  {t(
                    'Open this project in local Studio to edit its packaging files.',
                  )}
                </p>
              </>
            }
          />
        )}
      </PageBody>
    </>
  )
}
