import { Link, createFileRoute } from '@tanstack/react-router'
import { Button } from '@dazzlabs/dazzui'
import { PlayIcon, PlusIcon } from 'lucide-react'

import { PageBody, PageTitle } from '#/components/page-body'
import { PageHeader } from '#/components/page-header'
import { useI18n } from '#/lib/i18n'

export const Route = createFileRoute('/p/$projectId/_shell/workflows/')({
  component: Workflows,
})

// Mirrors `.fastforge/workflows/*.yml`.
const workflows = [
  {
    id: 'android.yml',
    name: 'Android package',
    trigger: 'workflow_dispatch',
    jobs: 1,
  },
  {
    id: 'release.yml',
    name: 'Release',
    trigger: 'workflow_dispatch · tag',
    jobs: 3,
  },
]

function Workflows() {
  const { projectId } = Route.useParams()
  const { t } = useI18n()

  return (
    <>
      <PageHeader crumbs={[{ label: t('Workflows') }]} />
      <PageBody>
        <PageTitle
          title={t('Workflows')}
          description={t(
            'The YAML under .fastforge/workflows. These are definitions — each execution shows up under Runs.',
          )}
          actions={
            <Button size="small" variant="filled">
              <PlusIcon data-icon="inline-start" />
              {t('New workflow')}
            </Button>
          }
        />

        <ul className="divide-y rounded-xl border bg-card">
          {workflows.map((workflow) => (
            <li key={workflow.id} className="flex items-center gap-4 p-4">
              <Link
                to="/p/$projectId/workflows/$workflowId"
                params={{ projectId, workflowId: workflow.id }}
                className="min-w-0 flex-1"
              >
                <p className="font-medium">{t(workflow.name)}</p>
                <p className="truncate font-mono text-xs text-muted-foreground">
                  .fastforge/workflows/{workflow.id}
                </p>
              </Link>
              <span className="hidden shrink-0 text-xs text-muted-foreground sm:block">
                {workflow.trigger}
              </span>
              <Button variant="outlined" size="small" className="shrink-0">
                <PlayIcon data-icon="inline-start" />
                {t('Run')}
              </Button>
            </li>
          ))}
        </ul>
      </PageBody>
    </>
  )
}
