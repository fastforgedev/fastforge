import {
  Badge,
  Callout,
  Card,
  Link as DazzLink,
  PillTabs,
  PreferenceSection,
  Table,
  TableRow,
  TableCell,
  PreferenceGroup,
  EmptyState,
  Button,
  Menu,
} from '@dazzlabs/dazzui'
import { createFileRoute, notFound } from '@tanstack/react-router'
import { ApiError, api } from 'studio-api-client'
import { ChevronDownIcon, DownloadIcon, StoreIcon } from 'lucide-react'

import { AuthFieldList, AuthStatusBadge } from '#/components/auth-status-badge'
import { PageBody } from '#/components/page-body'
import { PageHeader } from '#/components/page-header'
import { useI18n } from '#/lib/i18n'

import type { StoreApp, StoreKind, StoreListing } from 'studio-api-client'

type Search = {
  /** Which app in this store, when the project ships more than one. */
  app?: string
  locale?: string
  version?: string
}

export const Route = createFileRoute('/p/$projectId/_shell/stores/$storeId')({
  validateSearch: (search: Record<string, unknown>): Search => ({
    app: typeof search.app === 'string' ? search.app : undefined,
    locale: typeof search.locale === 'string' ? search.locale : undefined,
    version: typeof search.version === 'string' ? search.version : undefined,
  }),
  loaderDeps: ({ search }) => search,
  loader: async ({ params, deps }) => {
    const store = params.storeId as StoreKind
    try {
      const [connections, allApps] = await Promise.all([
        api.listStores(params.projectId),
        api.listStoreApps(params.projectId),
      ])

      const connection = connections.find((item) => item.store === store)
      if (!connection) {
        throw notFound({ data: `${params.storeId} is not a known store.` })
      }

      const apps = allApps.filter((app) => app.store === store)
      // `.at` rather than `[0]`: a store with no apps is a normal state, and
      // indexing would type it away.
      const selected = apps.find((app) => app.id === deps.app) ?? apps.at(0)
      const listing = selected
        ? await api.getStoreListing(params.projectId, selected.id, {
            locale: deps.locale,
            version: deps.version,
          })
        : null

      return { connection, apps, selected, listing }
    } catch (error) {
      if (error instanceof ApiError && error.status === 404) {
        throw notFound({ data: error.message })
      }
      throw error
    }
  },
  notFoundComponent: StoreNotFound,
  component: StoreDetail,
})

function StoreDetail() {
  const { projectId } = Route.useParams()
  const { connection, apps, selected, listing } = Route.useLoaderData()
  const { t } = useI18n()

  return (
    <>
      <PageHeader
        crumbs={[
          {
            label: t('Stores'),
            link: { to: '/p/$projectId/stores', params: { projectId } },
          },
          { label: connection.name },
        ]}
        actions={
          // Pulling needs fastforge's store clients, which are not wired into
          // the server yet. The button stays visible so the page shows what it
          // is for, and the empty state says what to run instead.
          <Button size="small" variant="outlined" disabled>
            <DownloadIcon data-icon="inline-start" />
            {t('Pull catalog')}
          </Button>
        }
      />
      <PageBody>
        <PreferenceGroup
          title={
            <span role="heading" aria-level={1}>
              {connection.name}
            </span>
          }
          description={connection.description}
          action={<AuthStatusBadge status={connection.authStatus} />}
        />

        {connection.authStatus.state === 'incomplete' ? (
          <Callout tint="warning" title={t('Credentials incomplete')}>
            <p className="text-sm text-muted-foreground">
              {t('Syncing this store will fail until these resolve.')}
            </p>
            <AuthFieldList status={connection.authStatus} />
          </Callout>
        ) : null}

        {!selected || !listing ? (
          <EmptyState
            title={
              <>
                <StoreIcon aria-hidden />
                <span role="heading" aria-level={2}>
                  {t('No apps under this store')}
                </span>
                <p>{`Add an app to the ${connection.store} section of .fastforge/config.yaml, then pull its catalog.`}</p>
              </>
            }
          />
        ) : (
          <>
            <AppPicker apps={apps} selected={selected} />
            {listing.empty ? (
              <EmptyState
                title={
                  <>
                    <StoreIcon aria-hidden />
                    <span role="heading" aria-level={2}>
                      {t('Nothing pulled yet')}
                    </span>
                    <p>{`Run \`fastforge store catalog pull\` in the project to fetch this app's listing, screenshots and release notes.`}</p>
                  </>
                }
              />
            ) : (
              <Listing listing={listing} storeAppId={selected.id} />
            )}
          </>
        )}
      </PageBody>
    </>
  )
}

