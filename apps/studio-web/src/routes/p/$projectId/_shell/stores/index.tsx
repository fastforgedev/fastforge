import {
  Card,
  Callout,
  Code,
  EmptyState,
  Table,
  TableRow,
  TableCell,
  Badge,
  PreferenceGroup,
} from '@dazzlabs/dazzui'
import { RouterLink } from '#/components/router-link'
import { createFileRoute } from '@tanstack/react-router'
import { api } from 'studio-api-client'
import { PackageIcon } from 'lucide-react'

import { AuthFieldList, AuthStatusBadge } from '#/components/auth-status-badge'
import { PageBody } from '#/components/page-body'
import { PageHeader } from '#/components/page-header'
import { useI18n } from '#/lib/i18n'

import type { StoreApp, StoreConnection } from 'studio-api-client'

export const Route = createFileRoute('/p/$projectId/_shell/stores/')({
  loader: async ({ params }) => {
    const [connections, apps] = await Promise.all([
      api.listStores(params.projectId),
      api.listStoreApps(params.projectId),
    ])
    return { connections, apps }
  },
  component: Stores,
})

function Stores() {
  const { projectId } = Route.useParams()
  const { connections, apps } = Route.useLoaderData()
  const { t } = useI18n()

  return (
    <>
      <PageHeader crumbs={[{ label: t('Stores') }]} />
      <PageBody>
        <PreferenceGroup
          title={
            <span role="heading" aria-level={1}>
              {t('Stores')}
            </span>
          }
          description={t(
            'Storefronts that own the listing: versions, tracks, review state and catalog metadata.',
          )}
        />

        <div className="space-y-4">
          {connections.map((connection) => (
            <StoreCard
              key={connection.store}
              projectId={projectId}
              connection={connection}
              apps={apps.filter((app) => app.store === connection.store)}
            />
          ))}
        </div>

        <p className="text-xs text-muted-foreground">
          Stores and their apps are read from{' '}
          <code className="font-mono">.fastforge/config.yaml</code>. Credentials
          come from the environment — Studio only reports whether they resolve,
          never their values.
        </p>
      </PageBody>
    </>
  )
}

function StoreCard({
  projectId,
  connection,
  apps,
}: {
  projectId: string
  connection: StoreConnection
  apps: Array<StoreApp>
}) {
  const { t } = useI18n()
  return (
    <Card>
      <PreferenceGroup
        title={
          <RouterLink
            to="/p/$projectId/stores/$storeId"
            params={{ projectId, storeId: connection.store }}
          >
            {connection.name}
          </RouterLink>
        }
        description={connection.description}
        action={<AuthStatusBadge status={connection.authStatus} />}
      />
      <AuthFieldList status={connection.authStatus} />
      {!connection.configured ? (
        <Callout>
          Add a <Code>{connection.store}</Code> section to{' '}
          <Code>.fastforge/config.yaml</Code> to connect this store.
        </Callout>
      ) : apps.length === 0 ? (
        <EmptyState title={t('No apps registered under this store.')} />
      ) : (
        <Table>
          {apps.map((app) => (
            <TableRow key={app.id}>
              <TableCell className="min-w-0 flex-1">
                <RouterLink
                  to="/p/$projectId/stores/$storeId"
                  params={{ projectId, storeId: app.store }}
                  search={{ app: app.id }}
                >
                  {app.name ?? app.identifier}
                </RouterLink>
                <p>
                  <Code>
                    {app.identifier}
                    {app.appId ? ` · ${app.appId}` : ''}
                    {app.track ? ` · ${app.track}` : ''}
                  </Code>
                </p>
              </TableCell>
              <TableCell>
                <Badge tint="neutral">
                  <PackageIcon />
                  {app.catalog.exists
                    ? t('{count} catalog files', {
                        count: app.catalog.fileCount,
                      })
                    : t('Not pulled')}
                </Badge>
              </TableCell>
            </TableRow>
          ))}
        </Table>
      )}
    </Card>
  )
}
