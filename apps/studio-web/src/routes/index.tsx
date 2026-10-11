import { createFileRoute } from '@tanstack/react-router'
import {
  Badge,
  Button,
  Card,
  Code,
  EmptyState,
  PreferenceGroup,
  SectionLabel,
} from '@dazzlabs/dazzui'
import { RouterLink } from '#/components/router-link'
import { ThemeToggle } from '#/components/theme-toggle'
import { LocaleMenu } from '#/components/locale-menu'
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
        <ThemeToggle />
        <LocaleMenu />
      </header>

      <section className="space-y-3">
        <div className="flex items-center justify-between">
          <SectionLabel>{t('Projects')}</SectionLabel>
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
          <EmptyState
            title={
              <>
                {t('No projects yet. Register one by running')}{' '}
                <Code>fastforge-studio serve</Code>{' '}
                {t('from a project directory.')}
              </>
            }
          />
        ) : (
          <ul className="grid gap-3 sm:grid-cols-2">
            {projects.map((project) => (
              <li key={project.id}>
                <Card>
                  <PreferenceGroup
                    title={
                      <RouterLink
                        to="/p/$projectId"
                        params={{ projectId: project.id }}
                      >
                        {project.name}
                      </RouterLink>
                    }
                    description={project.path}
                  />
                  {/* A registered directory that has moved or been deleted:
                        still listed so it can be removed, but nothing inside it
                        can be read. */}
                  {project.missing ? (
                    <Badge tint="warning">
                      <AlertTriangleIcon />
                      {t('Directory not found')}
                    </Badge>
                  ) : null}
                  {!project.missing && (
                    <Badge tint="neutral">
                      {formatPlatforms(project.platforms)}
                    </Badge>
                  )}
                </Card>
              </li>
            ))}
          </ul>
        )}
      </section>
    </main>
  )
}
