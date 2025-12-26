# Gromnie Architecture

## Overview

Gromnie is a Rust async client for the Asheron's Call protocol, built on tokio for high-performance network handling.

## Network Architecture

### Port Routing

The AC protocol uses two UDP ports:

- **Port 9000 (Login Server)**: LoginRequest, LoginCharacterSet, DDD responses, and most messages
- **Port 9001 (World Server)**: ConnectResponse only during handshake, later game commands

### ServerInfo Pattern

Tracks both ports and provides methods to resolve the correct endpoint:

```rust
pub struct ServerInfo {
    pub host: String,
    pub login_port: u16,  // 9000
    pub world_port: u16,  // 9001
}

impl ServerInfo {
    pub async fn login_addr(&self) -> Result<SocketAddr, std::io::Error>
    pub async fn world_addr(&self) -> Result<SocketAddr, std::io::Error>
    pub fn is_from(&self, peer: &SocketAddr) -> bool
}
```

## Message Handling

### Fragment Assembly → RawMessage → Message Handlers

```
Raw UDP Packet (from network)
           ↓
    PacketHeader (20 bytes)
           ↓
   BlobFragments (multiple)
           ↓
   Fragment reassembly (in pending_fragments map)
           ↓
    Complete RawMessage
       (includes 4-byte opcode)
           ↓
   Message Queue (VecDeque<RawMessage>)
           ↓
  Handler dispatch (handle_message)
           ↓
    Type-specific handler
    (LoginCharacterSet, DDD, etc.)
```

### Important: RawMessage Data Format

**RawMessage.data includes the opcode as the first 4 bytes**

When parsing:
```rust
let payload = &message.data[4..];  // Skip opcode
let mut cursor = Cursor::new(payload);
LoginLoginCharacterSet::read(&mut cursor)?;
```

## Outgoing Message Queue

For messages that need to be sent after receiving another message:

```rust
pub enum PendingOutgoingMessage {
    DDDInterrogationResponse(DDDInterrogationResponseMessage),
    // Add more types as needed
}
```

Process:
1. Message arrives → handler queues response
2. Main network loop: `if client.has_pending_outgoing_messages()`
3. Loop calls `client.send_pending_messages()`
4. Messages sent to appropriate port via ServerInfo

## Session State

```rust
struct SessionState {
    cookie: u64,           // From ConnectRequest
    client_id: u16,        // From ConnectRequest
    table: u16,            // Iteration counter
    outgoing_seed: u32,    // Server→Client checksum
    incoming_seed: u32,    // Client→Server checksum
}
```

## Network Loop

```rust
loop {
    match client.socket.recv_from(&mut buf).await {
        Ok((size, peer)) => {
            // 1. Parse packet header, handle flags
            client.process_packet(&buf[..size], size, &peer).await;
            
            // 2. Check for completed fragments → RawMessage
            if client.has_messages() {
                client.process_messages();  // Dispatch to handlers
            }
            
            // 3. Send any queued responses
            if client.has_pending_outgoing_messages() {
                client.send_pending_messages().await?;
            }
        }
        Err(e) => { /* error handling */ }
    }
}
```

## Message Types

### Received (S2C) - Server to Client

- **ConnectRequest** (0xF7E0): Server's handshake, sends cookie and credentials
- **LoginCharacterSet** (0xF658): Character list for the account
- **DDDInterrogationMessage** (0xF7E5): Request for game file checksums
- **AckSequence**: Packet acknowledgment
- **TimeSync**: Server time synchronization
- **BlobFragments**: Fragmented message data

### Sent (C2S) - Client to Server

- **LoginRequest** (0xF7DF): Initial login with credentials
- **ConnectResponse** (0xF7DF): Completes handshake with cookie
- **DDDInterrogationResponseMessage**: File checksum response
- **AckSequence**: Acknowledge received packets

## Fragment Reassembly

```rust
struct Fragment {
    sequence: u32,
    chunks: HashMap<u32, Vec<u8>>,
    count: u32,
    received: u32,
}
```

Process:
1. Receive BlobFragments from packet
2. Look up or create Fragment by sequence number
3. Add chunk at specified index
4. When all chunks received (received == count):
   - Concatenate all chunks
   - Parse as RawMessage with opcode
   - Queue for message processing
   - Clean up and remove

## Key Files

- **src/client/client.rs** (main client implementation)
  - Client struct
  - ServerInfo
  - Message handling
  - Fragment reassembly
  - Outgoing message queue

- **src/bin/gromnie.rs** (main entry point)
  - Network loop
  - Spawns client task
  - Uses tmux-friendly logging

## Dependencies

- **acprotocol**: Protocol definitions, message parsing
- **tokio**: Async runtime
- **tracing**: Structured logging
- **clap**: CLI argument parsing

## Next Steps

1. **Character Selection** - Send EnterWorld or similar to log into game world
2. **World State** - Handle game objects, players, locations
3. **Game Commands** - Movement, emotes, chat, combat
4. **Persistence** - Save world state, character state
