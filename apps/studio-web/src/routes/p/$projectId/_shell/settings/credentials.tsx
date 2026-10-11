import { PreferenceGroup, EmptyState } from '@dazzlabs/dazzui'
import { createFileRoute } from '@tanstack/react-router'
import { KeyIcon } from 'lucide-react'

import { PageBody } from '#/components/page-body'
import { useI18n } from '#/lib/i18n'

export const Route = createFileRoute(
  '/p/$projectId/_shell/settings/credentials',
)({
  component: SettingsCredentials,
})

function SettingsCredentials() {
  const { t } = useI18n()
  return (
    <>
      <PageBody>
        <PreferenceGroup
          title={
            <span role="heading" aria-level={1}>
              {t('Credentials')}
            </span>
          }
          description={t(
            'Signing keys and the API credentials stores and publishers authenticate with.',
          )}
        />
        <EmptyState
          title={
            <>
              <KeyIcon aria-hidden />
              <span role="heading" aria-level={2}>
                {t('No credentials stored')}
              </span>
              <p>
                {t(
                  'Keystores, App Store Connect keys and service accounts are referenced by name from the configuration.',
                )}
              </p>
            </>
          }
        />
      </PageBody>
    </>
  )
}
