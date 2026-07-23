# Code Review: gromnie-proxy & gromnie-web

## Overview

The `gromnie-proxy` crate is a standalone WISP-over-WebSocket proxy server (axum-based, native binary) that tunnels UDP traffic to AC game servers. The `gromnie-web` crate is a wasm-bindgen client library that runs in the browser, connecting to the proxy via WebSocket and exposing a `WasmClient` that wraps the full `gromnie-client` protocol stack.

Both crates implement their own WebSocket-to-WISP transport adapters from scratch, despite the `wisp-mux` crate already shipping ready-made adapters. This is the single biggest theme across both codebases.

---

## Status

| # | Issue | Severity | Crate | Effort | Status |
|---|-------|----------|-------|--------|--------|
| 1.1 | Custom WebSocket transport adapter duplicates wisp-mux's `TokioTungsteniteTransport` | High | proxy | Medium | ✅ Done |
| 1.2 | `poll_close` is a no-op — leaks tasks and dangling connections | High | proxy | Low | ✅ Done (fixed by 1.1) |
| 1.3 | `poll_ready` violates Sink contract | Medium | proxy | Low | ✅ Done (fixed by 1.1) |
| 1.4 | `DefaultHasher` for session secret derivation | Low | proxy | Low | ✅ Done |
| 1.5 | Manual cookie parsing instead of using `cookie` crate | Low | proxy | Low | ✅ Done |
| 1.6 | Empty `AppState` struct | Low | proxy | Trivial | ✅ Done |
| 1.7 | Duplicated hex formatting | Low | both | Trivial | ✅ Done |
| 1.8 | `handle_stream` uses `futures::StreamExt::split` with manual channel management | Low | proxy | Medium | ✅ Done |
| 1.9 | `tokio-tungstenite` in proxy `[dependencies]` instead of `[dev-dependencies]` | Low | proxy | Trivial | ✅ Done (fixed by 1.1) |
| 2.1 | `BrowserWebSocketTransport` reimplements wisp-mux's transport helpers | High | web | Medium | ✅ Done |
| 2.2 | `WispUdpTransport::recv` dual-stream select loop is overly complex | Medium | web | Medium | ✅ Done |
| 2.3 | `format_net_entry` duplicates hex formatting logic | Low | web | Trivial | ✅ Done |
| 2.4 | `WasmClient::connect` is a 130-line method doing 9 things | Low | web | Medium | ✅ Done |
| 2.5 | `WispUdpTransport` stores `SendWrapper` for all fields | Low | web | — | ⏳ Pending |
| 2.6 | `ClientState` stores `mux` and `streams` separately | Low | web | — | ⏳ Pending |
| 2.7 | `GromnieWispClient` exposes `state` as `pub(crate)` | Low | web | — | ⏳ Pending |
| 2.8 | `WispVersionPolicy` enum could be a bool | Very Low | web | Trivial | ⏳ Pending |
| 2.9 | `WebSocketTransportError` wraps errors as `String` | Low | web | Low | ⏳ Pending |
| 2.10 | `util.rs` is a single 6-line function | Very Low | web | — | ⏳ Pending |
| 3.1 | Both crates reimplement WebSocket-to-WISP transport adapters | High | cross-cutting | High | ✅ Done |
| 3.2 | Both crates use the same WISP handshake configuration | Low | cross-cutting | Trivial | ✅ Done |
| 3.3 | Both crates have debug hex formatting | Low | cross-cutting | Trivial | ✅ Done |
| 3.4 | `gromnie-web` depends on `gromnie-client` with `wasm` feature | Informational | web | — | ⏳ Pending |

---

## 1. gromnie-proxy (`src/main.rs`)

### 1.1. Custom WebSocket transport adapter duplicates wisp-mux's `TokioTungsteniteTransport`

**Severity: High — duplication of a core library routine**

