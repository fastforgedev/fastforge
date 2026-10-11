import { RouterLink } from '#/components/router-link'
import {
  Badge,
  PillTabs,
  PreferenceGroup,
  Table,
  TableRow,
  TableCell,
} from '@dazzlabs/dazzui'
import { createFileRoute } from '@tanstack/react-router'

import { PageBody } from '#/components/page-body'
import { PageHeader } from '#/components/page-header'
import { useI18n } from '#/lib/i18n'

export const Route = createFileRoute('/p/$projectId/_shell/runs/')({
  component: Runs,
})

/**
 * One timeline for every kind of execution. `build`, `package`, `publish` and
 * `workflow run` all land here rather than in separate per-stage histories.
 */
const runs = [
  {
    id: '1042',
    kind: 'workflow',
    label: 'Release',
    target: 'ios · ipa',
    status: 'Succeeded',
    when: '12m ago',
  },
  {
    id: '1041',
    kind: 'package',
    label: 'Package',
    target: 'android · aab',
    status: 'Succeeded',
    when: '1h ago',
  },
  {
    id: '1040',
    kind: 'publish',
    label: 'Publish',
    target: 'firebase',
    status: 'Failed',
    when: '3h ago',
  },
]

const kinds = ['All', 'Build', 'Package', 'Publish', 'Workflow']

function Runs() {
  const { projectId } = Route.useParams()
  const { t } = useI18n()

  return (
    <>
      <PageHeader crumbs={[{ label: t('Runs') }]} />
      <PageBody>
        <PreferenceGroup
          title={
            <span role="heading" aria-level={1}>
              {t('Runs')}
            </span>
          }
          description={t(
            'Every build, package, publish and workflow execution, newest first.',
          )}
        />

        <PillTabs
          items={kinds.map((kind) => ({ value: kind, label: t(kind) }))}
          defaultValue="All"
        />
        <Table>
          {runs.map((run) => (
            <TableRow key={run.id}>
              <TableCell className="min-w-0 flex-1">
                <RouterLink
                  to="/p/$projectId/runs/$runId"
                  params={{ projectId, runId: run.id }}
                >
                  {t(run.label)} #{run.id}
                </RouterLink>
                <p>
                  {run.kind} · {run.target}
                </p>
              </TableCell>
              <TableCell>
                <Badge tint={run.status === 'Failed' ? 'danger' : 'success'}>
                  {t(run.status)}
                </Badge>
              </TableCell>
              <TableCell align="end" className="hidden sm:block">
                {t(run.when)}
              </TableCell>
            </TableRow>
          ))}
        </Table>
      </PageBody>
    </>
  )
}
