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
The appearance panel selects a DazzUI theme and light/dark mode. Existing
Default/Maple preferences fall back to DazzUI's Studio theme.
