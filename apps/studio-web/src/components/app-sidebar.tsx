'use client'

import { StudioSidebar } from '#/components/studio-sidebar'

import { Link } from '@tanstack/react-router'
import {
  SidebarContent,
  SidebarFooter,
  SidebarHeader,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarRail,
  SidebarSeparator,
} from '@dazzlabs/dazzui'
import { CloudIcon, HardDriveIcon, Settings2Icon } from 'lucide-react'

import { NavMain } from '#/components/nav-main'
import { NavDoc } from '#/components/nav-doc'
import { ProjectSwitcher } from '#/components/project-switcher'
import { useCapabilities } from '#/lib/capabilities'
import { projectNavGroups } from '#/lib/navigation'
import { useI18n } from '#/lib/i18n'

import type * as React from 'react'
import type { ProjectSummary, StoreConnection } from 'studio-api-client'
import type { Project } from '#/lib/workspace'

export function AppSidebar({
  project,
  stores,
  projects,
  ...props
}: {
  project: Project
  stores: Array<StoreConnection>
  projects: Array<ProjectSummary>
} & React.ComponentProps<typeof StudioSidebar>) {
  const capabilities = useCapabilities()
  const { t } = useI18n()

  return (
    <StudioSidebar variant="inset" collapsible="icon" {...props}>
      <SidebarHeader>
        <ProjectSwitcher project={project} projects={projects} />
      </SidebarHeader>

      <SidebarContent>
        <NavMain
          groups={projectNavGroups(project.id, capabilities, stores, t)}
        />
      </SidebarContent>

      <SidebarFooter>
        <ModeIndicator project={project} />
        <SidebarMenu>
          <SidebarMenuItem>
            <SidebarMenuButton
              render={
                <Link
                  to="/p/$projectId/settings"
                  params={{ projectId: project.id }}
                />
              }
              icon={<Settings2Icon />}
              tooltip={t('Project Settings')}
            >
              <span>{t('Project Settings')}</span>
            </SidebarMenuButton>
          </SidebarMenuItem>
        </SidebarMenu>
        <SidebarSeparator className="mx-0" />
        <NavDoc />
      </SidebarFooter>

      <SidebarRail />
    </StudioSidebar>
  )
}

/** Where builds actually run — the one thing that differs between the two modes. */
function ModeIndicator({ project }: { project: Project }) {
  const { mode } = useCapabilities()
  const { t } = useI18n()
  const Icon = mode === 'local' ? HardDriveIcon : CloudIcon

  return (
    <div className="flex items-center gap-2 px-2 py-1 text-xs text-muted-foreground studio-mode-indicator">
      <Icon className="size-3.5 shrink-0" />
      <span className="truncate">
        {mode === 'local'
          ? (project.path ?? t('This machine'))
          : t('Cloud runners')}
      </span>
    </div>
  )
}
