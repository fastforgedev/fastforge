import * as React from 'react'
import { Divider, SidebarTrigger } from '@dazzlabs/dazzui'
import { ChevronRightIcon } from 'lucide-react'
import { RouterLink } from '#/components/router-link'

import { LocaleMenu } from '#/components/locale-menu'
import { ThemeToggle } from '#/components/theme-toggle'

import type { LinkComponentProps } from '@tanstack/react-router'

export type Crumb = {
  label: string
  /** Omit to render the crumb as plain text — used for grouping segments that have no page. */
  link?: LinkComponentProps
}

export function PageHeader({
  crumbs,
  actions,
}: {
  crumbs: Array<Crumb>
  actions?: React.ReactNode
}) {
  return (
    <header className="flex h-16 shrink-0 items-center gap-2 border-b">
      <div className="flex min-w-0 flex-1 items-center gap-2 px-4">
        <SidebarTrigger className="-ml-1" />
        <Divider
          orientation="vertical"
          className="mr-2 data-vertical:h-4 data-vertical:self-auto"
        />
        <nav aria-label="breadcrumb">
          <ol className="flex flex-wrap items-center gap-1.5 text-sm text-muted-foreground">
            {crumbs.map((crumb, index) => {
              const isLast = index === crumbs.length - 1
              return (
                <React.Fragment key={crumb.label}>
                  <li
                    className={isLast ? 'text-foreground' : 'hidden md:block'}
                  >
                    {isLast ? (
                      <span aria-current="page">{crumb.label}</span>
                    ) : crumb.link ? (
                      <RouterLink
                        {...crumb.link}
                        tint="neutral"
                        underline="none"
                      >
                        {crumb.label}
                      </RouterLink>
                    ) : (
                      <span>{crumb.label}</span>
                    )}
                  </li>
                  {isLast ? null : (
                    <li aria-hidden className="hidden md:block">
                      <ChevronRightIcon className="size-3.5" />
                    </li>
                  )}
                </React.Fragment>
              )
            })}
          </ol>
        </nav>
      </div>
      {actions ? (
        <div className="flex shrink-0 items-center gap-2 px-4">{actions}</div>
      ) : null}
      <div className="flex shrink-0 items-center gap-1 pr-4">
        <ThemeToggle />
        <LocaleMenu />
      </div>
    </header>
  )
}
