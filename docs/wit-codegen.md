# WIT Codegen: Research & Design Plan

## Problem Statement

The scripting system exposes protocol events to WASM scripts via a [WIT interface](../crates/gromnie-scripting-api/src/wit/gromnie-script.wit). Every type in that interface was written by hand. So far that covers 12 of 92 S2C message types and 2 of 101 game event types.

Covering the remaining ~80 S2C types and ~99 game event types manually would require:
- A WIT record per protocol type
- A `S2CEvent` / `GameEventMsg` Rust enum variant
- A `ToProtocolEvent` conversion impl
- A `MessageHandler` impl
- A client dispatch arm
- A `s2c_event_to_wit()` / `game_event_msg_to_wit()` match arm

That's six files touched per type, ~500 mechanical changes total. Codegen is the right answer.

---

## The Existing asheron-rs Codegen

`acprotocol/src/generated/` is entirely machine-generated. The pipeline is:

```
ACProtocol/protocol.xml   (8,515 lines, ground truth)
    ↓
crates/codegen/           (Rust XML parser + code emitter)
    ↓
acprotocol/src/generated/ (Rust structs, read/write impls, enums)
```

`protocol.xml` describes every message, field, type, and enum in the protocol. Example:

```xml
<message name="Movement_PositionEvent" opcode="0xF748" direction="S2C">
  <field name="ObjectId" type="ObjectId" />
  <field name="Position" type="PositionPack" />
</message>
```

The codegen reads this XML into an IR (`ProtocolType`, `Field`, `ProtocolEnum`), then emits Rust via per-category generators (`type_generation.rs`, `message_generation.rs`, etc.). Running `cargo xtask generate` in asheron-rs regenerates everything.

Type mapping used by asheron-rs codegen (`type_utils.rs`):

| XML type | Rust type |
|---|---|
| `uint` / `int` / `ushort` / etc. | `u32` / `i32` / `u16` / etc. |
| `float` / `double` | `f32` / `f64` |
| `string` | `String` |
| `bool` | `bool` |
| Semantic newtypes (`ObjectId`, `LandcellId`) | newtype struct wrapping primitive |
| Bitflag enums (`PositionFlags`) | `bitflags!` struct |
| Conditional fields (`<if>`, `<maskmap>`) | `Option<T>` |
| Lists | `Vec<T>` / `PackableList<T>` |
| Enums | `#[repr(u32)] enum` with `TryFromPrimitive` |

---

## Approach Options

### Option A: Extend asheron-rs codegen to also emit WIT + gromnie glue

The asheron-rs codegen already has the protocol IR. We could add new output targets:
- A WIT emitter that generates `gromnie-script.wit` records and variant arms
- A Rust glue emitter that generates the `S2CEvent` variants and conversion impls

**Pros:**
- Single source of truth (protocol.xml drives everything end-to-end)
- Changes to the protocol XML automatically propagate to both acprotocol and gromnie
- Protocol IR is already built; we only need new emitters

**Cons:**
- Requires changes to the upstream asheron-rs repo
- Couples gromnie's build tooling to asheron-rs internals
- The WIT output has different naming constraints (kebab-case) and different type system (no newtypes, no bitflags)

### Option B: Standalone gromnie codegen reading protocol.xml directly

A new crate in gromnie (e.g., `crates/gromnie-codegen` or `crates/xtask` extension) that reads `protocol.xml` independently and emits WIT + Rust glue.

**Pros:**
- No upstream dependency; fully self-contained
- Can be tailored to gromnie's simplification policy (e.g., always flatten newtypes, omit sequence fields)
- Can be added as a cargo xtask: `cargo xtask generate-wit`

**Cons:**
- Duplicates some XML parsing logic from asheron-rs
- Must be manually re-run when protocol.xml changes (already the case with Option A)
- Requires a copy of or path to `protocol.xml` at build time

### Option C: Derive from acprotocol Rust types via `syn`

Instead of reading XML, parse the already-generated acprotocol Rust source with `syn` and derive WIT + glue from the struct definitions.

**Pros:**
- No XML dependency; works from what's already in Cargo
- Tracks acprotocol's actual generated output, not the XML schema