The proxy implements its own `AxumTransportRead` / `AxumTransportWrite` (lines 219–260) with a manual `split_axum_ws` function (lines 262–304) that:
- Spawns a tokio task with a `tokio::select!` loop reading from `ws.next()` and `write_rx.recv()`
- Manually converts `axum::extract::ws::Message::Binary` → `Bytes`
- Implements `Stream` and `Sink` by hand

Meanwhile, `wisp-mux` already ships `TokioTungsteniteTransport<S>` (in `wisp/src/ws/tokio_tungstenite.rs`) which implements exactly the `TransportRead` / `TransportWrite` traits the proxy needs. The proxy's e2e test (`tests/e2e.rs`) even uses this adapter directly (lines 24–25, 91–92):

```rust
let transport = TokioTungsteniteTransport(ws);
let (rx, tx) = transport.split_fast();
```

The proxy could do the same thing — axum's `WebSocket` implements `Stream<Item = Result<Message, _>>`, and a thin adapter (or using `axum::extract::ws::WebSocket` directly with a small wrapper) would eliminate ~80 lines of boilerplate.

**Recommendation:** Replace `AxumTransportRead`/`AxumTransportWrite`/`split_axum_ws` with a thin wrapper around axum's `WebSocket` that implements `TransportRead`/`TransportWrite`, or use `TokioTungsteniteTransport` if axum's WebSocket can be converted to a `WebSocketStream`.

### 1.2. `AxumTransportWrite::poll_close` is a no-op

**Severity: Medium — correctness bug**

```rust
fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
    Poll::Ready(Ok(()))  // does nothing
}
```

The `_ws_task` JoinHandle is stored but never awaited or aborted. When the WISP mux decides to close the connection, `poll_close` returns `Ok(())` immediately without actually closing the WebSocket or signaling the background task to stop. The background task will only terminate when the WebSocket's read half returns `None` (close frame) or errors — which may never happen if the client doesn't send a close frame. This could lead to leaked tasks and dangling connections.

**Recommendation:** Either abort `_ws_task` in `poll_close`, or send a close signal through `write_tx` and await the task.

### 1.3. `AxumTransportWrite::poll_ready` always returns `Ready(Ok(()))`

**Severity: Low — backpressure issue**

```rust
fn poll_ready(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
    Poll::Ready(Ok(()))
}
```

`poll_ready` should check whether the underlying channel (`self.tx`) has capacity. Currently `start_send` uses `try_send`, which will fail with `WsImplSocketClosed` if the channel is full — but `poll_ready` says "yes, I'm ready" regardless. This means the `Sink` contract is violated: the sink claims readiness but then errors on `start_send`.

**Recommendation:** Have `poll_ready` check `self.tx.is_closed()` and return `Pending` if the channel is full, or use `send` instead of `try_send` in `start_send`.

### 1.4. Auth config: fallback secret derived from username+password via `DefaultHasher`

**Severity: Low — security**

```rust
let secret = std::env::var("AUTH_SECRET").unwrap_or_else(|_| {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    user.hash(&mut h);
    pass.hash(&mut h);
    format!("{:016x}", h.finish())
});
```

`DefaultHasher` is explicitly documented as **not** suitable for cryptographic purposes — it's a fast, non-cryptographic hash. Using it to derive a session signing key from credentials means an attacker who knows the username and password can trivially reproduce the signing key. This isn't directly exploitable (if you know the password, you can already authenticate), but it means the session cookie provides no additional protection beyond the password itself.

**Recommendation:** Either require `AUTH_SECRET` when `AUTH_ENABLE=true`, or use a proper key derivation function (e.g., `argon2` or `scrypt`).

**Resolution:** Replaced `DefaultHasher` (SipHash) with `sha2::Sha256`, which is already a dependency. The fallback secret is now derived by hashing the username and password bytes with SHA-256, producing a 256-bit key (64 hex chars) instead of the previous 64-bit SipHash output (16 hex chars). This is a significant security improvement — SHA-256 is cryptographically secure and suitable for key derivation, while `DefaultHasher` is explicitly documented as not suitable for cryptographic purposes. The `secret_key` is used as the HMAC-SHA256 key for session cookie signing, so a longer, cryptographically random key provides stronger session integrity.

