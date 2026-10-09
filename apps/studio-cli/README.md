# Studio

Fastforge Studio: one web client over one HTTP contract, served either by a
binary on your machine or by a hosted Worker.

## Structure

| Path                  | What it is                                                        |
| --------------------- | ----------------------------------------------------------------- |
| `crates/studio-core`         | `studio-core` — domain model, wire contract, pure logic. No I/O, compiles to `wasm32` |
| `apps/studio-cli`            | `studio-cli` — the `fastforge studio` command and standalone `fastforge-studio` binary; local server over a real checkout |
| `apps/studio-api`            | `studio-api` — the hosted API, a Cloudflare Worker      |
| `apps/studio-api/openapi.yaml` | The contract. Both hosts serve it; the TS client is generated from it |
| `apps/studio-web`            | TanStack Start client, built as a static SPA                       |
| `packages/studio-api-client` | `studio-api-client` — generated types plus a thin fetch client    |

## Development

Run these commands from the Fastforge repository root.

```bash
pnpm install
```

`fastforge studio` starts the local server and opens the browser. Use `fastforge studio --no-open --port 7391` to start it without opening a browser; `fastforge studio serve` accepts the same server options. The standalone `fastforge-studio` binary remains available.

Local Studio is two processes: the Rust server, and Vite proxying `/v1` to it.

Projects are added from the web client; nothing needs to be passed on the command line:

```bash
cargo run --bin fastforge -- studio serve --no-open
```

```bash
pnpm studio:dev
```

The client runs at `http://localhost:3000`, the API at `http://127.0.0.1:7391`.
`http://127.0.0.1:7391/reference` browses the contract.

To run it the way it ships — one process, the Rust server hosting the built
client — build the web app first:

```bash
pnpm studio:build && cargo run --bin fastforge -- studio serve
```

Check that a project's store credentials resolve without opening anything:

```bash
cargo run --bin fastforge -- studio doctor --dir /path/to/a/project
```

## Checks

```bash
pnpm studio:lint && pnpm studio:typecheck && pnpm studio:test
cargo clippy -p studio-core -p studio-cli --all-targets
```

The Worker only builds for `wasm32-unknown-unknown`, so it is excluded from the
Cargo workspace's default members and checked separately — `pnpm studio:typecheck`
covers it.

## Regenerating the client

`apps/studio-api/openapi.yaml` is the single source of the contract. After changing it:

```bash
pnpm --filter studio-api-client codegen
```

`cargo test -p studio-core` fails if the document and the Rust types
have drifted apart.
