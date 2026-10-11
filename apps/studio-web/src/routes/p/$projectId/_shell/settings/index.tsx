import { PreferenceGroup, EmptyState } from '@dazzlabs/dazzui'
import { createFileRoute } from '@tanstack/react-router'
import { SlidersHorizontalIcon } from 'lucide-react'

import { PageBody } from '#/components/page-body'
import { useI18n } from '#/lib/i18n'

export const Route = createFileRoute('/p/$projectId/_shell/settings/')({
  component: SettingsGeneral,
})

function SettingsGeneral() {
  const { t } = useI18n()
  return (
    <>
      <PageBody>
        <PreferenceGroup
          title={
            <span role="heading" aria-level={1}>
              {t('General')}
            </span>
          }
          description={t(
            'Project name, identifiers and the location Fastforge reads its config from.',
          )}
        />
        <EmptyState
          title={
            <>
              <SlidersHorizontalIcon aria-hidden />
              <span role="heading" aria-level={2}>
                {t('General settings')}
              </span>
              <p>
                {t(
                  'Display name, application id and the path to this project on disk.',
                )}
              </p>
            </>
          }
        />
      </PageBody>
    </>
  )
}
