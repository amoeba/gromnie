# Gromnie Client Progress - Character List Reception

## Current Status: Debugging Character List Reception

We're at the stage where the client should receive the Login_LoginCharacterSet (0xF658) message from the server after authentication, but there's a session matching issue preventing the server from transitioning to AuthConnected state.

## What We've Accomplished

### 1. Fixed "Unverified" Session Investigation ✅
- Added logging to ACE.Server to show session state transitions
- Server now logs: `Session [Unverified] -> [testing] authenticated`
- Confirmed authentication is working correctly

### 2. Simplified Logging ✅
- Removed verbose packet processing spans
- Kept only high-level send/receive logs
- Much cleaner output for debugging

### 3. Implemented Character List Handler ✅
- **File**: `/Users/bryce/src/amoeba/gromnie/src/client/client.rs`
- Added imports for `LoginLoginCharacterSet` message type
- Replaced panic-based `handle_message()` with proper message dispatcher
- Implemented `handle_character_list()` method that:
  - Parses message using acprotocol's generated parser
  - Displays account name and character count
  - Lists all characters with their IDs
  - Shows deletion status if applicable

### 4. Added Race Condition Delay ✅
- Discovered server validates password asynchronously
- Added 200ms delay before sending ConnectResponse
- This ensures server is in `AuthConnectResponse` state when we send ConnectResponse

## Current Problem: Session Matching Failure

### The Issue
Server receives ConnectResponse with correct cookie (`0x218F000BE05CDDF2`) but session lookup fails with:
```
Received ConnectResponse from 127.0.0.1:51158 but no matching session found
```

### Authentication Flow Timeline
1. Client sends LoginRequest → Server (port 9000)
2. Server sends ConnectRequest with cookie
3. Client waits 200ms
4. Client sends ConnectResponse with cookie → Server (port 9001)
5. **❌ Server can't find matching session**

### Server Session Lookup Conditions
Located in `/Users/bryce/src/acemulator/ace/Source/ACE.Server/Network/Managers/NetworkManager.cs:61-78`

The server looks for a session matching ALL of:
1. `k != null` ✅
2. `k.State == SessionState.AuthConnectResponse` ✅ (confirmed in logs)
3. `k.Network.ConnectionData.ConnectionCookie == connectResponse.Check` ❓ (cookie matches in logs but maybe byte order?)
4. `k.EndPointC2S.Address.Equals(endPoint.Address)` ❓ (might be the issue - port difference?)

### What We Were Debugging
Added detailed debug logging to see ALL sessions and their properties when ConnectResponse arrives:
```csharp
log.Debug($"Looking for session matching cookie 0x{connectResponse.Check:X16} from {endPoint.Address}");
foreach (var k in sessionMap)
{
    if (k != null)
    {
        log.Debug($"  Session: {k.LoggingIdentifier}, State: {k.State}, Cookie: 0x{k.Network.ConnectionData.ConnectionCookie:X16}, IP: {k.EndPointC2S.Address}");
    }
}
```

This will show us which specific condition is failing in the LINQ query.

## Next Steps

### 1. Run Server with Debug Logging
```bash
cd /Users/bryce/src/acemulator/ace/Source/ACE.Server
dotnet build
cd bin/Debug/net8.0
dotnet ACE.Server.dll
```

### 2. Run Client in Separate Tmux Session
```bash
cd /Users/bryce/src/amoeba/gromnie
cargo build
./target/debug/gromnie
```

### 3. Check Server Logs for Session Details
Look for the "Looking for session" debug output to see:
- What cookie the server has stored
- What state the session is in
- What IP address is stored (EndPointC2S vs endPoint)

### 4. Likely Issues to Investigate

**Issue A: Port Mismatch**
- `EndPointC2S` is from port 9000 (LoginRequest)
- `endPoint` is from port 9001 (ConnectResponse)
- The IP is the same but the PORT is different!
- Server might be comparing full `IPEndPoint` instead of just `Address`

**Issue B: Cookie Byte Order**
- Less likely since logs show matching cookie values
- But worth checking if there's a byte order conversion issue

### 5. Probable Fix
The issue is likely line 77 in NetworkManager.cs:
```csharp
k.EndPointC2S.Address.Equals(endPoint.Address)
```

