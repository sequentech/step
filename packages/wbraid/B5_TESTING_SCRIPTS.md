# B5 Testing Scripts - Summary

## Overview
Created B5 equivalents of the existing testing scripts with support for the new **ciphertext width (W)** parameter.

## Scripts Created

### 1. **demo-multi-b5.ps1** ✅
- **Purpose**: Native-only multi-board protocol demo
- **Equivalent to**: demo-multi.ps1
- **NEW Parameters**:
  - `-CiphertextWidth <int>` (default: 2) - Sets the W parameter for ciphertext width
- **Usage Examples**:
  ```powershell
  # Quick test with default W=2
  .\demo-multi-b5.ps1 -QuickTest
  
  # Test with W=1 (single element ciphertexts)
  .\demo-multi-b5.ps1 -NumTrustees 3 -Threshold 2 -NumBallots 10 -CiphertextWidth 1
  
  # Test with W=3
  .\demo-multi-b5.ps1 -NumTrustees 5 -Threshold 3 -NumBallots 100 -CiphertextWidth 3
  
  # Test with W=4
  .\demo-multi-b5.ps1 -CiphertextWidth 4 -QuickTest
  ```

### 2. **demo-browser-b5.ps1** ✅
- **Purpose**: Browser + Native trustees demo
- **Equivalent to**: demo-browser.ps1
- **NEW Parameters**:
  - `-CiphertextWidth <int>` (default: 2) - Sets the W parameter
- **Prerequisites**:
  - LocalStack running (`.\localstack.ps1`)
  - WASM build complete (`.\build-wasm-b5.ps1`)
- **Usage Examples**:
  ```powershell
  # Default W=2
  .\demo-browser-b5.ps1
  
  # Test with W=1
  .\demo-browser-b5.ps1 -NumTrustees 3 -Threshold 2 -NumBallots 10 -CiphertextWidth 1
  
  # Custom browser trustee slot with W=3
  .\demo-browser-b5.ps1 -BrowserTrusteeIndex 2 -CiphertextWidth 3
  ```

## Existing Scripts (Already Available)

### 3. **b5.ps1** ✅
- **Purpose**: Launch B5 bulletin board server
- **Equivalent to**: b4.ps1
- **Changes**: Targets `b5` package instead of `b4`

### 4. **build-wasm-b5.ps1** ✅
- **Purpose**: Build braid_b5 for WebAssembly
- **Equivalent to**: build-wasm.ps1
- **Already exists**: Located in workspace root

## Key Differences from Original Scripts

### Package Targeting
- **Old**: `--bin demo_tool` (from braid package)
- **New**: `--package braid_b5 --bin demo_tool`

- **Old**: `--bin b4` (from b4 package)
- **New**: `--package b5 --bin b5`

- **Old**: `--bin main_concurrent` (from braid)
- **New**: `--package braid_b5 --bin main_concurrent`

### Ciphertext Width Parameter
All new scripts accept `-CiphertextWidth <int>` parameter:
- Passed to `demo_tool gen-configs --ciphertext-width $CiphertextWidth`
- Default value: 2 (backward compatible with old code)
- Supported values: 1, 2, 3, 4 (per dispatch macro)

### Stack Usage
- **Messages**: b5 (instead of b4)
- **Protocol**: braid_b5 (instead of braid)
- **Crypto**: cryptography (instead of strand)
- **Server**: b5 binary (instead of b4 binary)

## Testing Workflow

### Native-Only Testing (Simplest)
```powershell
# 1. Clean build
cargo build --package braid_b5 --package b5 --release

# 2. Test with W=2 (default, backward compatible)
.\demo-multi-b5.ps1 -QuickTest

# 3. Test with W=1
.\demo-multi-b5.ps1 -QuickTest -CiphertextWidth 1

# 4. Test with W=3
.\demo-multi-b5.ps1 -QuickTest -CiphertextWidth 3

# 5. Test with W=4
.\demo-multi-b5.ps1 -QuickTest -CiphertextWidth 4
```

### Browser Testing (Full Stack)
```powershell
# 1. Start LocalStack (required for S3)
.\localstack.ps1

# 2. Build WASM (in separate terminal)
.\build-wasm-b5.ps1

# 3. Run browser demo (in another terminal)
.\demo-browser-b5.ps1 -CiphertextWidth 2

# 4. Follow on-screen instructions to connect browser trustee
```