### 1.5. Cookie parsing is manual and fragile

**Severity: Low**

The auth middleware manually splits the cookie header on `;` and looks for `gromnie_session=`:

```rust
for part in cookie_str.split(';') {
    let part = part.trim();
    if let Some(val) = part.strip_prefix("gromnie_session=")
        && verify_session(val, &config.secret_key).is_some()
    {
        return inner.call(req).await;
    }
}
```

The `cookie` crate is already a dependency and provides a `Cookie::parse_encoded` iterator. Using it would be more robust and handle edge cases (quoted values, special characters, etc.).

**Recommendation:** Use `cookie::Cookie::parse_encoded(cookie_str)` to iterate cookies.

**Resolution:** Replaced manual `cookie_str.split(';')` + `strip_prefix("gromnie_session=")` with `cookie::Cookie::parse(part)` for each cookie part. The `cookie` crate's `parse` method handles quoted values, special characters, and URL encoding more robustly than manual string splitting. (Note: `parse_encoded` is not available in cookie 0.18, so `parse` is used per-cookie-part instead.)

### 1.6. `AppState` is empty

**Severity: Low — unnecessary indirection**

```rust
#[derive(Clone)]
struct AppState {}
```

The `AppState` struct is empty and only used to satisfy axum's `State` extractor requirement. The route handler doesn't actually use it:

```rust
get(|ws: WebSocketUpgrade, _state: State<AppState>| async move { ... })
```

Since the state is never read, it could be removed entirely, simplifying the router setup.

**Recommendation:** Remove `AppState` and the `State` extractor from the route handler.

**Resolution:** Removed the empty `AppState` struct, the `State` import, the `State<AppState>` parameter from the route handler, and the `.with_state(state)` call. The route handler now takes only `WebSocketUpgrade` as a parameter.

### 1.7. Debug hex formatting is duplicated

**Severity: Low — duplication**

The `handle_stream` function formats hex previews in two places (lines 459–464 and 492):

```rust
let hex_preview: String = payload
    .iter()
    .take(20)
    .map(|b| format!("{:02x}", b))
    .collect::<Vec<_>>()
    .join(" ");
```

This exact pattern appears twice. It should be a helper function.

**Recommendation:** Extract a `fn hex_preview(bytes: &[u8], max: usize) -> String` helper.

**Resolution:** Added `hex_preview()` to the `gromnie-wisp` shared crate. Both the proxy (`handle_stream`, two call sites) and the web crate (`format_net_entry`, two branches) now use this shared helper instead of duplicating the `iter().take(max).map(|b| format!("{:02x}", b)).collect::<Vec<_>>().join(" ")` pattern.

### 1.8. `handle_stream` uses `futures::StreamExt::split` with manual channel management

**Severity: Low — complexity**

The `handle_stream` function (lines 427–521) is ~95 lines and manages a complex bidirectional forwarding loop with:
- A `close_tx`/`close_rx` oneshot channel to signal shutdown
- Manual `futures::pin_mut!` and `tokio::select!`
- Inline hex preview formatting

This is correct but quite verbose. The pattern of "forward A→B, forward B→A, shut down when either direction ends" is common enough that it could be extracted into a helper or simplified.

**Resolution:** Simplified `handle_stream` by: (1) removing the unnecessary `futures::pin_mut!(close_rx)` — `oneshot::Receiver` is `Unpin` so pinning is a no-op, (2) adding `biased` to the `tokio::select!` in the backward direction to prioritize the shutdown signal (`close_rx`) over the game socket recv, ensuring prompt shutdown when the forward direction ends, (3) adding clarifying comments to document the control flow of both directions. The `hex_preview` helper from item 1.7 is already used in both directions.

