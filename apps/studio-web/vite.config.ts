import { defineConfig } from 'vite'
import { devtools } from '@tanstack/devtools-vite'

import { tanstackStart } from '@tanstack/react-start/plugin/vite'

import viteReact from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'
import { dazzuiThemes } from './vite-plugins/dazzui-themes.ts'

/** Where `fastforge-studio serve` listens. Keep in sync with the CLI default. */
const STUDIO_API = process.env.STUDIO_API_URL ?? 'http://127.0.0.1:7391'

const config = defineConfig({
  resolve: { tsconfigPaths: true },
  server: {
    // The API is a separate process in development. Proxying keeps the client
    // same-origin, which is what it is in production too — so no CORS, and the
    // local server's origin check stays strict.
    proxy: {
      '/v1': { target: STUDIO_API, changeOrigin: false },
      '/openapi.json': { target: STUDIO_API, changeOrigin: false },
    },
  },
  plugins: [
    dazzuiThemes(),
    devtools(),
    tailwindcss(),
    tanstackStart({
      // Studio is served by a Rust binary locally, with no Node runtime to
      // render on. SPA mode emits a static shell that host can serve, and
      // every loader runs in the browser against the same HTTP contract.
      spa: { enabled: true, prerender: { outputPath: '/index.html' } },
    }),
    viteReact(),
  ],
})

export default config
