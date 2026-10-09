import { Link, createFileRoute } from '@tanstack/react-router'
import { Button } from '@dazzlabs/dazzui'
import { api } from 'studio-api-client'
import { AlertTriangleIcon, PlusIcon } from 'lucide-react'

import { BrandIcon } from '#/components/brand-icon'
import { useCapabilities } from '#/lib/capabilities'
import { formatPlatforms } from '#/lib/workspace'
import { useI18n } from '#/lib/i18n'

export const Route = createFileRoute('/')({
  loader: () => api.listProjects(),
  component: ProjectsIndex,
})

/**
 * The one screen outside a project. Everything else is project-scoped, so this
 * is where a project gets picked rather than a switcher hidden in a sidebar.
 */
function ProjectsIndex() {
  const capabilities = useCapabilities()
  const { t } = useI18n()
  const projects = Route.useLoaderData()

  return (
    <main className="mx-auto flex min-h-svh w-full max-w-4xl flex-col gap-8 p-6 md:p-12">
      <header className="flex items-center gap-3">
        <BrandIcon className="size-9" />
        <div className="flex-1">
          <h1 className="text-xl font-bold tracking-tight">Fastforge Studio</h1>
          <p className="text-sm text-muted-foreground">
            {capabilities.mode === 'local' ? t('On this machine') : t('Hosted')}
          </p>
        </div>
      </header>

      <section className="space-y-3">
        <div className="flex items-center justify-between">
          <h2 className="font-semibold">{t('Projects')}</h2>
          {/* Registering from the browser needs the directory picker, which
              lands with the rest of the local filesystem surface. */}
          <Button size="small" variant="filled" disabled>
            <PlusIcon data-icon="inline-start" />
            {capabilities.mode === 'local'
              ? t('Add local project')
              : t('New project')}
          </Button>
        </div>

        {projects.length === 0 ? (
          <p className="rounded-xl border border-dashed p-8 text-center text-sm text-muted-foreground">
            {t('No projects yet. Register one by running')}{' '}
            <code className="font-mono text-xs">fastforge-studio serve</code>{' '}
            {t('from a project directory.')}
          </p>
        ) : (
          <ul className="grid gap-3 sm:grid-cols-2">
            {projects.map((project) => (
              <li key={project.id}>
                <Link
                  to="/p/$projectId"
                  params={{ projectId: project.id }}
                  className="block rounded-xl border bg-card p-5 transition-colors hover:border-foreground/20 hover:bg-muted/40"
                >
                  <p className="flex items-center gap-2 font-medium">
                    {project.name}
                    {/* A registered directory that has moved or been deleted:
                        still listed so it can be removed, but nothing inside it
                        can be read. */}
                    {project.missing ? (
                      <AlertTriangleIcon className="size-4 text-muted-foreground" />
                    ) : null}
                  </p>
                  {project.path ? (
                    <p className="mt-1 truncate font-mono text-xs text-muted-foreground">
                      {project.path}
                    </p>
                  ) : null}
                  <p className="mt-3 text-xs text-muted-foreground">
                    {project.missing
                      ? t('Directory not found')
                      : formatPlatforms(project.platforms)}
                  </p>
                </Link>
              </li>
            ))}
          </ul>
        )}
      </section>
    </main>
  )
}