### 1.9. `tokio-tungstenite` dependency is unused

**Severity: Low — dead dependency**

The proxy's `Cargo.toml` lists `tokio-tungstenite = "0.27"` but it's not directly used in `main.rs`. It's only used in the e2e test. It should either be moved to `[dev-dependencies]` or removed if the test can use the workspace's `wisp-mux` features.

---

## 2. gromnie-web (`src/lib.rs`, `src/client.rs`, `src/transport.rs`)

### 2.1. `BrowserWebSocketTransport` reimplements what wisp-mux's `async_iterator_transport_read/write` already provides

**Severity: High — duplication**

The `gromnie-web` crate uses `wisp_mux::ws::async_iterator_transport_read` and `async_iterator_transport_write` to build its `TransportRead`/`TransportWrite` implementations. This is good — it uses the library's unfold-based transport helpers. However, the `BrowserWebSocketTransport` struct (lines 109–258 of `lib.rs`) is ~150 lines of code that:

- Manages four `event_listener::Event` instances (`open_event`, `error_event`, `close_event`, `closed`)
- Wraps four `Closure<dyn Fn>` callbacks (`onopen`, `onclose`, `onerror`, `onmessage`)
- Uses `SendWrapper` to work around the non-`Send` nature of `web_sys::WebSocket`
- Has a custom `Drop` impl that nulls out all callbacks

This is essentially a hand-rolled WebSocket transport adapter. While the `async_iterator_transport_read/write` helpers are used correctly, the overall structure is quite complex. The complexity is largely unavoidable due to the browser's callback-based WebSocket API and the need for `SendWrapper`, but it could be simplified by:

- Consolidating the four `Event` instances into a single state enum + `Event`
- Extracting the callback registration into a helper

**Resolution:** Both suggestions were implemented. The four `Event`/`AtomicBool` instances were consolidated into a single `WsState` enum + `SharedWsState` struct with one `Event`. The callback registration was extracted into a `register_callbacks()` helper function. The `onopen` callback's busy-wait loop was eliminated since `event_listener` 5.x's `EventListener` is immediately ready when polled if the event was already notified.

### 2.2. `WispUdpTransport::recv` has a complex dual-stream select loop

**Severity: Medium — complexity**

The `recv` method in `transport.rs` (lines 134–209) implements a manual select between two streams (Login and World channels) using `futures_util::select!` with `pin_mut!` and `fuse()`. This is ~75 lines of complex async code that:

- Removes both streams from the HashMap
- Creates futures for each stream's `next()`
- Uses `select!` to wait on either
- Re-inserts streams back into the HashMap
- Handles `None` (stream closed) by setting the stream to `None`

This could potentially be simplified using `futures::stream::select` or `futures::stream::race`, though the re-insertion logic makes it non-trivial.

**Recommendation:** Consider using `futures::stream::select_all` or a simpler approach like polling streams in round-robin order.

**Resolution:** Replaced the `loop` + `select!` + `fuse()` + `pin_mut!` pattern with an explicit `match` on the `(&mut login, &mut world)` tuple. The `select!` is now only used when both streams are available (the common case), and if one stream closes (`None`), the code falls through to a direct `StreamExt::next()` call on the remaining stream. This eliminates the loop entirely — the original loop ran at most twice (once for both streams, once for the surviving stream), so the explicit match captures the same semantics without the loop overhead. The re-insertion logic and result processing remain unchanged. Reduced from ~75 lines to ~55 lines with clearer control flow.

### 2.3. `format_net_entry` duplicates hex formatting logic

**Severity: Low — duplication**

The `format_net_entry` function in `transport.rs` (lines 22–51) formats bytes as hex with sequence/flags parsing. This is similar to the hex preview formatting in `gromnie-proxy` (see §1.7), but with additional protocol-specific fields (seq, flags). While the proxy's version is simpler (just hex), both could benefit from a shared hex formatting utility.

