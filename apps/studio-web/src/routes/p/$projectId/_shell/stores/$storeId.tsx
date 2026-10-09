import { createFileRoute, notFound } from '@tanstack/react-router'
import { ApiError, api } from 'studio-api-client'
import { Button, Menu } from '@dazzlabs/dazzui'
import { RouterLink } from '#/components/router-link'
import { ChevronDownIcon, DownloadIcon, StoreIcon } from 'lucide-react'

import { AuthFieldList, AuthStatusBadge } from '#/components/auth-status-badge'
import { EmptyState, PageBody, PageTitle } from '#/components/page-body'
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
        <PageTitle
          title={connection.name}
          description={connection.description}
          actions={<AuthStatusBadge status={connection.authStatus} />}
        />

        {connection.authStatus.state === 'incomplete' ? (
          <div className="rounded-xl border border-amber-500/40 bg-card p-4">
            <p className="text-sm font-medium">{t('Credentials incomplete')}</p>
            <p className="text-sm text-muted-foreground">
              {t('Syncing this store will fail until these resolve.')}
            </p>
            <AuthFieldList status={connection.authStatus} />
          </div>
        ) : null}

        {!selected || !listing ? (
          <EmptyState
            icon={StoreIcon}
            title={t('No apps under this store')}
            description={`Add an app to the ${connection.store} section of .fastforge/config.yaml, then pull its catalog.`}
          />
        ) : (
          <>
            <AppPicker apps={apps} selected={selected} />
            {listing.empty ? (
              <EmptyState
                icon={StoreIcon}
                title={t('Nothing pulled yet')}
                description={`Run \`fastforge store catalog pull\` in the project to fetch this app's listing, screenshots and release notes.`}
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
  if (apps.length < 2) {
    return null
  }
  return (
    <div className="flex flex-wrap gap-2">
      {apps.map((app) => (
        <RouterLink
          key={app.id}
          from={Route.fullPath}
          search={{ app: app.id }}
          tint={app.id === selected.id ? 'primary' : 'neutral'}
          underline="none"
          className="rounded-md px-3 py-1.5"
          aria-current={app.id === selected.id ? 'page' : undefined}
        >
          {app.name ?? app.identifier}
        </RouterLink>
      ))}
    </div>
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
                className="h-96 w-auto min-w-40 shrink-0 rounded-xl border bg-muted object-contain"
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
          <ul className="divide-y rounded-xl border bg-card">
            {tracks.map((track) => (
              <li key={track.name} className="space-y-1 p-4">
                <p className="text-sm font-medium">{track.name}</p>
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
              </li>
            ))}
          </ul>
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
      <ExternalLink key="m" href={listing.marketingUrl} />,
    ])
  }
  if (listing.supportUrl) {
    rows.push([
      t('Support'),
      <ExternalLink key="s" href={listing.supportUrl} />,
    ])
  }
  if (listing.privacyPolicyUrl) {
    rows.push([
      t('Privacy policy'),
      <ExternalLink key="p" href={listing.privacyPolicyUrl} />,
    ])
  }
  if (rows.length === 0) {
    return null
  }

  return (
    <section className="space-y-2">
      <h3 className="font-semibold">{t('Information')}</h3>
      <dl className="divide-y rounded-xl border bg-card text-sm">
        {rows.map(([label, value]) => (
          <div key={label} className="flex gap-4 p-4">
            <dt className="w-40 shrink-0 text-muted-foreground">{label}</dt>
            <dd className="min-w-0 flex-1 break-words">{value}</dd>
          </div>
        ))}
      </dl>
    </section>
  )
}

function ExternalLink({ href }: { href: string }) {
  return (
    <a
      href={href}
      target="_blank"
      rel="noreferrer noopener"
      className="underline underline-offset-4"
    >
      {href}
    </a>
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
      <p className="px-3 py-1.5 text-sm text-muted-foreground">
        {label}: {format(options[0])}
      </p>
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
        icon={StoreIcon}
        title={t('Store not found')}
        description={
          typeof data === 'string' ? data : `${storeId} is not a known store.`
        }
      />
    </PageBody>
  )
}