## What Each Script Does

### demo-multi-b5.ps1 Flow:
1. **Cleanup**: Remove old databases and processes
2. **Generate Config**: Create trustee configs with W parameter
3. **Start B5 Server**: Launch bulletin board on port 3000
4. **Initialize Boards**: Create N boards with shared config
5. **Start Trustees**: Launch M native trustees (minimized windows)
6. **Wait for DKG**: Pause for key generation
7. **Post Ballots**: Add encrypted ballots to boards
8. **Monitor Progress**: Check for completion
9. **Verify**: Confirm all messages present

### demo-browser-b5.ps1 Flow:
1. **Prerequisites Check**: Verify LocalStack & WASM build
2. **Cleanup**: Remove old state
3. **Generate Config**: Create trustee configs with W parameter
4. **Start B5 Server**: Launch bulletin board
5. **Initialize Board**: Single board for browser test
6. **Extract Browser Config**: Get keys for browser trustee
7. **Start Web Server**: Python http.server on port 8080
8. **Start Native Trustees**: All except one slot
9. **Display Instructions**: Show browser setup steps
10. **Wait for User**: Pause until browser connects
11. **Post Ballots**: After browser trustee ready
12. **Monitor**: Wait indefinitely (Ctrl+C to exit)

## Binaries Used

### From braid_b5:
- `demo_tool`: Generate configs, init protocol, post ballots, list messages
- `gen_trustee_config`: Generate individual trustee configs
- `main_concurrent`: Run trustee instance (single or multi-board)

### From b5:
- `b5`: Bulletin board server (HTTP API + SQLite + S3)

## Configuration Files

### Trustee Config (trustee.toml):
```toml
signing_key_sk = "<base64>"
signing_key_pk = "<base64>"
encryption_key = "<base64>"
```

### Generated by:
```powershell
cargo run --package braid_b5 --bin demo_tool --release -- gen-configs \
    --num-trustees 5 \
    --threshold 3 \
    --ciphertext-width 2
```

## Environment Variables (B5 Server)

```powershell
$env:RUST_LOG = "b5=info"
$env:DATABASE_URL = "sqlite:b4.db?mode=rwc"
$env:AWS_ENDPOINT_URL = "http://localhost:4566"
$env:AWS_ACCESS_KEY_ID = "test"
$env:AWS_SECRET_ACCESS_KEY = "test"
$env:AWS_REGION = "us-east-1"
$env:S3_BUCKET_NAME = "wbraid-messages"
$env:AWS_FORCE_PATH_STYLE = "true"
```

## Success Criteria

✅ **Compilation**: All packages compile without errors
✅ **W=2 Test**: Protocol runs end-to-end with default W=2
✅ **W=1 Test**: Protocol runs with single-element ciphertexts
✅ **W=3 Test**: Protocol runs with 3-element ciphertexts
✅ **W=4 Test**: Protocol runs with 4-element ciphertexts
✅ **Browser**: WASM trustee can participate alongside native trustees
✅ **Verification**: DKG, Mixing, and Decryption all complete successfully

## Troubleshooting

### "Failed to generate configuration"
- Check that braid_b5 builds successfully
- Verify demo_tool binary exists

### "b5 server failed to start"
- Check port 3000 isn't in use
- Verify LocalStack is running (for S3 features)
- Look at b5 server window for errors

### "Browser trustee can't connect"
- Ensure build-wasm-b5.ps1 ran successfully
- Check crates/braid_b5/pkg/ directory exists
- Verify web server running on port 8080

### "Trustees stuck at DKG"
- For browser demo: ensure browser trustee connected
- Check trustee windows for errors
- Verify threshold ≤ number of active trustees

## Next Steps

1. ✅ **Scripts Created**: demo-multi-b5.ps1, demo-browser-b5.ps1
2. ⏳ **Run Tests**: Execute with different W values
3. ⏳ **Verify Results**: Check protocol completion
4. ⏳ **Test Browser**: Verify WASM integration
5. ⏳ **Performance**: Benchmark W=1,2,3,4

## Notes

- **Database**: Scripts still use `b5.db` filename for backward compatibility
- **Monitor Tool**: Original demo-multi.ps1 uses a monitor binary - not ported yet to B5
- **Serve Script**: Can use existing serve.ps1 or create serve-b5.ps1 if needed
- **Quick Test**: `-QuickTest` flag reduces trustees/ballots for fast validation
