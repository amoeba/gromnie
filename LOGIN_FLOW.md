# Login Flow Analysis

## Summary
The login process is **working correctly**. The client successfully completes character login and receives world data.

## Complete Login Message Flow

Based on analysis of the ACE server source code and protocol.xml, here's the complete message sequence:

### 1. Character Selection Phase
**Client → Server:**
- `Login_SendEnterWorldRequest` (0xF7C8)
  - Sent when user clicks "Enter" on character select
  - No character ID included yet

**Server → Client:**
- `CharacterEnterWorldServerReady` (0xF7DF)
  - Confirms server is ready to receive the character login

### 2. Character Login Phase
**Client → Server:**
- `Login_SendEnterWorld` (0xF657)
  - Contains: character_id (u32) + account_name (String)
  - This is the actual character login request

**Server → Client** (sends many messages in sequence):
The server's `PlayerEnterWorld()` method sends:
1. `GameEventPlayerDescription` - Player stats and properties
2. `GameEventCharacterTitle` - Character title info
3. `GameEventFriendsListUpdate` - Friends list
4. `GameMessagePlayerCreate` - Creates the player object
5. `GameMessageCreateObject` (player) - Player object data
6. `GameMessageCreateObject` (×N) - All inventory items
7. `GameEventViewContents` - Container contents
8. `GameEventSendClientContractTrackerTable` - Quest/contract data
9. Various chat messages (welcome, MOTD, etc.)

### 3. Client Acknowledgment Phase
**Client → Server:**
- `Character_LoginCompleteNotification` (0x00A1) - GameAction
  - Sent after client has processed all world data
  - Signals client is ready for character to materialize
  - Triggered by `ClientAction::SendLoginComplete`

**Server Response:**
- Server calls `Player.OnTeleportComplete()`
- If first login: calls `Player.SendPropertyUpdatesAndOverrides()`
- Sets `FirstEnterWorldDone` flag

## Key Server Code Paths

### CharacterHandler.cs
```csharp
// Line 185-198: Handles CharacterEnterWorldRequest (0xF7C8)
[GameMessage(GameMessageOpcode.CharacterEnterWorldRequest, SessionState.AuthConnected)]
public static void CharacterEnterWorldRequest(ClientMessage message, Session session)
{
    // ... validation ...
    session.Network.EnqueueSend(new GameMessageCharacterEnterWorldServerReady());
}

// Line 200-265: Handles CharacterEnterWorld (0xF657)
[GameMessage(GameMessageOpcode.CharacterEnterWorld, SessionState.AuthConnected)]
public static void CharacterEnterWorld(ClientMessage message, Session session)
{
    var guid = message.Payload.ReadUInt32();
    string clientString = message.Payload.ReadString16L();
    // ... validation ...
    WorldManager.PlayerEnterWorld(session, character);
}
```

### WorldManager.cs
```csharp
// Line 93-110: PlayerEnterWorld entry point
public static void PlayerEnterWorld(Session session, Character character)
{
    // Loads character data from database
    // Calls DoPlayerEnterWorld()
}

// Line 112-278: DoPlayerEnterWorld
private static void DoPlayerEnterWorld(...)
{
    // Creates Player object
    // Line 220: session.Player.PlayerEnterWorld();
    // Line 222: LandblockManager.AddObject(session.Player, true);
    // Sends welcome messages
}
```

### Player_Networking.cs
```csharp
// Line 22-145: PlayerEnterWorld
public void PlayerEnterWorld()
{
    // Line 76: SendSelf();  // Triggers portal space entrance
    // Joins chat channels
    // Handles allegiance, house, etc.
}

// Line 214-229: SendSelf
private void SendSelf()
{
    // Line 216-220: Sends player description, title, friends
    // Line 224: Sends GameMessagePlayerCreate + GameMessageCreateObject
    // Line 226: SendInventoryAndWieldedItems();
    // Line 228: SendContractTrackerTable();
}
```

### GameActionLoginComplete.cs
```csharp
// Line 10-20: Handles LoginComplete from client (0x00A1)
[GameAction(GameActionType.LoginComplete)]
public static void Handle(ClientMessage message, Session session)
{
    session.Player.OnTeleportComplete();

    if (!session.Player.FirstEnterWorldDone)
    {
        session.Player.FirstEnterWorldDone = true;
        session.Player.SendPropertyUpdatesAndOverrides();
    }
}
```

## Current Implementation Status - ✅ COMPLETE

### Working ✓
All login flow steps are now implemented and working:

1. ✅ **Send CharacterEnterWorldRequest (0xF7C8)**
   - Implemented in `send_enter_world_request_internal()`
   - Sent when user clicks Enter on character select
   - No payload, just the opcode

2. ✅ **Receive CharacterEnterWorldServerReady (0xF7DF)**
   - Handled in `handle_enter_game_server_ready()`
   - Triggers sending of EnterWorld message with character ID
   - State machine: WaitingForServerReady → LoadingWorld

3. ✅ **Send Login_SendEnterWorld (0xF657)**
   - Implemented in `send_enter_world_internal()`
   - Contains character_id and account name
   - Only sent after receiving ServerReady

4. ✅ **Receive World Data**
   - GameMessagePlayerCreate (0xF746)
   - ItemCreateObject (0xF745) × N
   - Chat messages and other game state
   - All messages forwarded to event bus for UI

5. ✅ **Send LoginComplete (0x00A1)**
   - Properly wrapped in Ordered_GameAction (0xF7B1)
   - Format: 0xF7B1 + 0x00A1
   - Sent automatically 2 seconds after receiving world data
   - Server processes it and completes login

### Recent Fixes (2025-12-26)

1. **Added CharacterEnterWorldRequest step**
   - Was missing this initial handshake
   - Server was accepting login anyway but protocol is now correct

2. **Fixed LoginComplete format**
   - Was sending bare 0x00A1
   - Now properly wraps in Ordered_GameAction (0xF7B1)
   - Server now processes the message correctly

3. **Implemented state machine**
   - CharacterLoginState tracks progress through login flow
   - Prevents duplicate login attempts
   - Ensures messages sent in correct order

4. **All messages sent to event bus**
   - Every received message emits NetworkMessage event
   - UI can display all protocol activity
   - Helps with debugging and monitoring

## Message Opcodes Reference

### Client → Server
- `0xF7C8` - Login_SendEnterWorldRequest (click Enter)
- `0xF657` - Login_SendEnterWorld (character login with ID)
- `0x00A1` - Character_LoginCompleteNotification (GameAction, ready to materialize)

### Server → Client
- `0xF7DF` - CharacterEnterWorldServerReady (server ready for character ID)
- Various GameMessages for player creation and world data
