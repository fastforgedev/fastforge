'use client'

import { Link } from '@tanstack/react-router'
import { useId, useState } from 'react'
import {
  SidebarGroup,
  SidebarGroupLabel,
  SidebarMenu,
  SidebarMenuAction,
  SidebarMenuBadge,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarMenuSub,
  SidebarMenuSubButton,
  SidebarMenuSubItem,
} from '@dazzlabs/dazzui'
import { ChevronRightIcon } from 'lucide-react'

import type { NavGroup, NavItem } from '#/lib/navigation'

// TanStack links supply aria-current, which DazzUI uses to draw selection.
export function NavMain({ groups }: { groups: Array<NavGroup> }) {
  return (
    <>
      {groups.map((group, index) => (
        <SidebarGroup key={group.label ?? index}>
          {group.label ? (
            <SidebarGroupLabel>{group.label}</SidebarGroupLabel>
          ) : null}
          <SidebarMenu>
            {group.items.map((item) =>
              item.items?.length ? (
                <CollapsibleNavItem key={item.title} item={item} />
              ) : (
                <SidebarMenuItem key={item.title}>
                  <NavButton item={item} />
                  {item.badge ? (
                    <SidebarMenuBadge>{item.badge}</SidebarMenuBadge>
                  ) : null}
                </SidebarMenuItem>
              ),
            )}
          </SidebarMenu>
        </SidebarGroup>
      ))}
    </>
  )
}

function NavButton({ item }: { item: NavItem }) {
  return (
    <SidebarMenuButton
      tooltip={item.title}
      icon={<item.icon />}
      render={<Link {...item.link} />}
    >
      <span>{item.title}</span>
    </SidebarMenuButton>
  )
}

function CollapsibleNavItem({ item }: { item: NavItem }) {
  const [open, setOpen] = useState(true)
  const panelId = useId()
  return (
    <SidebarMenuItem>
      <NavButton item={item} />
      <SidebarMenuAction
        label={`Toggle ${item.title}`}
        aria-expanded={open}
        aria-controls={panelId}
        onClick={() => setOpen(!open)}
        className="aria-expanded:rotate-90"
      >
        <ChevronRightIcon />
      </SidebarMenuAction>
      <div id={panelId} hidden={!open}>
        <SidebarMenuSub>
          {item.items?.map((subItem) => (
            <SidebarMenuSubItem key={subItem.title}>
              <SidebarMenuSubButton render={<Link {...subItem.link} />}>
                <span>{subItem.title}</span>
              </SidebarMenuSubButton>
            </SidebarMenuSubItem>
          ))}
        </SidebarMenuSub>
      </div>
    </SidebarMenuItem>
  )
}
