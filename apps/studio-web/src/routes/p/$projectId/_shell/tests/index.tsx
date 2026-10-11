import { PreferenceGroup, EmptyState } from '@dazzlabs/dazzui'
import { createFileRoute } from '@tanstack/react-router'
import { FlaskConicalIcon } from 'lucide-react'

import { PageBody } from '#/components/page-body'
import { PageHeader } from '#/components/page-header'
import { useI18n } from '#/lib/i18n'

export const Route = createFileRoute('/p/$projectId/_shell/tests/')({
  component: UnitTests,
})

function UnitTests() {
  const { t } = useI18n()
  return (
    <>
      <PageHeader crumbs={[{ label: t('Unit Tests') }]} />
      <PageBody>
        <PreferenceGroup
          title={
            <span role="heading" aria-level={1}>
              {t('Unit Tests')}
            </span>
          }
          description={t(
            'The test suites defined for this project and their latest results.',
          )}
        />
        <EmptyState
          title={
            <>
              <FlaskConicalIcon aria-hidden />
              <span role="heading" aria-level={2}>
                {t('No tests yet')}
              </span>
              <p>
                {t(
                  'Test suites appear here once a run finishes. Each one can be analyzed from its detail page.',
                )}
              </p>
            </>
          }
        />
      </PageBody>
    </>
  )
}