/** Only shown when the store actually holds more than one app. */
function AppPicker({
  apps,
  selected,
}: {
  apps: Array<StoreApp>
  selected: StoreApp
}) {
  const navigate = Route.useNavigate()
  if (apps.length < 2) return null
  return (
    <PillTabs
      value={selected.id}
      items={apps.map((app) => ({
        value: app.id,
        label: app.name ?? app.identifier,
      }))}
      onValueChange={(app) => {
        void navigate({ search: (previous) => ({ ...previous, app }) })
      }}
    />
  )
}

/**
 * The listing as the store presents it — name, screenshots, description,
 * what's new — rather than as the catalog stores it.
 */
function Listing({
  listing,
  storeAppId,
}: {
  listing: StoreListing
  storeAppId: string
}) {
  const { projectId } = Route.useParams()
  const { t } = useI18n()
  // `media` and `tracks` are omitted rather than sent empty, so the page reads
  // them through a default instead of guarding at every use.
  const media = listing.media ?? []
  const tracks = listing.tracks ?? []

  return (
    <div className="space-y-8">
      <header className="flex flex-wrap items-start justify-between gap-4">
        <div className="min-w-0 space-y-1">
          <h2 className="text-2xl font-bold tracking-tight">
            {listing.name ?? listing.identifier}
          </h2>
          {listing.subtitle ? (
            <p className="text-muted-foreground">{listing.subtitle}</p>
          ) : null}
          <p className="font-mono text-xs text-muted-foreground">
            {listing.identifier}
          </p>
        </div>
        <div className="flex shrink-0 gap-2">
          <Picker
            label={t('Locale')}
            value={listing.locale}
            options={listing.locales}
            toSearch={(locale) => ({ locale })}
          />
          <Picker
            label={listing.store === 'googleplay' ? t('Track') : t('Version')}
            value={listing.version}
            options={listing.versions}
            format={versionLabel}
            toSearch={(version) => ({ version })}
          />
        </div>
      </header>

      {media.map((group) => (
        <section key={group.id} className="space-y-2">
          <h3 className="text-sm font-medium text-muted-foreground">
            {group.label}
          </h3>
          {/* Horizontal, like the store: screenshots are a sequence, and the
              order is the order `push` uploads them in. */}
          <div className="flex gap-3 overflow-x-auto pb-2">
            {group.items.map((item) => (
              <img
                key={item.path}
                src={api.catalogRawUrl(projectId, storeAppId, item.path)}
                alt={item.name}
                decoding="async"
                // Deliberately not `loading="lazy"`. With `w-auto` these have
                // no intrinsic width until they load, so they lay out at zero
                // and the lazy heuristic never decides they are near the
                // viewport — leaving the gallery blank indefinitely. The
                // `min-w` keeps the row from collapsing while they arrive.
                className="h-96 w-auto min-w-40 shrink-0 object-contain"
              />
            ))}
          </div>
        </section>
      ))}

      {listing.promotionalText ? (
        <Prose title={t('Promotional text')} body={listing.promotionalText} />
      ) : null}
      {listing.whatsNew ? (
        <Prose title={t("What's New")} body={listing.whatsNew} />
      ) : null}
      {listing.description ? (
        <Prose title={t('Description')} body={listing.description} />
      ) : null}

      {tracks.length > 0 ? (
        <section className="space-y-2">
          <h3 className="font-semibold">{t('Tracks')}</h3>
          <Card>
            {tracks.map((track) => (
              <PreferenceSection key={track.name} label={track.name}>
                {track.releases?.length ? (
                  track.releases.map((release, index) => (
                    <p
                      key={`${release.name ?? index}`}
                      className="text-sm text-muted-foreground"
                    >
                      {[
                        release.name,
                        release.status,
                        release.versionCodes?.length
                          ? `build ${release.versionCodes.join(', ')}`
                          : undefined,
                        release.userFraction !== undefined
                          ? `${Math.round(release.userFraction * 100)}% rollout`
                          : undefined,
                      ]
                        .filter(Boolean)
                        .join(' · ')}
                    </p>
                  ))
                ) : (
                  <p className="text-sm text-muted-foreground">
                    {t('No releases')}
                  </p>
                )}
              </PreferenceSection>
            ))}
          </Card>
        </section>
      ) : null}

      <Information listing={listing} />
    </div>
  )
}

