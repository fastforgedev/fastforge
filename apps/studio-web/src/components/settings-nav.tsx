'use client'

import { Link, useRouterState } from '@tanstack/react-router'
import {
  SidebarGroup,
  SidebarGroupLabel,
  SidebarMenu,
  SidebarMenuItem,
  SidebarMenuButton,
} from '@dazzlabs/dazzui'

import { settingsNavGroups } from '#/lib/navigation'
import { useI18n } from '#/lib/i18n'

/**
 * The settings sections as a secondary navigation column. The project sidebar
 * (primary) stays in place; this renders inside the settings pages.
 */
export function SettingsNav({ projectId }: { projectId: string }) {
  const { t } = useI18n()
  const groups = settingsNavGroups(projectId, t)
  const pathname = useRouterState({
    select: (state) => state.location.pathname,
  })

  return (
    <nav className="flex flex-col gap-5">
      {groups.map((group, index) => (
        <SidebarGroup key={group.label ?? index}>
          {group.label ? (
            <SidebarGroupLabel>{group.label}</SidebarGroupLabel>
          ) : null}
          <SidebarMenu>
            {group.items.map((item) => (
              <SidebarMenuItem key={item.title}>
                <SidebarMenuButton
                  size="small"
                  variant="tinted"
                  active={
                    pathname ===
                      item.link.to?.replace('$projectId', projectId) ||
                    (item.link.to?.endsWith('/publishers') &&
                      pathname.includes('/settings/publishers/'))
                  }
                  icon={<item.icon />}
                  render={
                    <Link {...item.link} activeOptions={{ exact: true }} />
                  }
                >
                  {item.title}
                </SidebarMenuButton>
              </SidebarMenuItem>
            ))}
          </SidebarMenu>
        </SidebarGroup>
      ))}
    </nav>
  )
}
