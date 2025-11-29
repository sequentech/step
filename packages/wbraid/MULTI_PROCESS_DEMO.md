# Multi-Process Demo Testing

This guide demonstrates running the complete wbraid protocol with multiple trustee processes, the b4 bulletin board server, and LocalStack S3 storage.

## Prerequisites

1. **Start LocalStack** (S3 service on port 4566)
2. **Start b4 server**:
   ```powershell
   .\b4.ps1
   ```
   This sets `DATABASE_URL=sqlite:b4.db` and starts the server on port 3000

## Demo Steps

### 1. Generate Configuration

Generate configuration for 3 trustees with threshold 2:

```powershell
cargo run --bin demo_tool --release --features native -- gen-configs --num-trustees 3 --threshold 2
```

This creates the `demo/` directory with:
- `config.bin` - Protocol configuration
- `pm.toml` - Protocol manager config
- `1/trustee.toml`, `2/trustee.toml`, `3/trustee.toml` - Trustee configs

### 2. Initialize Protocol

Create the board and post the configuration message:

```powershell
cargo run --bin demo_tool --release --features native -- init-protocol --board-name test
```

### 3. Start Trustees

Open **three separate terminals** and start each trustee.

You can use either the **basic trustee binary** (`main`) or the **advanced trustee binary** (`main_concurrent` with multiplexing and concurrency support):

#### Option A: Basic Trustee Binary (`main`)

**Terminal 1:**
```powershell
cd demo/1
cargo run --bin main --release --features native -- --b3-url http://localhost:3000 --trustee-config trustee.toml
```

**Terminal 2:**
```powershell
cd demo/2
cargo run --bin main --release --features native -- --b3-url http://localhost:3000 --trustee-config trustee.toml
```

**Terminal 3:**
```powershell
cd demo/3
cargo run --bin main --release --features native -- --b3-url http://localhost:3000 --trustee-config trustee.toml
```

#### Option B: Advanced Trustee Binary (`main_concurrent`)

The `main_concurrent` binary supports multiplexing, concurrency, and chunking for improved performance:

**Terminal 1:**
```powershell
cd demo/1
cargo run --bin main_concurrent --release --features native -- --b3-url http://localhost:3000 --trustee-config trustee.toml
```

**Terminal 2:**
```powershell
cd demo/2
cargo run --bin main_concurrent --release --features native -- --b3-url http://localhost:3000 --trustee-config trustee.toml
```

**Terminal 3:**
```powershell
cd demo/3
cargo run --bin main_concurrent --release --features native -- --b3-url http://localhost:3000 --trustee-config trustee.toml
```

**Note:** Both binaries are fully compatible with the b4 server and produce identical protocol results. The `main_concurrent` binary offers better performance through concurrent action execution and request multiplexing.

The trustees will automatically complete the **DKG phase** (Distributed Key Generation):
- Sign configuration
- Generate channels
- Compute shares
- Generate public key

### 4. Post Ballots

Once DKG completes (trustees show "Posting 0 messages"), post ballots to start the tally:

```powershell
cargo run --bin demo_tool --release --features native -- --board-name test post-ballots
```

This generates 100 random encrypted ballots.

### 5. Verify Completion

The trustees will automatically process the ballots through:
1. **Mix phase** - Shuffle ballots (2 mixes)
2. **Decryption phase** - Compute decryption factors
3. **Plaintexts phase** - Decrypt and verify

List all messages to see the complete protocol:

```powershell
cargo run --bin demo_tool --release --features native -- --board-name test list-messages
```

Or filter to see just the tally phase:

```powershell
cargo run --bin demo_tool --release --features native -- --board-name test list-messages 2>&1 | Select-String -Pattern "Ballots|Mix|DecryptionFactors|Plaintexts"
```

## Expected Message Flow

**DKG Phase (16 messages):**
1. Configuration
2. ConfigurationSigned (×3)
3. Channel (×3)
4. ChannelsAllSigned (×3)
5. Shares (×3)
6. PublicKey
7. PublicKeySigned (×2)

**Tally Phase (10 messages):**
1. Ballots
2. Mix (×2)
3. MixSigned (×2)
4. DecryptionFactors (×2)
5. Plaintexts
6. PlaintextsSigned (×2)

## Storage Architecture

- **Server Database**: `wbraid.db` (SQLite) at workspace root
- **Message Storage**: 
  - Small messages (< 1KB): Inline in database
  - Large messages: S3 bucket "wbraid-messages" (LocalStack)
- **Trustee Local Storage**: `demo/{1,2,3}/message_store/test/` (SQLite)

## Cleanup

To reset for a new demo run:

```powershell
# Stop all trustees (Ctrl+C in each terminal)
# Stop b4 server (Ctrl+C)
# Delete demo directory
Remove-Item -Recurse -Force demo
# Delete database
Remove-Item wbraid.db
# Restart b4 server
.\b4.ps1
```
