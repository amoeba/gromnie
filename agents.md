# Agent Instructions

This file contains instructions for AI agents working in the gromnie repository.

## Pre-commit checks

This repository uses [prek](https://github.com/j178/prek) (a fast Git hook
manager) to automatically run `cargo fmt` and `cargo clippy` — plus a few
zero-setup hygiene hooks — on every commit. The configuration lives in
[`.pre-commit-config.yaml`](.pre-commit-config.yaml).

If you haven't installed prek yet:

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/j178/prek/releases/latest/download/prek-installer.sh | sh
```

Then install the hooks (one-time setup):

```bash
prek install
```

The hooks will now run automatically on every `git commit`. To run them
manually on all files:

```bash
prek run --all-files
```

If a hook modifies files (e.g. `cargo fmt`, `end-of-file-fixer`), re-stage
the changes and commit again:

```bash
git add -u && git commit --amend --no-edit
```

### Manual fallback

If prek is unavailable, run the checks manually:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
```

If either fails, fix the issues and re-run until both pass:

```bash
cargo fmt          # auto-format
cargo clippy --all-targets --all-features --fix --allow-dirty
```

## Crate-specific notes

- **gromnie-web** — see [`crates/gromnie-web/agents.md`](crates/gromnie-web/agents.md) for WASM build and browser testing instructions.
