# Releasing

The repository uses cargo-dist 0.30.3. Its GitHub Actions release workflow builds
and attaches archives/installers for `gromnie`, `gromnie-cli`, and
`gromnie-proxy` to a GitHub Release. The workflow also runs `dist plan` on pull
requests, so a PR checks that the release configuration is valid.

After merging a release-ready change, push a version tag from that commit:

```sh
git tag vX.Y.Z
git push origin vX.Y.Z
```

Replace `X.Y.Z` with a new version matching the workspace (`Cargo.toml`), and update
the manifests before creating the tag. Do not recreate an existing release tag. A pushed version tag starts the release workflow; the
workflow creates the GitHub Release and uploads the cargo-dist artifacts.

## crates.io publication status

The release workflow publishes the public crates using trusted publishing.
The `gromnie` package replaces `gromnie-tui`; its initial publish and trusted-publisher
setup are described below. Existing `gromnie-tui` releases remain available; this workspace
no longer publishes that package. The WISP crates remain private because they
depend on unreleased `wisp-mux` 6.0.0 source (details below).

### External dependencies

| Dependency | Pinned by gromnie | Latest on crates.io | Status |
| --- | --- | --- | --- |
| `asheron-rs` | `=0.2.0` (registry) | `0.2.0` | Resolved |
| `wisp-mux` | git rev `0c11678` (6.0.0) | `5.1.0` (2024) | Used only by private crates |

`asheron-rs` is published and gromnie now depends on the registry release.

`wisp-mux` 6.0.0 is **not** on crates.io, and the upstream repo does not have a
crates.io publish step in CI (its release workflow publishes the npm package
only). Gromnie depends on the 6.0.0 API, which is a rewrite compared to 5.1.0.
To avoid making that an obstacle to publishing the main crates, `gromnie-wisp`,
`gromnie-proxy`, and `gromnie-web` are marked `publish = false` for now.

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
`gromnie`, `gromnie-cli`, `gromnie-scripting-api`, and
`gromnie-scripting-host` do not depend on `wisp-mux` and have versioned internal
path dependencies for crates.io publication.

## What gets published

Public crates share the workspace version and are released together.

| # | Crate | Kind | Publish | Notes |
| --- | --- | --- | --- | --- |
| 1 | `gromnie-events` | lib | yes | `asheron-rs` only |
| 2 | `gromnie-scripting-api` | lib | yes | WIT interface crate |
| 3 | `gromnie-client` | lib | yes | depends on 1 |
| 4 | `gromnie-scripting-host` | lib | yes | depends on 1-3 |
| 5 | `gromnie-runner` | lib | yes | depends on 1, 3, 4 |
| 6 | `gromnie` | lib + bin `gromnie` | yes | depends on 1, 3, 4, 5 |
| 7 | `gromnie-cli` | lib + bins `discord-bot`, `load-tester` | yes | depends on 1, 3, 4, 5 |
| - | `gromnie-wisp` | lib | no | `publish = false`; relies on unreleased `wisp-mux` 6.0.0 |
| - | `gromnie-proxy` | bin `gromnie-proxy` | no | `publish = false`; depends on `gromnie-wisp` |
| - | `gromnie-web` | wasm lib | no | `publish = false`; browser bindings depend on `gromnie-wisp` |
| - | `gromnie-ios-bridge` | staticlib | no | `publish = false` |
| - | `gromnie-test-script` | cdylib | no | test fixture, `publish = false` |
| - | `xtask` | bin | no | `publish = false`; name also taken on crates.io |

The order in the table is a valid publication order; crates.io requires a
crate's dependencies to exist before it is published.
`scripts/publish-crates.sh` encodes it, skips versions that are already live,
and waits for the index before publishing the next crate.

## Publishing with trusted publishing

`.github/workflows/release.yml` publishes on version tags using crates.io
[trusted publishing](https://crates.io/docs/trusted-publishing). The crates
job runs after CI and must succeed before cargo-dist creates the GitHub Release:
GitHub's OIDC token is exchanged for a short-lived crates.io token by
`rust-lang/crates-io-auth-action`, so no long-lived secret is stored.

### First publish of the renamed application

**Trusted publishing cannot perform a crate's first publish.** Before the next
tagged release:

1. Check out the merged commit intended for publication.
2. Run `cargo login` locally, then `cargo publish -p gromnie`. Its declared library
   dependencies must already be published; use `scripts/publish-crates.sh` if a
   version bump also requires publishing those dependencies first.
3. On the `gromnie` crate page, open Settings -> Trusted Publishing and add the
   GitHub publisher: owner `amoeba`, repository `gromnie`, workflow `release.yml`,
   environment `release`.

The existing library and utility crates already have trusted publishers.
Do not put a long-lived crates.io token in GitHub secrets.

### Subsequent releases

Bump the workspace version and internal dependency requirements, then push a
matching, unused `vX.Y.Z` tag. The workflow checks that every public crate exists,
verifies its version against the tag, and waits for CI before publishing.
The publish script skips versions already on crates.io, so it is safe to rerun
after a partial failure.

## Version bumps

Internal `path` dependencies also carry explicit `version` requirements. Update
those in the same commit as the workspace version, or publication may resolve
an older release from crates.io.

## Deferred WISP crates

`gromnie-wisp`, `gromnie-proxy`, and `gromnie-web` stay out of crates.io for
now. If we later want to publish them, either upstream must publish the
compatible `wisp-mux` 6.x release, we must vendor/fork the MIT 6.0.0 code, or
we must port to the older LGPL 5.1.0 API.
