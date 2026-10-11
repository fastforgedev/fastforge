import { PreferenceGroup, EmptyState } from '@dazzlabs/dazzui'
import { createFileRoute } from '@tanstack/react-router'
import { FileCodeIcon } from 'lucide-react'

import { PageBody } from '#/components/page-body'
import { useI18n } from '#/lib/i18n'

export const Route = createFileRoute(
  '/p/$projectId/_shell/settings/configuration',
)({
  component: SettingsConfiguration,
})

function SettingsConfiguration() {
  const { t } = useI18n()
  return (
    <>
      <PageBody>
        <PreferenceGroup
          title={
            <span role="heading" aria-level={1}>
              {t('Configuration')}
            </span>
          }
          description={t(
            'The raw .fastforge/config.yaml. Every form on the other pages is a view onto this file.',
          )}
        />
        <EmptyState
          title={
            <>
              <FileCodeIcon aria-hidden />
              <span role="heading" aria-level={2}>
                {'config.yaml'}
              </span>
              <p>
                {t(
                  'An editor for the file itself, so anything the forms do not cover stays reachable.',
                )}
              </p>
            </>
          }
        />
      </PageBody>
    </>
  )
}
