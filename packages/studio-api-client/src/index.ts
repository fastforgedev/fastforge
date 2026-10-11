import type { components } from './schema'

export type Capabilities = components['schemas']['Capabilities']
export type StudioMode = components['schemas']['StudioMode']
export type Platform = components['schemas']['Platform']
export type Project = components['schemas']['Project']
export type ProjectSummary = components['schemas']['ProjectSummary']
export type StoreKind = components['schemas']['StoreKind']
export type StoreConnection = components['schemas']['StoreConnection']
export type StoreApp = components['schemas']['StoreApp']
export type AuthStatus = components['schemas']['AuthStatus']
export type AuthFieldSource = components['schemas']['AuthFieldSource']
export type CatalogState = components['schemas']['CatalogState']
export type CatalogTree = components['schemas']['CatalogTree']
export type CatalogEntry = components['schemas']['CatalogEntry']
export type CatalogFile = components['schemas']['CatalogFile']
export type StoreListing = components['schemas']['StoreListing']
export type ListingMediaGroup = components['schemas']['ListingMediaGroup']
export type ListingMediaItem = components['schemas']['ListingMediaItem']
export type ListingTrack = components['schemas']['ListingTrack']
export type Run = components['schemas']['Run']
export type DirectoryListing = components['schemas']['DirectoryListing']
export type PackagingFile = components['schemas']['PackagingFile']
export type PackagingFormat = components['schemas']['PackagingFormat']

/**
 * A failure the server described. `code` is the stable half — branch on it —
 * while `message` is what a human should read.
 */
export class ApiError extends Error {
  readonly status: number
  readonly code: string

  constructor(status: number, code: string, message: string) {
    super(message)
    this.name = 'ApiError'
    this.status = status
    this.code = code
  }

  /** The endpoint exists in the contract but this build does not serve it. */
  get notImplemented(): boolean {
    return this.status === 501
  }
}

export type ClientOptions = {
  /** Defaults to same-origin, which is how both hosts serve the client. */
  baseUrl?: string
  fetch?: typeof globalThis.fetch
}

type Envelope<T> = { data: T } | { error: { code: string; message: string } }

export function createClient(options: ClientOptions = {}) {
  const baseUrl = (options.baseUrl ?? '').replace(/\/$/, '')
  const doFetch = options.fetch ?? globalThis.fetch

  function url(path: string, query?: Record<string, string | undefined>): string {
    const search = new URLSearchParams()
    for (const [key, value] of Object.entries(query ?? {})) {
      if (value !== undefined) search.set(key, value)
    }
    const suffix = search.size > 0 ? `?${search}` : ''
    return `${baseUrl}${path}${suffix}`
  }

  async function request<T>(
    path: string,
    init: RequestInit & { query?: Record<string, string | undefined> } = {},
  ): Promise<T> {
    const { query, ...rest } = init
    const response = await doFetch(url(path, query), {
      ...rest,
      headers: {
        Accept: 'application/json',
        ...(rest.body ? { 'Content-Type': 'application/json' } : {}),
        ...rest.headers,
      },
    })

    if (response.status === 204) {
      return undefined as T
    }

    // A non-JSON body means something upstream answered instead of the API —
    // a proxy, or the SPA shell. Saying so beats "Unexpected token <".
    let payload: Envelope<T>
    try {
      payload = (await response.json()) as Envelope<T>
    } catch {
      throw new ApiError(
        response.status,
        'INVALID_RESPONSE',
        `${path} did not return JSON (${response.status})`,
      )
    }

    if ('error' in payload) {
      throw new ApiError(response.status, payload.error.code, payload.error.message)
    }
    if (!response.ok) {
      throw new ApiError(response.status, 'UNKNOWN_ERROR', `${path} failed (${response.status})`)
    }
    return payload.data
  }

  const segment = encodeURIComponent

  return {
    /** Absolute URL for a catalog image, for use as an `<img src>`. */
    catalogRawUrl(projectId: string, storeAppId: string, path: string): string {
      return url(
        `/v1/projects/${segment(projectId)}/store-apps/${segment(storeAppId)}/catalog/raw`,
        { path },
      )
    },

    getCapabilities: () => request<Capabilities>('/v1/capabilities'),

    listProjects: () => request<Array<ProjectSummary>>('/v1/projects'),

    createProject: (body: components['schemas']['CreateProjectRequest']) =>
      request<Project>('/v1/projects', { method: 'POST', body: JSON.stringify(body) }),

    getProject: (projectId: string) => request<Project>(`/v1/projects/${segment(projectId)}`),

    updateProject: (projectId: string, body: components['schemas']['UpdateProjectRequest']) =>
      request<Project>(`/v1/projects/${segment(projectId)}`, {
        method: 'PATCH',
        body: JSON.stringify(body),
      }),

    deleteProject: (projectId: string) =>
      request<void>(`/v1/projects/${segment(projectId)}`, { method: 'DELETE' }),

    getPackagingFile: (projectId: string, format: PackagingFormat) =>
      request<PackagingFile>(
        `/v1/projects/${segment(projectId)}/packaging/${segment(format)}`,
      ),

    putPackagingFile: (
      projectId: string,
      format: PackagingFormat,
      body: components['schemas']['PutPackagingFileRequest'],
    ) =>
      request<PackagingFile>(
        `/v1/projects/${segment(projectId)}/packaging/${segment(format)}`,
        {
          method: 'PUT',
          body: JSON.stringify(body),
        },
      ),

    listStores: (projectId: string) =>
      request<Array<StoreConnection>>(`/v1/projects/${segment(projectId)}/stores`),

    listStoreApps: (projectId: string) =>
      request<Array<StoreApp>>(`/v1/projects/${segment(projectId)}/store-apps`),

    getStoreApp: (projectId: string, storeAppId: string) =>
      request<StoreApp>(
        `/v1/projects/${segment(projectId)}/store-apps/${segment(storeAppId)}`,
      ),

    getStoreListing: (
      projectId: string,
      storeAppId: string,
      options: { locale?: string; version?: string } = {},
    ) =>
      request<StoreListing>(
        `/v1/projects/${segment(projectId)}/store-apps/${segment(storeAppId)}/listing`,
        { query: { locale: options.locale, version: options.version } },
      ),

    getCatalogTree: (projectId: string, storeAppId: string) =>
      request<CatalogTree>(
        `/v1/projects/${segment(projectId)}/store-apps/${segment(storeAppId)}/catalog`,
      ),

    getCatalogFile: (projectId: string, storeAppId: string, path: string) =>
      request<CatalogFile>(
        `/v1/projects/${segment(projectId)}/store-apps/${segment(storeAppId)}/catalog/file`,
        { query: { path } },
      ),

    putCatalogFile: (
      projectId: string,
      storeAppId: string,
      path: string,
      body: components['schemas']['PutCatalogFileRequest'],
    ) =>
      request<CatalogFile>(
        `/v1/projects/${segment(projectId)}/store-apps/${segment(storeAppId)}/catalog/file`,
        { method: 'PUT', query: { path }, body: JSON.stringify(body) },
      ),

    pullCatalog: (projectId: string, storeAppId: string) =>
      request<Run>(
        `/v1/projects/${segment(projectId)}/store-apps/${segment(storeAppId)}/catalog/pull`,
        { method: 'POST' },
      ),

    browse: (path?: string) =>
      request<DirectoryListing>('/v1/fs/browse', { query: { path } }),
  }
}

export type StudioClient = ReturnType<typeof createClient>

/** Same-origin client. Both hosts serve the web client from their own origin. */
export const api: StudioClient = createClient()
