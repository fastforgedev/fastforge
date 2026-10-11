import { useState } from 'react'
import { Link } from '@tanstack/react-router'
import {
  Divider,
  EmptyState,
  Popover,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  SearchField,
} from '@dazzlabs/dazzui'
import { CheckIcon, ChevronsUpDownIcon, LayoutGridIcon } from 'lucide-react'
import { BrandIcon } from '#/components/brand-icon'
import { useStudioSidebar } from '#/components/studio-sidebar'
import { useI18n } from '#/lib/i18n'
import type { ProjectSummary, Project } from 'studio-api-client'

export function ProjectSwitcher({
  project,
  projects,
}: {
  project: Project
  projects: Array<ProjectSummary>
}) {
  const { isMobile } = useStudioSidebar()
  const { t } = useI18n()
  const [query, setQuery] = useState('')
  const [open, setOpen] = useState(false)
  const matches = projects.filter((candidate) =>
    candidate.name.toLowerCase().includes(query.trim().toLowerCase()),
  )
  return (
    <SidebarMenu>
      <SidebarMenuItem>
        <Popover
          open={open}
          onOpenChange={(next) => {
            setOpen(next)
            setQuery('')
          }}
          title={t('Projects')}
          width="18rem"
          align="start"
          side={isMobile ? 'bottom' : 'right'}
          trigger={
            <SidebarMenuButton
              size="large"
              variant="normal"
              icon={<BrandIcon className="size-8!" />}
            >
              <div className="grid min-w-0 flex-1 text-left text-sm leading-tight">
                <span className="truncate font-medium">{project.name}</span>
                <span className="truncate text-xs text-muted-foreground">
                  {project.path ?? project.repo}
                </span>
              </div>
              <ChevronsUpDownIcon className="ml-auto size-4 shrink-0" />
            </SidebarMenuButton>
          }
        >
          <SearchField
            value={query}
            onValueChange={setQuery}
            aria-label={t('Search projects…')}
            placeholder={t('Search projects…')}
            size="small"
            className="w-full"
          />
          <nav
            aria-label={t('Projects')}
            className="my-2 max-h-72 overflow-auto"
          >
            <SidebarMenu>
              {matches.length === 0 ? (
                <SidebarMenuItem>
                  <EmptyState title={t('No matching project')} />
                </SidebarMenuItem>
              ) : (
                matches.map((candidate) => (
                  <SidebarMenuItem key={candidate.id}>
                    <SidebarMenuButton
                      variant="tinted"
                      active={candidate.id === project.id}
                      render={
                        <Link
                          to="/p/$projectId"
                          params={{ projectId: candidate.id }}
                          onClick={() => setOpen(false)}
                          aria-current={
                            candidate.id === project.id ? 'page' : undefined
                          }
                        />
                      }
                    >
                      <span className="flex-1 truncate">{candidate.name}</span>
                      {candidate.id === project.id ? (
                        <CheckIcon className="size-4 text-muted-foreground" />
                      ) : null}
                    </SidebarMenuButton>
                  </SidebarMenuItem>
                ))
              )}
            </SidebarMenu>
          </nav>
          <Divider />
          <SidebarMenu>
            <SidebarMenuItem>
              <SidebarMenuButton
                render={<Link to="/" onClick={() => setOpen(false)} />}
                icon={<LayoutGridIcon />}
                variant="plain"
              >
                <span>{t('All projects')}</span>
              </SidebarMenuButton>
            </SidebarMenuItem>
          </SidebarMenu>
        </Popover>
      </SidebarMenuItem>
    </SidebarMenu>
  )
}
