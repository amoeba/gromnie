# WASM Scripting Async Migration Plan

## Goal

Convert the Gromnie WASM scripting system from synchronous to fully asynchronous so that scripts can perform long-running I/O operations without blocking the game client.

## Constraints

- Remove sync interface entirely (async only).
- Scripts must be able to perform long-running I/O.
- Use Wasmtime `async_support(true)` and `add_to_linker_async`.
- Match generated `async_trait`-style signatures from `bindgen!({ ..., async: true })`.

## Architecture Decisions

- WIT interface remains sync (`func`) because async is controlled purely by bindgen config, not WIT keywords.
- `async: true` in wasmtime bindgen generates `async_trait`-style methods with explicit `'life0` and `'async_trait` lifetimes returning `Pin<Box<dyn Future...>>`. Implementations must match this exactly.
- `ScriptConsumer` uses a channel (`UnboundedSender<RunnerMessage>`) to bridge the sync `EventConsumer` trait to the async runner task, avoiding `block_in_place`.

## Phases

### Phase 1: Update WIT Interface

Make all host/guest functions async in the WIT interface.

### Phase 2: Flip Host Engine to Async Wasmtime

Files: `engine.rs`, `wasm_script.rs`, `bindings.rs`, `loader.rs`

- Set `config.async_support(true)` in engine config.
- Set `async: true` in `bindgen!` macro.
- Use `add_to_linker_async` instead of `add_to_linker`.
- Use `instantiate_async` instead of `instantiate`.
- Remove `block_in_place` workarounds.
- Convert `loader.rs` to `async fn`, remove `std::thread::spawn` workaround.

### Phase 3: Make ScriptRunner & ScriptConsumer Async

Files: `script_runner.rs`

- Rewrite `ScriptRunner` with a `RunnerMessage` channel system.
- `EventConsumer` dispatches to an async tokio task.
- Event handling, script loading/unloading, and ticking all become async.

### Phase 4: Update Guest API Trait to Async Methods

Files: `lib.rs` (gromnie-scripting-api)

- Update `WasmScript` trait methods to return `Pin<Box<dyn Future<Output = ()> + 'a>>`.
- Methods: `on_load`, `on_unload`, `on_event`, `on_tick`.

### Phase 5: Replace TimerManager with Async Timers

Replace the synchronous `TimerManager` with async tokio-based timers.

### Phase 6: Make ScriptContext Async-Safe

Files: `context.rs`

- Add `client()` (async) method for safe client access across await boundaries.
- Add `client_sync()` for synchronous try_read access.
- Add `client_arc()` to get `Arc<RwLock<Client>>` for use across await boundaries.
- Implement `unsafe impl Send` for `ScriptContext`.

### Phase 7: Update Example Scripts

Update test/example scripts to match the new `WasmScript` trait signatures.

### Phase 8: Build & Fix Compilation Errors

Run `cargo build --workspace` and fix any compilation errors.

### Phase 9: Clean Up Unused Code Warnings

Remove unused methods and imports identified by compiler warnings.

## Key Implementation Patterns

### Generated trait signature for async host imports

```rust
fn foo<'life0, 'async_trait>(
    &'life0 mut self,
    ...
) -> ::core::pin::Pin<
    Box<dyn ::core::future::Future<Output = T> + ::core::marker::Send + 'async_trait>
> where 'life0: 'async_trait, Self: 'async_trait
```

### WasmScript trait signature for guest exports

```rust
fn on_load<'a>(
    &'a mut self,
) -> ::core::pin::Pin<Box<dyn ::core::future::Future<Output = ()> + 'a>>
```

### Guest-side host function calls

From the guest (WASM script) perspective, host functions appear synchronous in the generated bindings. Wasmtime handles async suspension internally via fibers. Scripts call `gromnie::log("msg")` directly without `.await`.

## Relevant Files

- `crates/gromnie-scripting-host/src/wasm/wasm_script.rs` - WASM script instantiation and `HostScript` trait impl
- `crates/gromnie-scripting-host/src/wasm/bindings.rs` - `gromnie::scripting::host::Host` impl for `WasmScriptState`
- `crates/gromnie-scripting-host/src/wasm/engine.rs` - Wasmtime engine configuration
- `crates/gromnie-scripting-host/src/wasm/loader.rs` - Async WASM module loading
- `crates/gromnie-scripting-host/src/script_runner.rs` - Async runner and `ScriptConsumer` channel bridge
- `crates/gromnie-scripting-host/src/context.rs` - Script context with async-safe client access
- `crates/gromnie-scripting-host/src/timer.rs` - Timer management
- `crates/gromnie-scripting-api/src/lib.rs` - `WasmScript` trait definition for script authors
- `crates/gromnie-scripting-api/src/wit/gromnie-script.wit` - WIT interface definition
- `tests/scripting/src/lib.rs` - Test script implementation

## Progress

| Phase | Status |
|-------|--------|
| Phase 1: Update WIT interface | Completed |
| Phase 2: Flip host engine to async wasmtime | Completed |
| Phase 3: Make ScriptRunner & ScriptConsumer async | Completed |
| Phase 4: Update guest API trait to async methods | Completed |
| Phase 5: Replace TimerManager with async timers | Pending |
| Phase 6: Make ScriptContext async-safe | Completed |
| Phase 7: Update example scripts | Completed |
| Phase 8: Build and fix compilation errors | Completed |
| Phase 9: Clean up unused code warnings | Completed |

## Notes

- The `Tick` variant in `RunnerMessage` may be unused and can be cleaned up.
- Per-script hot reload methods (`handle_script_changes`, `reload_script_by_path`, etc.) were removed in favor of full-set reload via `reload_scripts`.
- Host functions in the WIT interface are sync from the guest perspective; async suspension is handled by wasmtime fibers on the host side.
