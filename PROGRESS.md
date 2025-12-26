# Gromnie Progress Summary

## Session Summary (Dec 26, 2025)

### Issues Resolved

#### 1. **Generalized Message Queue Pattern** ✅
- **Problem**: DDD interrogation response sending was hardcoded to a single message type
- **Solution**: Created `PendingOutgoingMessage` enum to support any outgoing message type
- **Implementation**:
  - Messages are now queued generically without needing direct port knowledge
  - `send_pending_messages()` loop handles all queued messages in the network loop
  - Easy to add new message types by extending the enum

#### 2. **Port Routing (Login vs World)** ✅
- **Problem**: Server has two ports (9000 for login, 9001 for world), but client didn't know which to use for different messages
- **Solution**: Created `ServerInfo` struct following the TestClient pattern
- **Implementation**:
  - `ServerInfo` tracks both `login_port` (9000) and `world_port` (9001)
  - Methods: `login_addr()` and `world_addr()` handle DNS lookup and return correct `SocketAddr`
  - DDD responses correctly go to port 9000 (login server)
  - ConnectResponse still goes to port 9001 (world server)

#### 3. **Character List Parsing** ✅
- **Problem**: Character list showed garbage: `\x07testing` instead of `testing`, and 0 characters
- **Root Cause**: `RawMessage.data` field includes the 4-byte opcode as a header
- **Solution**: Skip first 4 bytes when parsing
- **Implementation**: `LoginLoginCharacterSet::read()` now reads from `&message.data[4..]` instead of `&message.data`
- **Result**: 
  - Account name parses correctly: `testing`
  - Character slots: `11`
  - Characters on account: `0` (no characters created yet in test server)

### Code Changes

#### client/client.rs

1. **Added ServerInfo struct** (lines 35-72)
   - Tracks host and both port numbers
   - Provides `login_addr()` and `world_addr()` async methods
   - `is_from()` checks if a peer matches this server

2. **Generalized outgoing messages** (lines 11-14)
   - `PendingOutgoingMessage` enum replaces hardcoded `pending_ddd_response`
   - Queue processes all message types generically

3. **Simplified message handlers**
   - Removed peer address tracking from message handlers
   - Messages are port-agnostic; ServerInfo handles routing
   - DDD response explicitly uses `server.login_addr()` for sending

4. **Fixed message parsing** (lines 366-378, 400-410)
   - Skip 4-byte opcode header: `&message.data[4..]`
   - Applied to both LoginLoginCharacterSet and DDDInterrogationMessage handlers

### Current Flow

```
Client                  Server (9000)       Server (9001)
  |                         |                   |
  |--LoginRequest--------->|                   |
  |                        |                   |
  |<--ConnectRequest-------|                   |
  |                        |                   |
  |------ConnectResponse-------------------->|
  |                        |                   |
  |<--LoginCharacterSet (fragmented)----------|
  |                        |                   |
  |--DDD Response--------->|                   |
  |                        |                   |
```

### Architecture Improvements

**Before**: Peer addresses tracked through message handling chain
**After**: ServerInfo knows port destinations; messages remain port-agnostic

This matches the TestClient (C#) approach and is much cleaner.

### Next Steps

1. **Character Selection** - Handle game world login after character selection
2. **World Commands** - Send player movement/action commands
3. **More S2CMessage Handlers** - Handle game events, object updates, etc.
4. **Session State Management** - Track player state (zone, equipment, etc.)

### Testing Notes

- Server: ACEmulator running in tmux session "ace"
- Client: Gromnie running in tmux session "gromnie"
- Both sessions auto-start and manage the network loop
- Client successfully receives and parses:
  - ConnectRequest ✓
  - LoginLoginCharacterSet ✓
  - DDDInterrogationMessage ✓
  - Can send ConnectResponse ✓
  - Can send DDD response ✓
