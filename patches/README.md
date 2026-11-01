# Dependency compatibility patches

The workspace overrides `minimatch` to 11.x to fix
[GHSA-g3hv-75wg-m249](https://github.com/advisories/GHSA-g3hv-75wg-m249).
Two upstream consumers still use APIs removed in that version:

- `eslint-plugin-import-x@4.17.1`: use `Minimatch.match()` for the
  `no-internal-modules` rule's allow and forbid patterns, replacing `makeRe()`.
  Both its ESM files and CommonJS bundle need the change.
- `@redocly/openapi-core@1.34.20`: import the named `minimatch` function instead
  of calling the CommonJS module object.

These patches are applied by pnpm through `patchedDependencies` in
`pnpm-workspace.yaml`. Remove each patch once its upstream package supports
minimatch 11. To update a patch, use `pnpm patch` and `pnpm patch-commit`.

Run `node --test scripts/test-dependency-patches.mjs` to check both plugin
formats and OpenAPI URL matching. These checks also run in `pnpm studio:test`.
