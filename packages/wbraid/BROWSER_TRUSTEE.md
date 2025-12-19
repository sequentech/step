<!--
 SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->
# Browser-Based Mixnet Trustee

This is a browser implementation of a WBraid mixnet trustee using WebAssembly. It allows running a full cryptographic mixnet node entirely in your browser.

## Features

- ✅ Full protocol implementation (DKG, mixing, decryption) in WASM
- ✅ In-memory message board (no database required)
- ✅ HTTP communication with B4 bulletin board server
- ✅ Real-time console logging from Rust code
- ✅ Manual step control and auto-execution mode
- ✅ Live protocol state visualization

## Quick Start

### 1. Start the Demo Environment

Use the browser-specific demo script that starts B4 + native trustees, leaving one slot for a browser trustee:

```powershell
.\demo-browser.ps1
```

Optional parameters:
```powershell
.\demo-browser.ps1 -NumTrustees 3 -Threshold 2 -NumBallots 10 -BrowserTrusteeIndex 1
```

This will:
- Start B4 bulletin board server on http://127.0.0.1:3000
- Initialize a protocol with N trustees
- Start N-1 native trustees (e.g., trustee2 and trustee3 if browser is trustee1)
- Leave one slot open for the browser trustee
- Display the browser trustee configuration

**Note:** The script outputs the exact configuration needed for the browser - just copy/paste!

### 2. Start the Web Server

In a **new terminal**, build WASM and start the local server:

```powershell
.\serve.ps1
```

This starts a web server at http://127.0.0.1:8080 with proper CORS headers for SharedArrayBuffer support.

### 3. Open in Browser

Navigate to: http://127.0.0.1:8080/trustee.html

### 4. Configure and Connect

1. **Fill in Configuration:**
   - The `demo-browser.ps1` script displays the exact config needed
   - Either paste the individual fields OR use the JSON output
   - The config is also saved to `browser_trustee_config.json` for convenience

2. **Initialize Trustee:**
   - Click "Initialize Trustee"
   - Wait for success message

3. **Select Board:**
   - Click "Fetch Available Boards" - you should see "browser_test"
   - Click on the board to select it
   - Click "Connect" to join the session

4. **Run Protocol:**
   - Click "Execute Step" to manually run one protocol step
   - Or click "Auto (1s)" to run steps automatically every second
   - Watch the native trustee windows - they'll start processing too!
   - Monitor the progress bar as the protocol completes

## UI Components

### Configuration Panel
- **Trustee Name:** Identifier for this trustee
- **Signing Key:** Base64-encoded DER signing key (from `signing_key.der`)
- **Encryption Key:** Base64-encoded encryption key (from `config.json`)
- **B4 URL:** Bulletin board server address (default: http://127.0.0.1:8000)

### Board Selection Panel
- **Fetch Boards:** Query B4 for available boards
- **Board List:** Click to select a board
- **Manual Entry:** Or type board name directly
- **Connect:** Initialize session for selected board

### Protocol Execution Panel
- **Execute Step:** Fetch messages → process → post (one iteration)
- **Auto Mode:** Run steps continuously (1 per second)
- **Statistics:**
  - Current/Max Messages: Progress through protocol phases
  - Last Step: Messages added/posted in last step
  - Current Board: Active board name
- **Progress Bar:** Visual protocol completion indicator

### Console Output Panel
- Real-time Rust logs from WASM
- INFO (blue), WARN (orange), ERROR (red) messages
- Timestamped entries
- Clear button to reset

## Architecture

### WASM Implementation
- **LocalBoard:** In-memory message storage (no SQLite)
- **Crypto:** Full ed25519 signing + Ristretto group operations
- **Parallelism:** Rayon with atomics (requires SharedArrayBuffer)
- **Logging:** `tracing-wasm` routes Rust logs to browser console

### HTTP Communication
- **GET /boards:** List available boards
- **GET /messages/{board}?after_id={id}:** Fetch new messages
- **POST /messages/{board}:** Submit messages (currently stubbed)

### Protocol Flow
1. **Fetch:** GET new messages from B4 since last ID
2. **Process:** Run `trustee.step_public()` with new messages
3. **Post:** Submit generated messages back to B4
4. **Repeat:** Continue until protocol completes

## Multiple Browser Trustees

You can run multiple browser trustees simultaneously:

1. Open multiple browser tabs
2. Use different trustee configs (trustee1, trustee2, trustee3)
3. All connect to the same B4 server
4. Watch them coordinate the protocol together

## Development Notes

### Current Status
- ✅ WASM builds successfully
- ✅ Browser UI functional
- ✅ HTTP GET working (fetch boards, fetch messages)
- ✅ Protocol execution working
- ⚠️ HTTP POST stubbed (messages logged but not sent)
- ⚠️ Need to implement proper message serialization for B4

### Next Steps
1. Implement HTTP POST for submitting messages
2. Test with real multi-trustee sessions
3. Add message visualization (show board contents)
4. Add better error handling and retry logic
5. Consider IndexedDB persistence (optional)

### Known Limitations
- In-memory only (no persistence)
- POST not implemented (protocol runs but doesn't submit)
- No automatic reconnection on network errors
- Requires modern browser with SharedArrayBuffer support

## Browser Requirements

- Chrome/Edge 92+
- Firefox 95+
- Safari 15.2+
- Requires:
  - WebAssembly
  - SharedArrayBuffer (for rayon parallelism)
  - ES6 modules
  - Fetch API

## Troubleshooting

### "SharedArrayBuffer is not defined"
Make sure you're using the provided `serve.ps1` which sets required CORS headers.

### "Failed to fetch boards"
Check that demo_multi.ps1 is running and B4 is on port 8000.

### "WASM module not found"
Run `.\serve.ps1` which builds WASM before starting the server.

### "Invalid signing key"
Make sure you copied the full Base64 string from `get-trustee-config.ps1`.

## Files

- `trustee.html` - Browser UI
- `get-trustee-config.ps1` - Extract configs from demo environment
- `serve.ps1` - Build WASM and start web server
- `server.py` - HTTP server with CORS headers
- `crates/braid-wasm/src/trustee.rs` - WASM API implementation
- `crates/braid/src/protocol/board/local_wasm.rs` - In-memory LocalBoard

## License

AGPL-3.0-only
