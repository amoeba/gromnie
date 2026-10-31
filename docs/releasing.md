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

### Open decisions

1. Which workspace crates should be public? `gromnie-ios-bridge` and `xtask`
   are explicitly private.
2. How to resolve `wisp-mux`:
   - get upstream to publish `6.0.0` to crates.io (smallest change here),
   - vendor/fork `6.0.0` into the workspace or a published shim crate (MIT),
   - port to `5.1.0` (rewrite of the transport layer plus an LGPL dependency),
   - or ship binaries only, and publish just the crates that don't use it.
3. Should public crates share the workspace version and release together, or
   have independent versions?
4. Which crates.io credential/owner should Actions use: repository secret
   `CARGO_REGISTRY_TOKEN` or trusted publishing?

Once these are settled, add the version requirements to internal path
dependencies, validate the selected packages with `cargo publish --dry-run`,
and wire publication into the tag release after dependencies are available.