function Prose({ title, body }: { title: string; body: string }) {
  return (
    <section className="space-y-2">
      <h3 className="font-semibold">{title}</h3>
      {/* Store descriptions are plain text with meaningful line breaks, not
          markup — so they are preserved rather than rendered. */}
      <p className="max-w-3xl text-sm leading-relaxed whitespace-pre-wrap">
        {body}
      </p>
    </section>
  )
}

function Information({ listing }: { listing: StoreListing }) {
  const { t } = useI18n()
  const rows: Array<[string, React.ReactNode]> = []
  if (listing.categories?.length) {
    rows.push([t('Categories'), listing.categories.join(' · ')])
  }
  if (listing.keywords) rows.push([t('Keywords'), listing.keywords])
  if (listing.copyright) rows.push([t('Copyright'), listing.copyright])
  if (listing.marketingUrl) {
    rows.push([
      t('Marketing'),
      <DazzLink
        key="m"
        href={listing.marketingUrl}
        target="_blank"
        rel="noreferrer noopener"
      >
        {listing.marketingUrl}
      </DazzLink>,
    ])
  }
  if (listing.supportUrl) {
    rows.push([
      t('Support'),
      <DazzLink
        key="s"
        href={listing.supportUrl}
        target="_blank"
        rel="noreferrer noopener"
      >
        {listing.supportUrl}
      </DazzLink>,
    ])
  }
  if (listing.privacyPolicyUrl) {
    rows.push([
      t('Privacy policy'),
      <DazzLink
        key="p"
        href={listing.privacyPolicyUrl}
        target="_blank"
        rel="noreferrer noopener"
      >
        {listing.privacyPolicyUrl}
      </DazzLink>,
    ])
  }
  if (rows.length === 0) {
    return null
  }

  return (
    <section className="space-y-2">
      <h3 className="font-semibold">{t('Information')}</h3>
      <Table>
        {rows.map(([label, value]) => (
          <TableRow key={label}>
            <TableCell className="w-40 shrink-0">{label}</TableCell>
            <TableCell className="min-w-0 flex-1 break-words">
              {value}
            </TableCell>
          </TableRow>
        ))}
      </Table>
    </section>
  )
}

/** `IOS/1.2.0` is how a version is addressed; `1.2.0 · IOS` is how it reads. */
function versionLabel(version: string): string {
  const [platform, name] = version.split('/')
  return name ? `${name} · ${platform}` : version
}

function Picker({
  label,
  value,
  options,
  format = (option: string) => option,
  toSearch,
}: {
  label: string
  value?: string
  options: Array<string>
  format?: (option: string) => string
  toSearch: (option: string) => Search
}) {
  const navigate = Route.useNavigate()
  if (options.length === 0) {
    return null
  }
  // One option is not a choice — showing a menu that cannot change anything
  // only invites a click that does nothing.
  if (options.length === 1) {
    return (
      <Badge tint="neutral">
        {label}: {format(options[0])}
      </Badge>
    )
  }

  return (
    <Menu
      align="end"
      className="max-h-80 overflow-auto"
      trigger={
        <Button size="small" variant="outlined">
          {label}: {format(value ?? options[0])}
          <ChevronDownIcon />
        </Button>
      }
      items={options.map((option) => ({
        label: format(option),
        checked: option === (value ?? options[0]),
        onSelect: () => {
          void navigate({
            search: (prev) => ({ ...prev, ...toSearch(option) }),
          })
        },
      }))}
    />
  )
}

function StoreNotFound({ data }: { data?: unknown }) {
  const { storeId } = Route.useParams()
  const { t } = useI18n()

  return (
    <PageBody>
      <EmptyState
        title={
          <>
            <StoreIcon aria-hidden />
            <span role="heading" aria-level={2}>
              {t('Store not found')}
            </span>
            <p>
              {typeof data === 'string'
                ? data
                : `${storeId} is not a known store.`}
            </p>
          </>
        }
      />
    </PageBody>
  )
}
