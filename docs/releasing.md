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

Crate publication is not enabled yet. Internal path dependencies (for example,
`gromnie-cli` -> `gromnie-client`) need a `version` requirement, which is now
recorded next to each `path`. The WISP crates are explicitly private because
they depend on the unreleased `wisp-mux` 6.0.0 source (details below).

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

One-time bootstrap is required, because **trusted publishing cannot perform a
crate's first publish**. After the PR is merged:

1. Confirm `Cargo.toml` is still at version `0.1.0` and check out the exact
   commit you intend to release.
2. Create a crates.io API token with publish permission. Set it in your local
   shell only (do not add it to GitHub secrets):
   `read -rsp "crates.io token: " CARGO_REGISTRY_TOKEN; echo; export CARGO_REGISTRY_TOKEN`.
3. Run `scripts/publish-crates.sh`. It publishes the seven public crates in
   dependency order and waits for each one to appear on crates.io. Unset the
   token afterwards with `unset CARGO_REGISTRY_TOKEN`.
4. On **each** of the seven published crate pages, open Settings -> Trusted
   Publishing and add a GitHub publisher with owner `amoeba`, repository
   `gromnie`, workflow filename `release.yml`, and environment `release`.
5. Create the GitHub Actions environment named `release` if it does not exist.
   Protection rules are optional.
6. Push the first release tag:
   `git tag v0.1.0 && git push origin v0.1.0`.
   cargo-dist builds the binaries; after CI passes, the release workflow
   authenticates with OIDC and skips the seven versions already published
   during bootstrap. The GitHub Release is created only after this succeeds.

After bootstrap, future releases need only a version bump (including internal
path-dependency version requirements), then pushing the matching `vX.Y.Z` tag.
The publishing workflow first checks that the tag matches every public crate's
version and waits for the repository CI workflow to pass. It is safe to rerun
after partial failure: already-published versions are skipped. It can also be
started manually from the release tag with `workflow_dispatch`.

## Version bumps

Internal `path` dependencies also carry `version = "0.1.0"`. When bumping the
workspace version, update those `version` fields in the same commit, or
publication will resolve the previous release from crates.io instead of the
new one.

## Deferred WISP crates

`gromnie-wisp`, `gromnie-proxy`, and `gromnie-web` stay out of crates.io for
now. If we later want to publish them, either upstream must publish the
compatible `wisp-mux` 6.x release, we must vendor/fork the MIT 6.0.0 code, or
we must port to the older LGPL 5.1.0 API.
