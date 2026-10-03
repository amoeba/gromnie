# Implementation Plan: Headless Client API

> **Note:** this file previously held the iOS client plan (`feat/ios`, shipped in #50).
> That content is recoverable with `git checkout 036725a -- plan.md`.

## Fixed scope

Ship a headless-first Rust API for driving an Asheron's Call client with no UI attached. The
target shape:

```rust
let client = GromnieClient::builder()
    .with_server("play.example.com:9000")
    .with_account("acct", "password")
    .connect()                       // returns at CharacterSelect; errors on failure
    .await?;

let chars = client.list_characters();      // sync, already-known snapshot
client.enter_world("Character Name").await?; // resolves once InWorld
```

This crate is in development, so the API may change freely. The goal is a usable shape, not
backwards compatibility.

## Progress

All six stages are implemented and **verified against the live server** (`play.coldeve.ac`, see
"Live verification" below). `cargo fmt --check`, `cargo clippy --workspace --all-targets
--all-features -- -D warnings`, and `cargo test --workspace` (144 tests) all pass.

- [x] Stage 1: `ClientError` implements `Display` + `std::error::Error`
- [x] Stage 2: move `run_client_loop` from `gromnie-runner` to `gromnie-client` as
      `spawn_client_loop` → `ClientHandle`
- [x] Stage 3: `gromnie-runner` delegates to the moved loop
- [x] Stage 4: publish `Scene` snapshots on a `watch` channel each loop iteration
- [x] Stage 5: facade — builder, `connect`, `list_characters`, `enter_world`, `say`/`tell`,
      `subscribe`, `disconnect`
- [x] Stage 6: unit tests + `examples/hello_world.rs` live smoke test

## Live verification

Run against `play.coldeve.ac:9000` as `treestats` via
`cargo run -p gromnie-client --example hello_world`.

| Scenario | Result |
|---|---|
| Valid credentials | `connect()` → `CharacterSelect` in **1.26s**, 10 characters listed |
| `enter_world("Treestats")` | resolved in **1.31s**, `character_id 1342189415` — the requested character |
| Case-insensitive lookup | `--character treestats` (all lowercase) matched `Treestats` |
| Event stream | 52 events over a 10s hold, zero `Lagged`, scene still `InWorld` at the end |
| `disconnect()` | clean; 13.0s total wall clock |
| Wrong password | `connect()` → `Authentication("...password...not correct")` in 1.11s |
| Unreachable server (`192.0.2.1`, TEST-NET-1) | `connect()` → `Client(LoginTimeout)` in 20.1s |

**Two bugs were found by this testing, and fixed afterwards:**

1. **The final scene was never published on a `break` path.** Scene snapshots were written at the
   tail of each loop iteration, but every `break` skipped that. A client that failed via
   `check_state_timeout` therefore left observers watching a stale `Scene::Connecting` indefinitely.
   The loop now carries its reason out (`break reason`), publishes the final scene, and only then
   records the exit reason — so anyone who can observe an exit reason can also read the final scene.
2. **An unreachable server was reported as an authentication failure.** `pump_events` mapped
   `ClientSystemEvent::AuthenticationFailed` to `ApiError::Authentication`, but that event is
   overloaded: the client emits it both for a real rejection (`message_handlers.rs:153`) and for a
   connect timeout (`client.rs:757`). It now resolves through the scene instead (`Client(LoginTimeout)`),
   because both paths set `Scene::Error`, making the scene authoritative.

**Still unverified:** `say()` / `tell()`. They post real messages into a shared game world, so they
were left alone deliberately. Each is a single `game_action_tx` send on the same channel the
scripting host already uses for chat.

## Deviations from the original plan

Found while implementing. Each is a correction, not a scope change.

1. **The driver module is `client/driver.rs`, not `client/loop.rs`.** `loop` is a Rust keyword, so
   `mod loop;` will not parse.
2. **The facade is `api.rs`, not `api/mod.rs`.** One file is enough at this size.
3. **`ClientError` gets a manual `Display` + `Error`, not `thiserror`.** `ConfigLoadError`
   (`gromnie_config.rs:18`) already established the manual style in this crate; matching it avoids
   adding a dependency nothing else in `gromnie-client` uses.
4. **Blast radius was larger than first claimed.** `gromnie-scripting-host` also holds
   `Arc<RwLock<Client>>` in ~12 places (`context.rs:52,66,103`, `script_runner.rs:42,66,79,100,109,117`,
   `registry.rs`, tests), fed from `client_runner_builder.rs:452`. So `spawn_client_loop` takes the
   `Arc` rather than consuming the `Client` — otherwise the scripting host loses access.
   `run_client_internal`'s signature is unchanged as a result.
5. **Ctrl+C handling stays in the runner.** `gromnie-client` depends on tokio without the `signal`
   feature, and cannot enable it: its tokio features are unconditional and `net` already breaks the
   wasm build (finding 11). The runner bridges either an external shutdown channel or a Ctrl+C
   watcher into the loop's own shutdown channel.
6. **`disconnect()` takes `&self`, not `self`.** `GromnieClient` has a `Drop` impl that shuts the
   loop down, and moving fields out of a `Drop` type is not allowed. `disconnect(&self)` plus the
   `Drop` fallback also removes the footgun of forgetting to call it.
7. **`list_characters()` filters out pending-deletion characters.** Returning them would invite
   `enter_world` calls that always fail with `NoSuchCharacter`.
8. **Added `LoopExit::TaskFailed(String)` and an `ExitGuard` in the loop.** Without it, a panic in
   the driver task left the exit slot `None` and `wait_for_exit()` hung forever — turning a crash
   into a deadlock. This was not in the original plan.

## Decisions made now

1. **`connect()` returns at `Scene::CharacterSelect` and errors on failure.** A failed login is a
   `connect()` error, not a later `list_characters()` error.

2. **Mutations go through channels, never the `Client` lock.** See review finding 2 — the lock is
   held for a full second during the handshake. The facade exposes no `&mut Client` and no
   `Arc<RwLock<Client>>`.

3. **Game-level operations use `GameActionMessage`.** `say`, `tell`, and movement go over
   `game_action_tx`. Lifecycle operations (`enter_world`, `disconnect`) use the narrow
   `ClientCommand` enum owned by the facade. Both are reachable through the one `ClientSender`
   value, which is what finally let `SimpleClientAction` be deleted — see "Explicit non-goals".

4. **The driver loop moves into `gromnie-client`.** The loop has no dependency on the runner's
   event bus (`EventEnvelope`/`EventType` already live in `gromnie-events`; `event_bus.rs:8-11`
   re-exports them), so the move needs no new dependency. Only `client_runner.rs` changed, and its
   public signatures are unchanged. **Correction:** `client_runner.rs` is *not* the sole holder of
   `Arc<RwLock<Client>>` — `gromnie-scripting-host` holds it too, so `spawn_client_loop` accepts the
   `Arc` and leaves that access intact. The TUI never touches the client directly; it only receives
   `action_tx`.

5. **The web and iOS loops are left alone.** They stay divergent for now. Unifying them is a
   behavior change, not a code move (review finding 1).

6. **`reconnect` defaults to `false` in the facade.** `start_reconnection` resets the scene to
   `Connecting` (`client.rs:873`), which silently kills a pending `enter_world` wait.

7. **`UI_DELAY_MS` stays as-is.** See review finding 6. Headless `connect()` still costs ~1.3s, of
   which ~1s is one artificial sleep.

8. **`ApiError` is new and wraps `ClientError`.** `ClientError` is a protocol-level type; builder
   validation errors do not belong in it.

## Review findings

These were established by reading the code before designing. They constrain the implementation.

1. **Three divergent copies of the driver loop exist.** Unifying them changes behavior.

   | Copy | Location | Ownership | Cadence | Notes |
   |---|---|---|---|---|
   | Runner | `client_runner.rs:542` | `Arc<RwLock<Client>>` | 100ms | full reconnect + tick |
   | Web/WASM | `gromnie-web/client.rs:40` | owned `Client` | blocks on recv | **no timeouts, no retry, no idle keepalive** |
   | iOS | `gromnie-ios-bridge/session.rs:194` | owned `Client` | 50ms | own `entering_world_since` watchdog |

   The web copy only checks its keepalive *after a packet arrives*
   (`gromnie-web/client.rs:60-64`), so an idle connection never sends `TimeSync`. Latent bug;
   out of scope, but it is why the loops are not interchangeable.

2. **`process_packet` sleeps 1s while the write lock is held.** `client.rs:1812` awaits
   `instant::sleep(UI_DELAY_MS)` between `ConnectRequest` and `ConnectResponse`, inside a method
   taking `&mut self`. The runner holds `client.write()` across that call
   (`client_runner.rs:588`). Any handle method taking `read()`/`write()` can block ~1s during
   handshake. **This is why decision 2 exists.**

3. **The write lock is contended by design.** `client_runner.rs:581-584` holds `client.write()`
   across a `timeout(100ms, recv_packet)` on every iteration.

4. **Events are dropped silently when the channel is full.** `emit_protocol` (`client.rs:462`)
   and `emit_progress` (`client.rs:467`) use `try_send` on a 256-slot channel
   (`client_runner.rs:334`). The runner always has `EventWrapper` draining it
   (`client_runner.rs:337`). **The facade must own the `mpsc::Receiver` and republish to a
   broadcast**, or the first 256 events vanish and `connect()` may never observe the character
   list. This was already a known issue in the previous plan (see finding: "1,024-slot channel
   fills before the actor drains it").

5. **A panic goes silent once spawned.** `client_runner.rs:563` is a bare
   `panic!("Failed to send initial LoginRequest")`. Today it is loud because
   `run_client_internal` is awaited in the caller's task. Inside `tokio::spawn` it becomes an
   unawaited `JoinError`. Must be converted to `ApiError::ClientPanicked`.

6. **~3s of hardcoded UI delay is nominally in the connect path — but only ~1s actually costs
   wall clock.** `UI_DELAY_MS = 1000` (`constants.rs:9`), applied at `client_runner.rs:556` (before
   `LoginRequest`), `client.rs:1812` (before `ConnectResponse`), `message_handlers.rs:267` (DDD
   response), and `message_handlers.rs:289`. These exist to animate a progress bar. Two corrections,
   both from measuring the real server:
   - The facade passes `Duration::ZERO` for `client_runner.rs:556`, so headless `connect()` skips
     that second entirely.
   - `client.rs:1812` is the only *blocking* sleep (it is awaited inside `process_packet`, which
     holds the write lock). The DDD pause at `message_handlers.rs:267` is a queued *send* delay
     (`with_delay_ms`), which overlaps with the handshake rather than adding to it.

   Measured: `connect()` to `CharacterSelect` takes **1.26s** against `play.coldeve.ac`, of which
   ~1s is the `client.rs:1812` sleep. `connecting.reset()` at `client.rs:1821` re-arms the timeout,
   so this does not consume the 20s login budget.

7. **`ClientError` is not an `Error`.** `scene.rs:83-92` has no `Display` and no
   `std::error::Error`, so a public `Result<_, ClientError>` cannot `?` into `anyhow::Error` or
   `Box<dyn Error>`. The crate already depends on `thiserror` (`gromnie_config.rs:28`).

8. **`enter_world().await` can hang if it only watches for `InWorld`.** Completion depends on
   `entering_world` surviving until `LoginCreatePlayer`; the handler no-ops with a warning
   otherwise (`message_handlers.rs:45`). Also `attempt_character_login` rejects re-entry with
   `"Login already in progress"` (`client.rs:438`). A timeout is mandatory.

9. **Reconnect erases the scene being waited on.** `start_reconnection` resets
   `scene = Scene::Connecting` (`client.rs:873`); `enter_disconnected` clears
   `session.connection` (`client.rs:905`). Hence decision 6.

10. **WASM cannot share the loop.** The runner loop uses `tokio::time::sleep` /
    `tokio::time::Instant`, but `crate::instant::sleep` is a deliberate no-op on `wasm32`
    (`instant.rs:107-116`). `client::driver` is therefore `#[cfg(not(target_arch = "wasm32"))]`,
    and the web crate keeps its own loop.

11. **Pre-existing: the wasm build is already broken, independent of this work.** `cargo build -p
    gromnie-web --target wasm32-unknown-unknown` fails with ~48 errors from `mio`. Root cause:
    `gromnie-client`'s tokio dependency enables `"net"` unconditionally
    (`gromnie-client/Cargo.toml:47`), and mio refuses to compile for `wasm*-unknown-unknown`.
    Verified identical failure at `HEAD` (`036725a`) in a clean worktree, so this plan neither
    caused nor fixed it. Fixing it means moving `"net"` behind the `native` feature, or dropping
    it if `tokio::net` turns out to be unused. **This means `crates/gromnie-web/agents.md` is
    currently stale** — the documented `cargo xtask web build` cannot succeed as written.

## Existing Gromnie API used

| Need | API | Location |
|---|---|---|
| Construct client | `Client::new_with_reconnect` | `client.rs:155` |
| Custom transport (web/WASM) | `Client::new_with_transport` | `client.rs:184` |
| Start handshake | `Client::do_login` | `client.rs:1888` |
| Send game action, no lock | `Client::game_action_tx` | `client.rs:95` |
| Serialize a game action | `Client::queue_game_action` | `client.rs:600` |
| Drain game actions in loop | `Client::process_game_actions` | `client.rs:619` |
| Request world entry | `Client::attempt_character_login` | `client.rs:420` |
| Read current scene | `Client::scene` (pub field) | `client.rs:81` |
| Match character select | `Scene::as_character_select` | `scene.rs:223` |
| Character list snapshot | `CharacterSelectScene::characters` | `scene.rs:62` |
| Scene snapshots | `Scene` and sub-scenes derive `Clone` | `scene.rs:33-81` |
| Observe auth failure | `ClientSystemEvent::AuthenticationFailed` | `client_events.rs:15` |
| Auto-login reference semantics | `character` field handler | `message_handlers.rs:211-239` |
| Loop being moved | `run_client_loop` | `client_runner.rs:542` |
| Sendability for spawning | `ClientTransport: Send + Sync` | `transport.rs:18` |

### `SimpleClientAction` status

**Removed** in `refactor/remove-simple-client-action` (issue #60). `simple_client_actions.rs` is
deleted and `ClientAction`/`types.rs` went with it.

The split was: `action_rx` drained by `process_actions`, versus `game_action_tx` drained by
`process_game_actions`. That collapsed into `ClientSender` (`client/command.rs`), which bundles
both channels so an embedder holds one clonable value:

| Method | Underlying channel |
|---|---|
| `say`, `tell`, `do_movement_command`, `stop_movement_command`, `send_game_action` | `game_action_tx` → `GameActionMessage` |
| `enter_world`, `send_login_complete`, `disconnect` | `commands_tx` → `ClientCommand` |

The old variants mapped as:

| Old variant | Fate |
|---|---|
| `SendChatSay` | `ClientSender::say` → `CommunicationTalk` |
| `SendChatTell` | `ClientSender::tell` → `CommunicationTalkDirectByName` |
| `DoMovementCommand` | `ClientSender::do_movement_command` |
| `StopMovementCommand` | `ClientSender::stop_movement_command` |
| `LoginCharacter` | `ClientSender::enter_world` → `ClientCommand::EnterWorld` |
| `SendLoginComplete` | `ClientCommand::SendLoginComplete` |
| `Disconnect` | `ClientCommand::Disconnect` |
| `ReloadScripts` | dropped — the client already warned it did not belong there |
| `LogScriptMessage` | `ScriptContext::log_script_message`, straight to the `script` tracing target |

Two side effects worth knowing:

- `pending_auto_login` is gone. Auto-login used to stash the action on the client and pick it up
  on the next loop iteration; `message_handlers.rs` now calls `attempt_character_login` inline,
  since the scene has already transitioned to `CharacterSelect`.
- `ConsumerContext`/`ConsumerFactory` moved from `gromnie-events` to `client/consumer.rs`, because
  the context hands out a `ClientSender` and `gromnie-client` depends on `gromnie-events`, not the
  reverse. `gromnie-runner` still re-exports both names.

## Design

As built:

```text
crates/gromnie-client/src/
  client/driver.rs  ← the loop, moved from gromnie-runner + handle plumbing
  api.rs            ← GromnieClient, GromnieClientBuilder, ApiError
```

### Handle

```rust
pub struct ClientHandle {
    commands:     mpsc::UnboundedSender<ClientCommand>,     // lifecycle mutations; never the lock
    game_actions: mpsc::UnboundedSender<GameActionMessage>, // cloned from Client::game_action_tx
    scene:        watch::Receiver<Scene>,                   // Scene: Clone
    exit:         watch::Receiver<Option<LoopExit>>,
    shutdown:     watch::Sender<bool>,
    join:         JoinHandle<()>,
}
```

No `broadcast::Receiver` — event fan-out is the embedder's business, so the runner keeps its
existing `EventWrapper` wiring untouched and the facade does its own republish.

`exit` is a `watch` rather than a `Mutex<Option<..>>` so `wait_for_exit()` cannot miss a wakeup, and
`ExitGuard` guarantees the slot is never left `None` after the task ends (see deviation 8).

The loop publishes `Scene::clone()` to the watch channel at the tail of each iteration, after the
`select!`, so every non-break path is covered by one line. `send_replace` is synchronous and
non-blocking, so it adds no await while the client lock is held.

Internal control flow uses the scene watch and the exit slot, **not** the broadcast, so a
lagging subscriber cannot wedge `connect()` on a chatty server.

### Facade

```rust
impl GromnieClient {
    pub fn builder() -> GromnieClientBuilder;

    pub fn list_characters(&self) -> Vec<CharacterIdentity>;      // cached snapshot
    pub fn character(&self, name: &str) -> Option<&CharacterIdentity>;
    pub async fn enter_world(&self, name: &str) -> Result<InWorldScene, ApiError>;
    pub async fn say(&self, msg: impl Into<String>) -> Result<(), ApiError>;
    pub async fn tell(&self, to: &str, msg: impl Into<String>) -> Result<(), ApiError>;
    pub fn subscribe(&self) -> broadcast::Receiver<ClientEvent>;
    pub fn scene(&self) -> Scene;                                 // borrowed, cheap
    pub async fn disconnect(&self) -> Result<(), ApiError>;
}
```

`list_characters()` is synchronous because the data is already in the `Scene`; it returns the
snapshot captured during `connect()`. It cannot be a `const` — it returns a heap `Vec`. It also
filters pending-deletion characters (deviation 7).

### `connect()`

1. Validate builder fields → `ApiError::MissingServer` / `MissingAccount` / `MissingPassword`
2. `Client::new_with_reconnect`, with `reconnect: false`
3. `set_login_timeout(...)` if configured
4. `spawn_client_loop`, which owns the only shutdown channel — so the facade never hijacks
   Ctrl+C (contrast `client_runner.rs:687`)
5. The loop calls `do_login()` on its first turn; the facade does not
6. `select!` until `Scene::CharacterSelect`

| Condition | Signal | `ApiError` |
|---|---|---|
| `Scene::CharacterSelect` | scene watch | returns `Ok`, caches `characters` |
| bad password / account booted | `AuthenticationFailed` (`message_handlers.rs:153`) | `Authentication(reason)` |
| `Scene::Error(ClientError)` | scene watch | `Client(ClientError)` |
| `check_state_timeout` broke the loop (`client.rs:754`) | exit watch | `LoopExit::Failed` → `LoopStopped` |
| reconnect unavailable | exit watch | `LoopStopped(ReconnectUnavailable)` |
| `Disconnected { will_reconnect: false }` (`client.rs:920`) | terminal slot | `ConnectionLost` |
| initial `LoginRequest` failed | exit watch | `LoopStopped(LoginRequestFailed)` |
| task panicked / cancelled | `ExitGuard` → exit watch | `LoopStopped(TaskFailed(..))` |
| `connect_timeout` elapsed (default 30s) | timer | `ConnectTimeout` |

All waits `select!` on the scene watch, the terminal slot, and a deadline. The terminal slot is a
`watch` holding the *first* terminal error, so a later, less specific failure cannot mask the
original cause.

### `enter_world(name)`

1. Look up in the cached list — case-insensitive, skipping `seconds_greyed_out > 0`, matching
   the existing auto-login semantics at `message_handlers.rs:211-239`
2. Miss → `ApiError::NoSuchCharacter`
3. Send `ClientCommand::EnterWorld` over the unbounded sender
4. Await `Scene::InWorld` with matching `character_id`; `Scene::Error`, a terminal error, or a
   loop exit aborts early; timeout required (review finding 8)
5. Return `InWorldScene`

The constructor's `character` auto-login field is deliberately **not** reused: it only fires on
`LoginLoginCharacterSet`, so it cannot switch characters after connect. It remains available as
`with_character()` for callers that know the character up front.

## Implementation stages

All complete. Notes on what actually shipped:

**Stage 1 — `ClientError` as an error type.** Manual `Display` + `std::error::Error` (deviation 3).

**Stage 2 — move the loop.** `client/driver.rs` holds `spawn_client_loop(Arc<RwLock<Client>>,
Duration) -> ClientHandle`, wrapping the body of `run_client_loop` with `gromnie_client::` paths
rewritten to `crate::`. The `panic!` at `client_runner.rs:563` became
`LoopExit::LoginRequestFailed`. A `ClientCommand` channel is drained at the top of each iteration,
alongside — not instead of — `process_actions()`, so legacy `SimpleClientAction` consumers keep
working unchanged. (Superseded: `process_actions()` is now `drain_commands()` and
`SimpleClientAction` is gone — see "`SimpleClientAction` status".)

**Stage 3 — delegate from the runner.** `run_client_loop` is now ~50 lines of signal wiring and
delegation. `create_client_from_config` still returns `(Client, action_tx)` and every public
`run_client*` / `run_multi_client` / `ClientRunner` signature is unchanged.

**Stage 4 — scene publication.** Publish `Scene::clone()` each iteration to the watch channel.

**Stage 5 — the facade.** Builder + `connect` + `list_characters` + `enter_world` + `say`/`tell`
+ `subscribe` + `disconnect`, plus `ApiError`. The facade owns the `mpsc::Receiver<ClientEvent>` and
republishes to a broadcast (review finding 4), which is what makes `try_send` lossless.

**Stage 6 — tests.** 13 unit tests: builder validation, `ApiError` messages, character lookup
(case-insensitivity, pending-deletion skip), `with_server_addr` formatting, `Send`-ness of the
facade, terminal-error first-wins and late-waiter resolution, and the `ExitGuard` fail-safe.

**Not done: the README example.** The doc comment on `api.rs` carries a compiling `no_run`
example instead, which is better placed and cannot rot unnoticed. A README section is still worth
adding.

## Explicit non-goals

- **`list_objects()` / any world-object model.** Deferred at the user's request. There is no
  object state in `gromnie-client` at all; `ItemCreateObject` only emits a protocol event
  (`message_handlers.rs:51`). The model lives in `gromnie-tui/src/object_tracker.rs` and is fed
  from the event bus in `gromnie-tui/src/app.rs:560`. Doing this later means moving `ObjectTracker`
  to a shared crate and updating it inside the client.
- **Unifying the web and iOS loops.** Review finding 1.
- **Removing or shortening `UI_DELAY_MS`.** Review finding 6; load-bearing for TUI progress.
- **Reconnect support in the facade.** Decision 6.
- **WASM support for the facade.** Review finding 10; `gromnie-web` keeps its own loop.
- **Character creation.** `CharacterCreateScene` remains a stub (`scene.rs:66-69`).
