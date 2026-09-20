# Implementation Plan: Minimal iOS Gromnie Client

## Fixed scope

Ship an iPhone/iPad SwiftUI app whose only gameplay capability is chat. Its complete v1 flow is:

```text
Connect form → character list → select character → chat transcript + composer → disconnect
```

The app accepts a hostname (or IPv4 address), port, account name, and password. It uses the existing Rust Gromnie client to communicate directly with the AC server over UDP. It does not render the world, move the player, create characters, handle inventory, reconnect automatically, or stay connected in the background.

Swift is not a reimplementation of Gromnie. Rust retains all AC protocol parsing, packet sequencing, cryptography, UDP, login, character selection, and chat transmission. SwiftUI owns only presentation, user input, app lifecycle, and secure preferences.

## Progress

- [x] Added the `gromnie-ios-bridge` workspace crate as a `staticlib`.
- [x] Added the opaque C session API, Rust-owned session actor, serialized event queue, and C header/configuration scaffold.
- [x] Connected `LoginCharacter`, `SendChatSay`, and `Disconnect` to the existing Rust client and mapped its character, login, chat, and error events.
- [x] Added local ABI/input-validation tests and verified formatting, tests, and Clippy for the bridge.
- [x] Added the missing Tokio `net` and `time` features required by `gromnie-client`'s existing native UDP code.
- [x] Verified the bridge's minimal `gromnie-client` dependency build and added `cargo xtask ios build-core` to generate the header and XCFramework.
- [x] Installed `aarch64-apple-ios`, `aarch64-apple-ios-sim`, `x86_64-apple-ios`, and `cbindgen` 0.29.4.
- [x] Ran the packaging command through header generation; the checked-in C header is reproducible.
- [x] `cargo check` passes for all three iOS targets without linking.
- [ ] Install full Xcode and select it with `xcode-select`, then run and validate the remaining XCFramework build steps.
- [ ] Add the Swift/Xcode wrapper and three SwiftUI screens after an iOS artifact can be produced.

## Decisions made now

| Question | v1 decision |
| --- | --- |
| Protocol implementation | Link the existing Rust `gromnie-client`; no Swift protocol implementation. |
| Transport | Direct UDP, no WISP/WebSocket proxy. |
| IP support | IPv4 only. The current `NativeUdpTransport` binds `0.0.0.0` and client address parsing is not IPv6-safe. Reject IPv6 literals and document this limitation. |
| FFI | A small C ABI plus handwritten Swift wrapper; do not introduce UniFFI. This keeps artifact production, threading, and callback ownership explicit. |
| Rust artifact | `staticlib` packaged into `GromnieCore.xcframework`, with a generated C header/module map. |
| Event delivery | Swift polls a Rust-owned FIFO event queue on a dedicated serial queue. Rust never invokes arbitrary Swift callbacks. |
| One session | Exactly one active Rust session per app process. Connect while active returns a local `already_connected` error. |
| UI framework | SwiftUI, iOS 17 minimum, portrait and landscape supported. |
| Credential storage | Host, port, username: `UserDefaults`. Password: Keychain only after an explicit “Save password” toggle; the default is off. |
| Backgrounding | Send disconnect, stop the actor, and return to the form. No background network entitlement or reconnecting. |
| Outgoing display | Do not local-echo a sent message. Show it only when received from the server's chat event. |
| Distribution | Debug/device and TestFlight beta only in v1; App Store submission is out of scope. |

## Existing Gromnie API used

The bridge uses `gromnie_client::Client` and its native UDP transport. It does not copy the browser client: browser code needs WISP because browsers lack UDP, while iOS can use native UDP.

The implementation consumes only these current `gromnie-events` variants:

| Rust event | Bridge event / UI action |
| --- | --- |
| `ClientSystemEvent::ConnectingStarted` | `.connecting` |
| `SimpleGameEvent::CharacterListReceived { characters, .. }` | `.characters` and character-list screen |
| `ClientStateEvent::EnteringWorld` | `.enteringWorld` spinner |
| `SimpleGameEvent::LoginSucceeded { character_id, character_name }` | `.enteredWorld`; present chat |
| `ClientStateEvent::InWorld` | internal confirmation only; it must follow `LoginSucceeded` before accepting chat input |
| `SimpleGameEvent::ChatMessageReceived { message, message_type }` | `.chat` |
| `SimpleGameEvent::LoginFailed` or `CharacterError` | `.error` and return to character list or form as applicable |
| `ClientSystemEvent::AuthenticationFailed` | `.error`; return to form |
| `ClientSystemEvent::Disconnected` | `.disconnected`; return to form |

`SimpleClientAction::LoginCharacter`, `SendChatSay`, and `Disconnect` are the only Rust actions exposed by v1. The bridge supplies the account name and selected character ID to `LoginCharacter`; it never exposes arbitrary game actions.

## Rust bridge design

### New crate and files

Add workspace crate `crates/gromnie-ios-bridge`:

