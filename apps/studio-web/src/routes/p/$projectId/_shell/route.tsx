import { StudioSidebarProvider } from '#/components/studio-sidebar'
import { Outlet, createFileRoute, useLoaderData } from '@tanstack/react-router'
import { SidebarInset } from '@dazzlabs/dazzui'
import { api } from 'studio-api-client'

import { AppSidebar } from '#/components/app-sidebar'

export const Route = createFileRoute('/p/$projectId/_shell')({
  // Everything the sidebar needs, loaded once here rather than by each page
  // under it: the stores it lists, and the projects its switcher offers.
  loader: async ({ params }) => {
    const [stores, projects] = await Promise.all([
      api.listStores(params.projectId),
      api.listProjects(),
    ])
    return { stores, projects }
  },
  component: ProjectShell,
})

function ProjectShell() {
  const project = useLoaderData({ from: '/p/$projectId' })
  const { stores, projects } = Route.useLoaderData()

  return (
    <StudioSidebarProvider>
      <AppSidebar project={project} stores={stores} projects={projects} />
      <SidebarInset>
        <Outlet />
      </SidebarInset>
    </StudioSidebarProvider>
  )
}
