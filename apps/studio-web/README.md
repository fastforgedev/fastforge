# studio-web

TanStack Start application for the Fastforge monorepo.

Run commands from the repository root:

```bash
pnpm studio:dev
pnpm studio:lint
pnpm studio:typecheck
pnpm studio:build
```

Studio imports components and both stylesheets directly from
[`@dazzlabs/dazzui`](https://github.com/dazzlabs/dazzui). Business components
and the responsive sidebar extension live in `src/components`.

DazzUI is pinned to a Git revision in this application's `package.json`; keep
the matching build permission in `pnpm-workspace.yaml` in sync when updating it.
The appearance panel offers all themes from the installed DazzUI stylesheet,
with light/dark mode. The Vite plugin discovers the theme names automatically.
The root theme provider restores preferences on every page, including the project
list. Components use DazzUI styling directly; the responsive sidebar adapter only
coordinates desktop collapse and the mobile drawer. Existing
Default/Maple preferences fall back to DazzUI's Studio theme.