```text
crates/gromnie-ios-bridge/
  Cargo.toml                 # crate-type = ["staticlib"]
  src/lib.rs                 # opaque C handle and exported ABI
  src/session.rs             # session actor / Gromnie client loop
  src/event.rs               # serializable, bridge-owned event records
  include/gromnie_ios.h      # generated by cbindgen; committed
  cbindgen.toml
```

It depends on `gromnie-client`, `gromnie-events`, `serde`, `serde_json`, `tokio`, and `thiserror`. No Apple-specific code is added to `gromnie-client`.

### ABI

Use an opaque `gromnie_session_t`; it is allocated by Rust and may be used only from the Swift bridge's one serial `DispatchQueue`. A null handle is invalid. All functions return an integer result code; `0` means success, nonzero means a local API error.

```c
typedef struct gromnie_session_t gromnie_session_t;

gromnie_session_t *gromnie_session_create(void);
int32_t gromnie_session_connect(gromnie_session_t *,
                                const char *host_utf8,
                                uint16_t port,
                                const char *username_utf8,
                                const char *password_utf8);
int32_t gromnie_session_select_character(gromnie_session_t *, uint32_t character_id);
int32_t gromnie_session_send_chat(gromnie_session_t *, const char *message_utf8);
int32_t gromnie_session_next_event(gromnie_session_t *, uint32_t timeout_ms,
                                   uint8_t **json_utf8, size_t *json_len);
void gromnie_buffer_free(uint8_t *json_utf8, size_t json_len);
int32_t gromnie_session_disconnect(gromnie_session_t *);
void gromnie_session_destroy(gromnie_session_t *);
const char *gromnie_result_message(int32_t code);
```

Rules:

- `connect` validates input then starts the actor and returns immediately. Network success/failure is reported as an event, not a blocking FFI result.
- `next_event` blocks for at most `timeout_ms`; `0` means no queued event, otherwise Rust allocates an exact UTF-8 JSON buffer. Swift must call `gromnie_buffer_free` exactly once.
- `disconnect` requests shutdown and waits for the actor to stop. The actor polls UDP with a 50 ms timeout, so a normal disconnect completes on the next poll. The Swift wrapper must call it only from its core queue, never the main actor. A future hard timeout requires a nonblocking worker-completion design and is not claimed by v1.
- `destroy` is valid after any outcome, is never called on the main thread, and releases the handle only after issuing disconnect if necessary.
- All C strings must be valid UTF-8, NUL-free, and at most 255 bytes for host/account and 512 bytes for password/chat. Invalid input returns `invalid_argument` and is never logged.
- Bridge code catches panics at every ABI boundary and returns `internal_error`; no panic may cross C/Swift.

### Actor and client loop

`connect` starts one named Rust thread. That thread creates a multi-thread Tokio runtime, constructs `Client::new(..., reconnect = false)`, calls `do_login()`, and solely owns that `Client` until shutdown. The handle communicates with it through bounded command and event queues; it never accesses `Client` directly.

The actor uses a 1,024-item command queue and a 4,096-item event queue. Commands are `SelectCharacter`, `SendChat`, and `Disconnect`. It must:

1. Receive UDP datagrams and call `recv_packet`, `process_packet`, `process_messages`, `process_actions`, `process_game_actions`, and `send_pending_messages` in that order.
2. On a command, enqueue the corresponding existing `SimpleClientAction`, then run `process_actions`, `process_game_actions`, and `send_pending_messages` immediately.
3. Send `send_keepalive()` every five seconds while connected.
4. Forward the `ClientEvent` channel in source order into bridge events.
5. Emit exactly one terminal `.disconnected` event, close sockets/channels, and exit on disconnect, fatal socket error, or actor shutdown.

Chat events are never dropped: when the event queue is full, actor progress pauses until Swift drains it. This is deliberate for a chat-only client; it preserves ordering and avoids silent message loss. UI-only progress events may be coalesced before entering the queue. The bridge JSON includes monotonic `sequence` and Unix-millisecond `timestamp` fields so Swift can detect a programming error in ordering.

### JSON event schema

`next_event` returns one object, never an array:

```json
{"sequence":42,"timestamp_ms":1735689600000,"type":"characters","characters":[{"id":123,"name":"A Character"}]}
```

Allowed `type` values and required fields are fixed:

- `connecting`
- `characters`: `characters: [{ id: u32, name: string }]`, `account: string`, `slots: u32`
- `entering_world`: `character_id: u32`
- `entered_world`: `character_id: u32`, `character_name: string`
- `chat`: `message: string`, `message_type: u32`
- `error`: `code: string`, `message: string`, `recover_to: "form" | "characters"`
- `disconnected`: `reason: string`, `user_initiated: bool`

Rust, not Swift, selects `recover_to`: bad credentials and transport/login failures go to `form`; a character-specific failure goes to `characters`. Raw protocol event debug strings, passwords, packet bytes, and server addresses are never emitted.

## Swift application design

### Project layout and linking

Create `ios/Gromnie/` containing `Gromnie.xcodeproj`, the `GromnieCore.xcframework`, and a Swift `GromnieCore` wrapper target. Xcode links the framework and imports its module map; app views import only the Swift wrapper, never the C header.

