# Gromnie WASM Scripts

WebAssembly scripts for use with Gromnie. Each script is compiled to the `wasm32-unknown-unknown` target and can be executed by the Gromnie runtime.

## Scripts

- **hello-world** - A basic hello world script demonstrating script functionality
- **auto-login** - Automated login script for handling authentication
- **file-logger** - File logging script for capturing and persisting application logs

## Building

To build all scripts, run:

```bash
cargo build --release --target wasm32-unknown-unknown -p hello-world-script -p auto-login-script -p file-logger-script
```

The compiled WASM files will be available in `target/wasm32-unknown-unknown/release/`.

## Using Scripts

Load and execute scripts in Gromnie applications by referencing the compiled WASM files from the distribution package.