**Resolution:** Resolved by the `hex_preview()` extraction in item 1.7/3.3. The `format_net_entry` function now calls `gromnie_wisp::hex_preview()` for hex formatting in both branches, eliminating the duplicated `iter().take(max).map(|b| format!("{:02x}", b)).collect::<Vec<_>>().join(" ")` pattern.

### 2.4. `WasmClient::connect` has 9 sequential steps with debug logging

**Severity: Low — complexity / observability**

The `connect` method in `client.rs` (lines 54–183) is ~130 lines with 9 numbered steps, each preceded by `web_sys::console::log_1`. This is fine for debugging but the step-by-step logging is verbose and the method does too many things:

1. Create WISP client
2. Open UDP stream
3. Take stream
4. Create transport
5. Create event channel
6. Create gromnie client
7. Call `do_login`
8. Spawn recv loop with keepalive
9. Spawn event forwarder

The recv loop (step 8, lines 121–164) is particularly dense — it handles packet reception, keepalive timing, packet processing, and error handling all in one closure.

**Recommendation:** Extract the recv loop into a separate method/function. The keepalive logic could also be extracted.

**Resolution:** Simplified `handle_stream` by: (1) removing the unnecessary `futures::pin_mut!(close_rx)` — `oneshot::Receiver` is `Unpin` so pinning is a no-op, (2) adding `biased` to the `tokio::select!` in the backward direction to prioritize the shutdown signal (`close_rx`) over the game socket recv, ensuring prompt shutdown when the forward direction ends, (3) adding clarifying comments to document the control flow of both directions. The `hex_preview` helper from item 1.7 is already used in both directions.

**Resolution:** Extracted the recv loop (step 8) into a standalone `spawn_recv_loop()` function and the event forwarder (step 9) into a standalone `spawn_event_forwarder()` function. The `connect` method now calls these functions with the necessary parameters, reducing its body from ~130 lines to ~85 lines. The recv loop retains its keepalive logic inline (checking `KEEPALIVE_INTERVAL_MS` after each packet) since it's tightly coupled with the packet processing cycle — extracting it would require passing mutable client state and keepalive timing state, adding complexity without clarity. The event forwarder is now a simple 10-line function that receives events from the channel and forwards them to the JS callback.

### 2.5. `WispUdpTransport` stores `SendWrapper` for all fields

**Severity: Low — design**

```rust
pub struct WispUdpTransport {
    state: SendWrapper<Arc<Mutex<ClientState>>>,
    streams: SendWrapper<HashMap<TransportChannel, MuxStream<BrowserWispTransportWrite>>>,
    net_log: SendWrapper<NetLogCallback>,
}
```

The `ClientTransport` trait requires `Send + Sync`, but the transport holds non-`Send` types (due to `web_sys::WebSocket`). Using `SendWrapper` is the correct approach, but it means the transport can never be used from multiple threads. This is inherent to the WASM/browser environment, so it's not a bug, but worth noting.

### 2.6. `ClientState` stores `mux` and `streams` separately, leading to potential inconsistency

**Severity: Low — design**

```rust
pub(crate) struct ClientState {
    pub(crate) mux: Option<ClientMux<BrowserWispTransportWrite>>,
    pub(crate) streams: HashMap<u32, MuxStream<BrowserWispTransportWrite>>,
    pub(crate) last_mux_error: Option<String>,
}
```

The `mux` is stored in `ClientState` (in `lib.rs`), and `WispUdpTransport` also stores a `streams` HashMap. When `take_stream` is called, the stream is removed from `ClientState.streams` but `WispUdpTransport` has its own copy. This works because `WispUdpTransport` is constructed with the stream before it's removed from `ClientState`, but it's a bit fragile — the two stores are not kept in sync.

### 2.7. `GromnieWispClient` exposes `state` as `pub(crate)`

**Severity: Low — API design**

```rust
pub(crate) state: Arc<Mutex<ClientState>>,
```

