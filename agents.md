# Agent Instructions

This file contains instructions for AI agents working in the gromnie repository.

## Pre-commit / pre-push checks

Before committing or pushing any changes, always run:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
```

If either fails, fix the issues and re-run until both pass:

```bash
cargo fmt          # auto-format
cargo clippy --all-targets --all-features --fix --allow-dirty
```

## Crate-specific notes

- **gromnie-web** — see [`crates/gromnie-web/agents.md`](crates/gromnie-web/agents.md) for WASM build and browser testing instructions.
