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

Crate publication is not enabled yet. A dry-run of `cargo publish --workspace`
currently fails because workspace path dependencies (for example,
`gromnie-cli` -> `gromnie-client`) do not declare a version requirement. Some
workspace crates also depend on Git-sourced dependencies, which need to be
available from crates.io (or otherwise made publishable) before those crates
can be published there. The release workflow therefore must not attempt a
crates.io publish yet; doing so would fail after a tag is pushed.

Before adding tag-triggered crates.io publication, decide:

1. Which workspace crates are intended to be public on crates.io (libraries,
   binaries, or both). `gromnie-ios-bridge` and `xtask` are explicitly private.
2. Whether Git dependencies such as `asheron-rs` and `wisp-mux` should be
   published first, replaced with crates.io dependencies, or excluded by
   narrowing the public crate set.
3. Whether all public crates should share the workspace version and be published
   together, or have independent versions/releases.
4. Which crates.io publishing credential/owner should be used. The eventual
   release workflow will need a repository secret such as
   `CARGO_REGISTRY_TOKEN` (or an approved trusted-publishing setup).

Once these are settled, add the version requirements to internal path
dependencies, validate the selected packages with `cargo publish --dry-run`,
and wire publication into the tag release after dependencies are available.
