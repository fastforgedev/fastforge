import { Outlet, createFileRoute, notFound } from '@tanstack/react-router'
import { RouterLink } from '#/components/router-link'
import { ApiError, api } from 'studio-api-client'
import { useI18n } from '#/lib/i18n'

/**
 * Resolves the project once for both shells below it — the working shell
 * (`_shell`) and the settings shell — without rendering any chrome itself.
 */
export const Route = createFileRoute('/p/$projectId')({
  loader: async ({ params }) => {
    try {
      return await api.getProject(params.projectId)
    } catch (error) {
      // A registered directory that has since moved reads as a 404 too, with
      // its own code — either way there is no project to show here.
      if (error instanceof ApiError && error.status === 404) {
        throw notFound({ data: error.message })
      }
      throw error
    }
  },
  notFoundComponent: ProjectNotFound,
  component: () => <Outlet />,
})

function ProjectNotFound({ data }: { data?: unknown }) {
  const { projectId } = Route.useParams()
  const { t } = useI18n()

  return (
    <div className="flex min-h-svh flex-col items-center justify-center gap-4 p-6 text-center">
      <div className="space-y-1">
        <p className="text-lg font-medium">{t('Project not found')}</p>
        <p className="text-sm text-muted-foreground">
          {typeof data === 'string' ? (
            data
          ) : (
            <>
              No project <code className="font-mono">{projectId}</code>.
            </>
          )}
        </p>
      </div>
      <RouterLink to="/">{t('Back to projects')}</RouterLink>
    </div>
  )
}