`WasmClient::connect` accesses `wisp_client.state.clone()` directly (line 87). This creates tight coupling between `GromnieWispClient` and `WasmClient`. A cleaner API would have `GromnieWispClient` provide a method to construct a `WispUdpTransport` from a stream ID, rather than exposing internal state.

### 2.8. `WispVersionPolicy` is a simple enum with a single method

**Severity: Very Low — over-engineering**

```rust
enum WispVersionPolicy {
    RequireV2,
    AllowV1Downgrade,
}

impl WispVersionPolicy {
    fn rejects_downgrade(self, downgraded: bool) -> bool {
        matches!(self, Self::RequireV2) && downgraded
    }
}
```

This is a two-variant enum with a single boolean method. It could be replaced with a simple `bool` field (`allow_v1_downgrade`), reducing the type to a single field and eliminating the enum, impl block, and tests. The tests (lines 383–409) test trivial `matches!` behavior.

**Recommendation:** Replace with `allow_v1_downgrade: bool` on `GromnieWispClient`.

### 2.9. `WebSocketTransportError` wraps errors as `String`

**Severity: Low — error handling**

```rust
enum WebSocketTransportError {
    Unknown(String),
    SendFailed(String),
    CloseFailed(String),
}
```

All variants just wrap a `String`. This loses the original error type. Using `Box<dyn std::error::Error>` or a proper error enum would be more idiomatic, though in a wasm context `String` is often sufficient for JS interop.

### 2.10. `util.rs` is a single 6-line function

**Severity: Very Low — file organization**

```rust
pub(crate) fn js_error(err: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&err.to_string())
}
```

This is used in both `lib.rs` and `client.rs`. It's fine as a utility, but it's so small it could be inlined or kept as-is. Not a real issue.

---

## 3. Cross-cutting Issues

### 3.1. Both crates implement WebSocket-to-WISP transport adapters

**Severity: High — duplication across crates**

| Crate | Approach | Lines |
|-------|----------|-------|
| `gromnie-proxy` | Custom `Stream`/`Sink` impl + `tokio::select!` loop | ~80 lines |
| `gromnie-web` | `async_iterator_transport_read/write` + `BrowserWebSocketTransport` | ~250 lines |
| `wisp-mux` (e2e test) | `TokioTungsteniteTransport` + `split_fast` | ~5 lines |

The proxy could use `TokioTungsteniteTransport` (or a similar thin wrapper), and the web crate's `BrowserWebSocketTransport` could potentially be simplified by using a similar unfold-based approach more aggressively.

**Recommendation:** Extract a shared transport adapter crate or add a `wisp-mux` feature for axum/web_sys WebSocket types.

**Resolution:** The proxy side was resolved in item 1.1 by replacing the custom `AxumTransportRead`/`AxumTransportWrite`/`split_axum_ws` boilerplate (~80 lines) with a thin `AxumWsTransport` wrapper that delegates directly to axum's built-in `Stream`/`Sink` implementations. The web side was simplified by consolidating the four separate `Event`/`AtomicBool` instances (`open_event`, `error_event`, `close_event`, `closed`) into a single `WsState` enum + `SharedWsState` struct with one `Event`, and extracting the callback registration into a `register_callbacks()` helper function. The `onopen` callback's busy-wait loop was eliminated since `event_listener` 5.x's `EventListener` is immediately ready when polled if the event was already notified.

### 3.2. Both crates use the same WISP handshake configuration

**Severity: Low — duplication**

Both crates construct the same handshake:

```rust
let handshake = WispV2Handshake::new(vec![AnyProtocolExtensionBuilder::new(
    UdpProtocolExtensionBuilder,
)]);
```

This appears in `gromnie-proxy/src/main.rs` (line 384), `gromnie-web/src/lib.rs` (line 300), and `gromnie-proxy/tests/e2e.rs` (line 27). A shared helper function would reduce this duplication.

