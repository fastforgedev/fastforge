import { Sidebar } from '@dazzlabs/dazzui'
import { Outlet, createFileRoute, useRouterState } from '@tanstack/react-router'

import { PageHeader } from '#/components/page-header'
import { SettingsNav } from '#/components/settings-nav'
import { useI18n } from '#/lib/i18n'

import type { Crumb } from '#/components/page-header'

export const Route = createFileRoute('/p/$projectId/_shell/settings')({
  component: ProjectSettingsShell,
})

/** Second crumb by the path segment after `/settings`. */
const SECTION_LABELS: Record<string, string> = {
  '': 'General',
  build: 'Build & Package',
  environments: 'Environments',
  credentials: 'Credentials',
  publishers: 'Publishers',
  configuration: 'Configuration',
}

function settingsCrumbs(
  pathname: string,
  projectId: string,
  t: (key: string) => string,
): Array<Crumb> {
  const rest = pathname.split('/settings')[1]?.replace(/^\/+/, '') ?? ''
  const [section = '', detail] = rest.split('/')

  const crumbs: Array<Crumb> = [
    {
      label: t('Settings'),
      link: { to: '/p/$projectId/settings', params: { projectId } },
    },
  ]

  if (section === 'publishers' && detail) {
    crumbs.push(
      {
        label: t('Publishers'),
        link: {
          to: '/p/$projectId/settings/publishers',
          params: { projectId },
        },
      },
      { label: detail },
    )
  } else {
    crumbs.push({ label: t(SECTION_LABELS[section] ?? 'General') })
  }

  return crumbs
}

/**
 * Settings renders as a secondary sidebar inside the page rather than
 * replacing the project sidebar. The breadcrumb header spans the full content
 * width (it lives here, not in the pages), and below it the body splits into
 * the settings nav column and the page itself.
 */
function ProjectSettingsShell() {
  const { projectId } = Route.useParams()
  const { t } = useI18n()
  const pathname = useRouterState({ select: (s) => s.location.pathname })

  return (
    <>
      <PageHeader crumbs={settingsCrumbs(pathname, projectId, t)} />
      <div className="flex min-h-0 flex-1">
        <Sidebar collapsible="none" className="hidden! md:flex!">
          <SettingsNav projectId={projectId} />
        </Sidebar>
        <div className="flex min-w-0 flex-1 flex-col">
          <Outlet />
        </div>
      </div>
    </>
  )
}
