import { PreferenceGroup, EmptyState } from '@dazzlabs/dazzui'
import { createFileRoute } from '@tanstack/react-router'
import { LayersIcon } from 'lucide-react'

import { PageBody } from '#/components/page-body'
import { useI18n } from '#/lib/i18n'

export const Route = createFileRoute(
  '/p/$projectId/_shell/settings/environments',
)({
  component: SettingsEnvironments,
})

function SettingsEnvironments() {
  const { t } = useI18n()
  return (
    <>
      <PageBody>
        <PreferenceGroup
          title={
            <span role="heading" aria-level={1}>
              {t('Environments')}
            </span>
          }
          description={t(
            'Flavors, build variables and the values that differ per environment.',
          )}
        />
        <EmptyState
          title={
            <>
              <LayersIcon aria-hidden />
              <span role="heading" aria-level={2}>
                {t('No environments configured')}
              </span>
              <p>
                {t(
                  'Define staging, production and any custom flavors, along with the build arguments each one passes.',
                )}
              </p>
            </>
          }
        />
      </PageBody>
    </>
  )
}
