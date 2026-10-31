# Releasing

The repository uses cargo-dist 0.30.3. Its GitHub Actions release workflow builds
and attaches archives/installers for `gromnie-cli`, `gromnie-tui`, and
`gromnie-proxy` to a GitHub Release. The workflow also runs `dist plan` on pull
requests, so a PR checks that the release configuration is valid.

After merging a release-ready change, push a version tag from that commit:

```sh
git tag v0.1.0
git push origin v0.1.0
```

Use the version declared in the workspace (`Cargo.toml`) and update that version
before creating the tag. A pushed version tag starts the release workflow; the
workflow creates the GitHub Release and uploads the cargo-dist artifacts.

## crates.io publication status

Crate publication is not enabled yet. Two things block it:

1. Internal path dependencies (for example, `gromnie-cli` -> `gromnie-client`)
   do not declare a `version`, so `cargo publish` rejects the manifests. Adding
   `version = "0.1.0"` next to each `path` fixes this mechanically.
2. `wisp-mux` is pinned to an unpublished upstream commit (see below).

### External dependencies

| Dependency | Pinned by gromnie | Latest on crates.io | Status |
| --- | --- | --- | --- |
| `asheron-rs` | `=0.2.0` (registry) | `0.2.0` | Resolved |
| `wisp-mux` | git rev `0c11678` (6.0.0) | `5.1.0` (2024) | Blocked |

`asheron-rs` is published and gromnie now depends on the registry release.

`wisp-mux` 6.0.0 is **not** on crates.io, and the upstream repo does not have a
crates.io publish step in CI (its release workflow publishes the npm package
only). Gromnie depends on the 6.0.0 API, which is a rewrite compared to 5.1.0:

- `WispV2Handshake` / `AnyProtocolExtensionBuilder` (5.1.0 has no handshake
  wrapper; extension builders are `Box<dyn ProtocolExtensionBuilder>`)
- `ws::TransportRead`, `ws::TransportWrite`, `ws::TransportExt`,
  `ws::async_iterator_transport_read/write`, `ws::TokioTungsteniteTransport`
  (5.1.0 has no transport abstraction at all: it flips the model to
  `WebSocketRead` / `WebSocketWrite` traits over `Frame`s)
- generic `MuxStream<W>` with `Stream`/`Sink` impls (5.1.0 `MuxStream` is
  non-generic and provides `into_io()` / `read()` / `write()`)
- `ServerMux::new` / `ClientMux::new` / `wait_for_stream` / `new_stream`
  (5.1.0 names these `create` / `server_new_stream` / `client_new_stream`)

Also relevant: crates.io `wisp-mux` 5.1.0 is `LGPL-3.0-only`, while upstream
6.0.0 is `MIT`. Moving to 5.1.0 would add an LGPL dependency to MIT-licensed
gromnie binaries.

Impact: `gromnie-wisp`, `gromnie-web`, and `gromnie-proxy` consume the
6.0.0-only API. `gromnie-events`, `gromnie-client`, `gromnie-runner`,
`gromnie-tui`, `gromnie-cli`, `gromnie-scripting-api`, and
`gromnie-scripting-host` do not depend on `wisp-mux` and can be published once
the path-dependency versions are added.

## What gets published

All crates share the workspace version and are released together. Names are
available on crates.io today.

| # | Crate | Kind | Publish | Notes |
| --- | --- | --- | --- | --- |
| 1 | `gromnie-events` | lib | yes | `asheron-rs` only |
| 2 | `gromnie-scripting-api` | lib | yes | WIT interface crate |
| 3 | `gromnie-client` | lib | yes | depends on 1 |
| 4 | `gromnie-scripting-host` | lib | yes | depends on 1-3 |
| 5 | `gromnie-runner` | lib | yes | depends on 1, 3, 4 |
| 6 | `gromnie-tui` | lib + bin `tui` | yes | depends on 1, 3, 4, 5 |
| 7 | `gromnie-cli` | lib + bins `cli`, `discord-bot`, `load-tester` | yes | depends on 1, 3, 4, 5 |
| - | `gromnie-wisp` | lib | yes, later | blocked on `wisp-mux` |
| - | `gromnie-proxy` | bin `gromnie-proxy` | yes, later | blocked on `wisp-mux` |
| - | `gromnie-web` | wasm lib | yes, later | wasm32-only browser bindings, blocked on `wisp-mux` |
| - | `gromnie-ios-bridge` | staticlib | no | `publish = false` |
| - | `gromnie-test-script` | cdylib | no | test fixture, `publish = false` |
| - | `xtask` | bin | no | `publish = false`; name also taken on crates.io |

The order in the table is a valid publication order; crates.io requires a
crate's dependencies to exist before it is published.
`scripts/publish-crates.sh` encodes it, skips versions that are already live,
and waits for the index before publishing the next crate.

## Publishing with trusted publishing

`.github/workflows/publish-crates.yml` publishes on version tags using
crates.io [trusted publishing](https://crates.io/docs/trusted-publishing):
GitHub's OIDC token is exchanged for a short-lived crates.io token by
`rust-lang/crates-io-auth-action`, so no long-lived secret is stored.

One-time bootstrap is required, because **trusted publishing cannot perform a
crate's first publish**:

1. Merge to `main`, then publish 0.1.0 locally with your personal crates.io
   token: `scripts/publish-crates.sh`.
2. For each crate on crates.io, go to Settings -> Trusted Publishing and add
   a GitHub trusted publisher with repository owner `amoeba`, repository
   `gromnie`, workflow filename `publish-crates.yml`, and environment
   `release`.
3. From then on, pushing a version tag publishes the crates automatically.

Until step 1 is done the workflow fails fast with a message pointing here.
Re-running the workflow after a partial failure is safe: already-published
versions are skipped, and it can be started manually via `workflow_dispatch`.

The workflow uses the `release` environment, so protection rules (required
reviewers, deployment branches) can be added in repository settings.

## Version bumps

Internal `path` dependencies also carry `version = "0.1.0"`. When bumping the
workspace version, update those `version` fields in the same commit, or
publication will resolve the previous release from crates.io instead of the
new one.

## Open decisions

1. How to resolve `wisp-mux`, which blocks publishing `gromnie-wisp`,
   `gromnie-proxy`, and `gromnie-web`:
   - get upstream to publish `6.0.0` to crates.io (smallest change here, but
     requires waiting on MercuryWorkshop),
   - vendor/fork the MIT `6.0.0` code into the workspace or a published shim
     crate (no waiting, but we own the code),
   - port to `5.1.0` (rewrite of the transport layer plus an LGPL dependency).
2. Whether to publish `gromnie-ios-bridge`, which is marked `publish = false`
   and builds only a static library; otherwise keep it out of the crates.io
   release.
