import { PreferenceGroup, EmptyState } from '@dazzlabs/dazzui'
import { createFileRoute } from '@tanstack/react-router'
import { RocketIcon } from 'lucide-react'

import { PageBody } from '#/components/page-body'
import { PageHeader } from '#/components/page-header'
import { useI18n } from '#/lib/i18n'

export const Route = createFileRoute('/p/$projectId/_shell/releases/')({
  component: Releases,
})

function Releases() {
  const { t } = useI18n()
  return (
    <>
      <PageHeader crumbs={[{ label: t('Releases') }]} />
      <PageBody>
        <PreferenceGroup
          title={
            <span role="heading" aria-level={1}>
              {t('Releases')}
            </span>
          }
          description={t(
            'One row per version, showing where that version currently stands on each channel.',
          )}
        />
        <EmptyState
          title={
            <>
              <RocketIcon aria-hidden />
              <span role="heading" aria-level={2}>
                {t('No releases yet')}
              </span>
              <p>
                {t(
                  'A release groups the artifacts of one version and tracks their status across every store and publisher.',
                )}
              </p>
            </>
          }
        />
      </PageBody>
    </>
  )
}