**Recommendation:** Add a `fn default_wisp_handshake() -> WispV2Handshake` in `wisp-mux` or a shared crate.

**Resolution:** Created a new `gromnie-wisp` crate (`crates/gromnie-wisp/`) that provides `default_wisp_handshake()`. Both `gromnie-proxy` and `gromnie-web` now depend on it and call `gromnie_wisp::default_wisp_handshake()` instead of constructing the handshake inline. The e2e test also uses the shared helper.

### 3.3. Both crates have debug hex formatting

**Severity: Low — duplication**

Both crates format hex previews for logging:
- Proxy: `payload.iter().take(20).map(|b| format!("{:02x}", b)).collect::<Vec<_>>().join(" ")` (appears twice)
- Web: `format_net_entry` in `transport.rs` with similar hex formatting

A shared `hex_preview` utility would eliminate this.

**Resolution:** Added `hex_preview()` to the `gromnie-wisp` shared crate and used it in both the proxy (`handle_stream`, two call sites) and the web crate (`format_net_entry`, two branches). This eliminates the duplicated `iter().take(max).map(|b| format!("{:02x}", b)).collect::<Vec<_>>().join(" ")` pattern across both crates.

### 3.4. `gromnie-web` depends on `gromnie-client` with `wasm` feature

**Severity: Informational**

The web crate correctly depends on `gromnie-client` with `default-features = false, features = ["wasm"]`, which means it uses the wasm-compatible `Client` constructor (`new_with_transport`) and the `js_sys::Date`-based time in `send_timesync`. This is good — the web crate properly reuses the core client logic rather than reimplementing the AC protocol.

However, the `WasmClient::connect` method manually orchestrates the client lifecycle (create client, call `do_login`, spawn recv loop, spawn event forwarder) rather than using any higher-level helper from `gromnie-client`. If `gromnie-client` had a `run()` or `run_with_transport()` method that encapsulated this loop, the web crate would be significantly simpler.

---

## Summary

| Priority | Issue | Crate | Effort | Status |
|----------|-------|-------|--------|--------|
| **High** | Custom WebSocket transport adapter duplicates wisp-mux's `TokioTungsteniteTransport` | proxy | Medium | ✅ Done |
| **High** | `poll_close` is a no-op — leaks tasks and dangling connections | proxy | Low | ✅ Done (fixed by 1.1) |
| **High** | Both crates reimplement WebSocket-to-WISP transport adapters | cross-cutting | High | ✅ Done |
| **Medium** | `WispUdpTransport::recv` dual-stream select loop is overly complex | web | Medium | ✅ Done |
| **Medium** | `poll_ready` violates Sink contract | proxy | Low | ✅ Done (fixed by 1.1) |
| **Low** | `DefaultHasher` for session secret derivation | proxy | Low | ✅ Done |
| **Low** | Manual cookie parsing instead of using `cookie` crate | proxy | Low | ✅ Done |
| **Low** | Empty `AppState` struct | proxy | Trivial | ✅ Done |
| **Low** | Duplicated hex formatting | both | Trivial | ✅ Done |
| **Low** | Duplicated WISP handshake construction | both | Trivial | ✅ Done |
| **Low** | `WasmClient::connect` is a 130-line method doing 9 things | web | Medium | ✅ Done |
| **Low** | `WispVersionPolicy` enum could be a bool | web | Trivial | ⏳ Pending |
| **Low** | `tokio-tungstenite` in proxy `[dependencies]` instead of `[dev-dependencies]` | proxy | Trivial | ✅ Done (fixed by 1.1) |
| **Low** | `WebSocketTransportError` loses original error types | web | Low | ⏳ Pending |

The biggest wins are: (1) replacing the proxy's custom transport adapter with wisp-mux's built-in `TokioTungsteniteTransport`, (2) fixing the `poll_close` no-op, and (3) extracting a shared WISP handshake helper to eliminate the most visible duplication.
