import { RouterLink } from '#/components/router-link'
import {
  PreferenceGroup,
  Table,
  TableRow,
  TableCell,
  Code,
  Button,
} from '@dazzlabs/dazzui'
import { createFileRoute } from '@tanstack/react-router'
import { PlayIcon, PlusIcon } from 'lucide-react'

import { PageBody } from '#/components/page-body'
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
        <PreferenceGroup
          title={
            <span role="heading" aria-level={1}>
              {t('Workflows')}
            </span>
          }
          description={t(
            'The YAML under .fastforge/workflows. These are definitions — each execution shows up under Runs.',
          )}
          action={
            <Button size="small" variant="filled">
              <PlusIcon data-icon="inline-start" />
              {t('New workflow')}
            </Button>
          }
        />

        <Table>
          {workflows.map((workflow) => (
            <TableRow key={workflow.id}>
              <TableCell className="min-w-0 flex-1">
                <RouterLink
                  to="/p/$projectId/workflows/$workflowId"
                  params={{ projectId, workflowId: workflow.id }}
                  className="min-w-0 flex-1"
                >
                  <p className="font-medium">{t(workflow.name)}</p>
                  <Code>.fastforge/workflows/{workflow.id}</Code>
                </RouterLink>
              </TableCell>
              <TableCell>
                <span className="hidden shrink-0 text-xs text-muted-foreground sm:block">
                  {workflow.trigger}
                </span>
                <Button variant="outlined" size="small" className="shrink-0">
                  <PlayIcon data-icon="inline-start" />
                  {t('Run')}
                </Button>
              </TableCell>
            </TableRow>
          ))}
        </Table>
      </PageBody>
    </>
  )
}