The Swift wrapper owns one `OpaquePointer`, executes every C call on `DispatchQueue(label: "net.gromnie.core")`, and exposes an `AsyncStream<BridgeEvent>`. A single task repeatedly calls `next_event` with a 250 ms timeout, decodes JSON with `JSONDecoder`, frees the Rust buffer in `defer`, and yields typed events. It stops before destroy. No FFI call runs on the main actor.

`@MainActor final class SessionViewModel: ObservableObject` is the sole app state owner. It has:

```swift
enum Screen { case form, characters, enteringWorld, chat }
enum ConnectionStatus { case idle, connecting, error(String), disconnected(String) }
```

It holds a selected character, character list, up to 1,000 chat lines, draft message, status, and the wrapper. At 1,000 lines it removes the oldest 100 before appending more. It starts the event task immediately after `connect`; it cancels it only after the terminal event or explicit teardown.

Views are fixed as follows:

- `ConnectionView`: Host, port, username, secure password field, “Save password” toggle, Connect. Disable Connect while connecting. Port uses decimal keyboard but still validates `1...65535`.
- `CharacterListView`: plain list of character names; selecting a row issues `selectCharacter` once and switches to entering state. It has Disconnect.
- `ChatView`: `ScrollView`/`LazyVStack` transcript, one text field, Send, Disconnect. Send is disabled unless the bridge has emitted both `entered_world` and `InWorld` confirmation. The composer clears only after the local command is accepted into Rust's command queue.

The app stores host, port, username, and save-password preference in `UserDefaults`. When Save password is enabled it writes the password to a Keychain item with service `net.gromnie.ios` and account `<host>:<port>:<username>`; disabling it immediately deletes that item. It uses `kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly`. Password fields and raw server responses are excluded from `os.Logger` calls.

## iOS network/lifecycle configuration

- `Info.plist` includes `NSLocalNetworkUsageDescription`: “Gromnie connects directly to game servers on your local network.” It is required for LAN testing; public-server traffic uses the same direct UDP code path.
- No App Transport Security exceptions are needed because v1 makes no HTTP/HTTPS/WebSocket connection.
- When the scene becomes inactive or backgrounds, `SessionViewModel` asynchronously disconnects on the core queue and clears chat/character state. It does not request background execution.
- Host validation accepts a DNS hostname or IPv4 address only, rejects URLs, brackets, spaces, colons, and IPv6 literals. Gromnie resolves the hostname normally; an IPv6-only result is presented as “This version requires an IPv4-reachable server.”

## Build and packaging pipeline

Prerequisites are Xcode command-line tools, Rust stable, `cbindgen`, and these Rust targets:

```bash
rustup target add aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios
cargo install cbindgen
```

Add `cargo xtask ios build-core`. It must:

1. Run `cbindgen` from `crates/gromnie-ios-bridge/cbindgen.toml`, fail if the generated header differs from committed `include/gromnie_ios.h`.
2. Build release static libraries for `aarch64-apple-ios`, `aarch64-apple-ios-sim`, and `x86_64-apple-ios`.
3. Create a universal simulator library with `lipo` from the two simulator slices.
4. Run `xcodebuild -create-xcframework` with the device library, universal simulator library, and the generated header directory.
5. Write the result to `ios/Gromnie/Frameworks/GromnieCore.xcframework` and replace only that generated artifact.

CI runs the Rust bridge tests on the host, builds all three iOS targets, builds the XCFramework, and runs `xcodebuild test` on an iOS simulator. It does not attempt live-server tests in CI.

## Tests and release gate

### Automated

- Rust unit tests: C input validation, JSON schema, every event mapping above, command ordering, terminal-event uniqueness, full-queue backpressure, and repeated `disconnect`/`destroy`.
- Rust integration test with a fake `ClientTransport`: login event → character selection action → login success → chat event, verifying the exact bridge event sequence.
- Swift unit tests: decoding, screen reducer transitions, 1,000-line trimming, form validation, and Keychain save/delete behavior.
- Swift UI tests with a fake `GromnieCoreClient`: form → list → chat, disabled states, error recovery, and accessible labels.

### Manual device matrix

Test a debug build on a physical iPhone on Wi-Fi and cellular, against a non-production account:

1. Valid login and character list.
2. Bad password.
3. Invalid hostname, unreachable UDP port, and IPv6-only endpoint.
4. Empty character list.
5. Character selection, entered-world confirmation, incoming chat, outgoing chat, and ordering under a burst of messages.
6. User disconnect, server disconnect, app backgrounding, and returning foreground.
7. Saved-password opt-in, relaunch restore, and deletion after toggling it off.

Beta is ready only when all automated checks pass and every device scenario succeeds without password leakage, a crash, a retained Rust actor, or a stuck connecting screen.

## Explicit non-goals after v1

World rendering, movement, map, inventory, tells, automatic reconnect, IPv6, a proxy fallback, App Store submission, and multi-account switching require new design work and are not silently added to this implementation.
