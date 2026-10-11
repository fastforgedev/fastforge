import { PreferenceGroup, EmptyState } from '@dazzlabs/dazzui'
import { createFileRoute } from '@tanstack/react-router'
import { PackageIcon } from 'lucide-react'

import { PageBody } from '#/components/page-body'
import { PageHeader } from '#/components/page-header'
import { useI18n } from '#/lib/i18n'

export const Route = createFileRoute('/p/$projectId/_shell/artifacts/')({
  component: Artifacts,
})

function Artifacts() {
  const { t } = useI18n()
  return (
    <>
      <PageHeader crumbs={[{ label: t('Artifacts') }]} />
      <PageBody>
        <PreferenceGroup
          title={
            <span role="heading" aria-level={1}>
              {t('Artifacts')}
            </span>
          }
          description={t(
            'Every package the packagers produced — APK, AAB, IPA, DMG, PKG, ZIP.',
          )}
        />
        <EmptyState
          title={
            <>
              <PackageIcon aria-hidden />
              <span role="heading" aria-level={2}>
                {t('No artifacts yet')}
              </span>
              <p>
                {t(
                  'Packages appear here once a run finishes. Each one can be analyzed and published from its detail page.',
                )}
              </p>
            </>
          }
        />
      </PageBody>
    </>
  )
}
