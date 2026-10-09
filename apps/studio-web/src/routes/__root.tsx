import { HeadContent, Scripts, createRootRoute } from '@tanstack/react-router'
import { TanStackRouterDevtoolsPanel } from '@tanstack/react-router-devtools'
import { TanStackDevtools } from '@tanstack/react-devtools'

import { TooltipProvider } from '@dazzlabs/dazzui'
import { CapabilitiesProvider } from '#/lib/capabilities'
import { I18nProvider } from '#/lib/i18n'

import appCss from '../styles.css?url'

const faviconSvg = `data:image/svg+xml,${encodeURIComponent(
  '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 128 128"><defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="1"><stop stop-color="#1E40B0"/><stop offset=".35" stop-color="#2563EB"/><stop offset=".7" stop-color="#1688F5"/><stop offset="1" stop-color="#22D3EE"/></linearGradient></defs><path d="M19,23 L37,23 L75,64 L37,105 L19,105 L41,64 Z" fill="url(#g)"/><path d="M66,23 L84,23 L122,64 L84,105 L66,105 L88,64 Z" fill="url(#g)"/></svg>',
)}`

export const Route = createRootRoute({
  head: () => ({
    meta: [
      {
        charSet: 'utf-8',
      },
      {
        name: 'viewport',
        content: 'width=device-width, initial-scale=1',
      },
      {
        title: 'Fastforge Studio',
      },
      {
        name: 'description',
        content: 'Build, package, and publish your apps with Fastforge Studio.',
      },
    ],
    links: [
      {
        rel: 'stylesheet',
        href: appCss,
      },
      {
        rel: 'icon',
        type: 'image/svg+xml',
        href: faviconSvg,
      },
    ],
  }),
  // Studio's client is client-only: it is served as static files, by a Rust
  // binary locally, with no Node runtime to render on.
  ssr: false,
  shellComponent: RootDocument,
})

function RootDocument({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en">
      <head>
        <HeadContent />
      </head>
      <body>
        {/*
          The sidebar lives in the layout routes rather than here: the project
          shell and the settings shell use different sidebars, and the projects
          index has none at all.
        */}
        {/*
          Capabilities gate every sidebar and settings surface, so the provider
          fetches them before anything under it renders.
        */}
        <I18nProvider>
          <TooltipProvider>
            <CapabilitiesProvider>{children}</CapabilitiesProvider>
          </TooltipProvider>
        </I18nProvider>
        <TanStackDevtools
          config={{
            position: 'bottom-right',
          }}
          plugins={[
            {
              name: 'Tanstack Router',
              render: <TanStackRouterDevtoolsPanel />,
            },
          ]}
        />
        <Scripts />
      </body>
    </html>
  )
}