This should work because it's comparing just the Address (IP), not the port. But if it's not working, we might need to check:
- Is `EndPointC2S` set correctly?
- Is the comparison working as expected?

## Files Modified

### Client Side (Gromnie)
- `/Users/bryce/src/amoeba/gromnie/src/client/client.rs`
  - Line 6: Added `use acprotocol::messages::s2c::LoginLoginCharacterSet;`
  - Line 17: Added `info` and `warn` to tracing imports
  - Lines 232-249: Replaced panic with message dispatcher in `handle_message()`
  - Lines 251-280: New `handle_character_list()` method
  - Lines 342-344: Added cookie/client ID debug logging
  - Lines 357-359: Added 200ms delay before ConnectResponse
  - Lines 566-567: Added cookie debug logging for ConnectResponse

### Server Side (ACE.Server)
- `/Users/bryce/src/acemulator/ace/Source/ACE.Server/Network/Session.cs`
  - Lines 179-189: Modified `SetAccount()` to log transition from "Unverified" to account name

- `/Users/bryce/src/acemulator/ace/Source/ACE.Server/Network/Handlers/AuthenticationHandler.cs`
  - Lines 231-234: Added debug logging for state transitions

- `/Users/bryce/src/acemulator/ace/Source/ACE.Server/Network/Managers/NetworkManager.cs`
  - Lines 61-69: Added debug logging to show all sessions when ConnectResponse arrives
  - Lines 77-85: Added log message when ConnectResponse received but state changes
  - Lines 82-85: Added warning when no matching session found

## Expected Outcome Once Fixed

1. Client sends LoginRequest
2. Server authenticates and transitions to AuthConnectResponse
3. Client sends ConnectResponse (after 200ms delay)
4. Server finds matching session and transitions to AuthConnected
5. **Server sends Login_LoginCharacterSet (0xF658) as fragmented message**
6. Client reassembles fragments
7. Client parses character list and displays:
   ```
   === Character List for Account: testing ===
   Available character slots: 10
   Characters on account: 3
     - MyCharacter1 (ID: ObjectId(...))
     - MyCharacter2 (ID: ObjectId(...))
     - MyCharacter3 (ID: ObjectId(...))
   ```

## Tmux Sessions

- **ace**: ACE.Server running
- **gromnie**: Gromnie client running

## Key Protocol Insights Learned

1. **"Unverified"** is just the initial LoggingIdentifier before SetAccount() is called
2. **ConnectResponse must go to port 9001**, not 9000 (world server port)
3. **Password validation happens asynchronously** - need delay before ConnectResponse
4. **Session lookup requires exact match** on state, cookie, and IP address
5. **acprotocol has full parser support** for LoginLoginCharacterSet message

## Architecture Notes

### Authentication Flow
```
Client                          Server (9000)         Server (9001)
  |                                 |                      |
  |--LoginRequest------------------>|                      |
  |                                 |                      |
  |<---ConnectRequest---------------|                      |
  |                                 |                      |
  | (200ms delay for auth)          |                      |
  |                                 |                      |
  |--ConnectResponse---------------------------------->|   |
  |                                 |                 (looking for session)
  |                                 |                      |
  |<---Login_LoginCharacterSet------|                      |
  | (fragmented message)            |                      |
```

### Session States
1. `AuthLoginRequest` - Initial state, waiting for login
2. `AuthConnectResponse` - Password validated, waiting for ConnectResponse
3. `AuthConnected` - **Target state** where character list is sent
4. `WorldConnected` - After character selection

## Questions to Answer Next

1. Why does the session lookup fail when the cookie matches?
2. Is there an IP/port comparison issue with EndPointC2S vs endPoint?
3. Do we need to send from a specific source port?

## Useful Debug Commands

```bash
# Watch client logs
tmux capture-pane -t gromnie -p | tail -30

# Watch server logs
tmux capture-pane -t ace -p | tail -50

# Search for authentication
tmux capture-pane -t ace -p -S -200 | grep -B 5 -A 10 "authenticated\|AuthConnected"

# Search for cookie
tmux capture-pane -t ace -p -S -200 | grep -i "cookie"

# Build client
cd /Users/bryce/src/amoeba/gromnie && cargo build

# Build server
cd /Users/bryce/src/acemulator/ace/Source/ACE.Server && dotnet build
```