**Cons:**
- Harder to extract metadata (`syn` doesn't know a field is a newtype wrapping `u32` without following all types)
- Fragile to code-generation style changes in asheron-rs
- Can't use protocol comments/descriptions without parsing doc attributes

**Verdict:** Option B is the most practical starting point. Option A is the right long-term solution once upstream codegen in asheron-rs is stable.

---

## WIT Type System Constraints

WIT supports only: `u8 u16 u32 u64 s8 s16 s32 s64 f32 f64 bool char string`, `option<T>`, `list<T>`, `record`, `variant`, `enum`, `tuple`, `result`. No newtypes, no bitflags, no generics, no inheritance.

This means every acprotocol type needs a mapping decision:

| acprotocol type | WIT mapping | Notes |
|---|---|---|
| `ObjectId(u32)` | `u32` | Always flatten; IDs are just numbers to scripts |
| `LandcellId(u32)` | `u32` | Same |
| `DataId(u32)` | `u32` | Same |
| `PositionFlags` (bitflags) | `u32` | Expose raw bits; document flag constants |
| `HoldKey` (C-style enum) | `u32` | Script passes constants; too small to enumerate in WIT |
| `StanceMode`, `Command` (enums) | `u32` | Same: pass raw discriminant |
| `Vector3 { x, y, z: f32 }` | `record vec3 { x: f32, y: f32, z: f32 }` | Straightforward |
| `Quaternion { w, x, y, z: f32 }` | inline fields or record | Flatten into parent |
| `Option<f32>` (conditional field) | `option<f32>` | Direct |
| `Vec<T>` / `PackableList<T>` | `list<T>` | Direct |
| `MovementData` (variant with 5 subtypes) | special case | See below |
| Sequence numbers (`object_instance_sequence`, etc.) | **omit** | Internal protocol bookkeeping; scripts don't need them |
| Protocol flags used only for parsing | **omit** | e.g., `PositionFlags` is consumed during parse |

### The `MovementData` Problem

`MovementData` is an enum with 5 variants (Type0–Type9), each containing different combinations of `InterpretedMotionState`, movement targets, `StanceMode`, etc. It appears in `MovementSetObjectMovement` and `MovementPositionAndMovementEvent`.

Options:
1. **Omit entirely** (current approach) — expose only `object_id`; scripts that need motion state parse it via `MovementPosition` instead
2. **Flatten to a simplified record** — expose only the most common subtype (Type0/`InterpretedMotionState`) with an `unknown` fallback
3. **Expose as opaque `u32` tag + `list<u8>` payload** — scripts that care can decode it themselves (defeats the point of typed access)
4. **Multiple event variants** — `movement-set-object-movement-interpreted`, `movement-set-object-movement-move-to`, etc. (one per MovementData subtype)

Option 4 is cleanest for scripts but requires careful WIT design. This is a key open question.

---

## What the Generator Must Emit

For each S2C message type (e.g., `QualitiesUpdatePosition`):

### 1. WIT record
```wit
record qualities-update-position-msg {
    object-id: u32,
    key: u32,        // PropertyPosition discriminant
    landcell: u32,
    x: f32,
    y: f32,
    z: f32,
}
```

### 2. WIT `s2c-event` variant arm
```wit
variant s2c-event {
    // ... existing ...
    qualities-update-position(qualities-update-position-msg),
}
```

### 3. `S2CEvent` Rust enum variant (`protocol_events.rs`)
```rust
QualitiesUpdatePosition {
    object_id: u32,
    key: u32,
    landcell: u32,
    x: f32, y: f32, z: f32,
},
```

### 4. `ToProtocolEvent` impl (`protocol_conversions.rs`)
```rust
impl ToProtocolEvent for acprotocol::messages::s2c::QualitiesUpdatePosition {
    fn to_protocol_event(&self) -> S2CEvent {
        S2CEvent::QualitiesUpdatePosition {
            object_id: self.object_id.0,
            key: self.key.clone() as u32,
            // position fields...
        }
    }
}
```

### 5. `MessageHandler` impl (`message_handlers.rs`)
```rust
impl MessageHandler<acprotocol::messages::s2c::QualitiesUpdatePosition> for Client {
    fn handle(&mut self, msg: ...) -> Option<GameEvent> {
        let protocol_event = ProtocolEvent::S2C(msg.to_protocol_event());
        let _ = self.raw_event_tx.try_send(ClientEvent::Protocol(protocol_event));
        None
    }
}
```

### 6. Client dispatch arm (`client.rs`)
```rust
S2CMessage::QualitiesUpdatePosition => {
    dispatch_message::<acprotocol::messages::s2c::QualitiesUpdatePosition, _>(
        self, message, &event_tx,
    ).ok();
}
```

### 7. `s2c_event_to_wit()` match arm (`wasm_script.rs`)
```rust
S2CEvent::QualitiesUpdatePosition { object_id, key, landcell, x, y, z } => {
    WitS2cEvent::QualitiesUpdatePosition(QualitiesUpdatePositionMsg {
        object_id: *object_id,
        key: *key,
        landcell: *landcell,
        x: *x, y: *y, z: *z,
    })
}
```

That's 7 outputs per type. With ~80 S2C types and ~99 game event types, that's ~1,260 generated code units.

---

## Naming Conventions

WIT requires kebab-case identifiers. acprotocol uses PascalCase. The mapping is mechanical:

| acprotocol | WIT |
|---|---|
| `QualitiesUpdatePosition` | `qualities-update-position` |
| `MovementPositionEvent` | `movement-position-event` |
| `object_id` | `object-id` |
| `HearDirectSpeech` | `hear-direct-speech` |

One edge case: WIT variant arms cannot start with a digit, so any type like `Type0`/`Type6` needs renaming (relevant for `MovementData` subtypes if we expose them).

---

## Input: Where Does protocol.xml Come From?

The asheron-rs repo is a git dependency in `Cargo.toml`. At build time, Cargo checks it out to `~/.cargo/git/checkouts/`. The `protocol.xml` file lives alongside the source:

```
~/.cargo/git/checkouts/asheron-rs-.../ddbffb9/ACProtocol/protocol.xml
```

The codegen tool could locate it via `cargo metadata`. Alternatively, we could vendor a copy of `protocol.xml` into the gromnie repo (simpler, but requires manual updates when protocol.xml changes upstream).

---

## Open Questions

1. **Where does the generator live?** New `crates/gromnie-codegen` xtask sub-command, or extend the existing `crates/xtask`?

2. **How does it find `protocol.xml`?** Locate via `cargo metadata` at runtime, or vendor a copy?

3. **What's the `MovementData` strategy?** Omit / flatten Type0 / expose all subtypes as separate events?

4. **Do we expose all fields, or continue the "simplification" policy** (strip sequence numbers, flatten newtypes)?  The current hand-written types strip a lot. Codegen could be configurable per-field via annotations in a `gromnie-overrides.toml` or similar.

5. **How do we handle types shared between categories?** `PositionPack` is used by both S2C messages and game events. It should generate one WIT record, not N duplicates.

6. **Should the WIT and Rust glue be generated into the source tree** (checked in, regenerated on demand like acprotocol) **or generated at build time** (via `build.rs`)?  Checking in is easier to review; build-time avoids stale generated code.

7. **Should enums be WIT `enum` or `u32`?** WIT `enum` variants must be exhaustive at the WIT level. If acprotocol adds a new `HoldKey` variant, scripts compiled against the old WIT break. Using `u32` is more future-proof.

---

## Suggested Next Steps

1. **Prototype**: Write a minimal codegen that reads `protocol.xml`, picks one category (e.g., all `QualitiesPrivateUpdate*` S2C messages), and emits correct WIT records + Rust glue. Validate that it compiles and that the existing scripting tests still pass.

2. **Decide on MovementData** before writing full codegen, since it affects the variant design of `game-event-msg`.

3. **Decide on shared types** (PositionPack, Vector3, etc.) — define them once in WIT and reference them from generated records.

4. **Write the full generator** based on validated prototype, covering all S2C and game event categories.

5. **Consider upstreaming** the WIT emitter to asheron-rs as Option A once the design is stable.
