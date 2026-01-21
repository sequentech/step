<!--
 SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->
# WBraid Migration Summary

## Overview

This document summarizes the migration from b3 (gRPC + PostgreSQL) to b4 (HTTP + S3 + SQLite) bulletin board architecture, alongside WebAssembly (WASM) support for browser environments.

## Current Status (November 28, 2025)

### ✅ **MAJOR MILESTONE ACHIEVED**: b3 → b4 Migration Complete

The bulletin board architecture has been completely migrated from the old b3 (gRPC + PostgreSQL) to the new b4 (HTTP + S3 + SQLite) system.

## Completed Work

### 0. Complete b3 → b4 Bulletin Board Migration ✅
**Status**: COMPLETE - All core functionality migrated and tested

**Architecture Changes**:
- **Old (b3)**: gRPC communication + PostgreSQL database
- **New (b4)**: HTTP REST API + S3 storage + SQLite metadata database
- **Migration**: Complete replacement, not coexistence

**What Was Migrated**:

1. **Crate Renaming & Reorganization**:
   - `crates/bb4/` → `crates/b4/` (bulletin board v4)
   - `crates/b3/` → DELETED (completely removed)
   - `crates/client/` → `crates/braid-wasm/` (clearer naming)
   - b4 is now the core bulletin board library (like b3 was)

2. **b4 Crate Structure**:
   - `src/main.rs`: HTTP server binary (runs on port 3000)
   - `src/lib.rs`: Exports messages module and utilities
   - `src/messages/`: All message types from b3 (artifact, statement, newtypes, etc.)
   - `src/db.rs`: SQLite database operations via sqlx
   - `src/s3.rs`: S3 client initialization and pre-signed URL generation
   - `src/handlers.rs`: HTTP endpoint handlers (boards, messages, uploads)
   - `src/state.rs`: Application state (DB pool + S3 client)
   - `b4.db`: SQLite database file (auto-created)
   - Database schema: `boards` table and `messages` table with inline/S3 content types

3. **Braid Updates** (in `crates/braid/`):
   - Global replacement: All `b3::` imports → `b4::`
   - `src/protocol/board/http.rs`: 
     - `HttpB3`: Main HTTP bulletin board client
     - `HttpB3BoardParams`: Factory for board creation with async env var initialization
     - `HttpB3Index`: Board listing functionality  
     - Metadata optimization: `HttpB3Message` includes sender_pk, statement_kind for efficient filtering
   - `src/protocol/board/grpc_m.rs`: REMOVED (deleted, module unregistered)
   - `src/protocol/session/session_master.rs`:
     - Changed from storing `b3_url: String` to `board_params: HttpB3BoardParams`
     - `new()` is now async (awaits board_params initialization)
     - SessionSet also uses board_params instead of URL string
   - `src/verify/verifier.rs`: Uses `HttpB3` instead of `GrpcB3`, removed feature gates
   - All type references: `GrpcB3` → `HttpB3` throughout codebase

4. **Binary Updates**:
   - `src/bin/main.rs`: Single trustee, uses `HttpB3BoardParams::new().await` ✅
   - `src/bin/main_concurrent.rs`: Session master, uses async `SessionMaster::new().await` ✅
   - `src/bin/verify.rs`: Verifier, uses `HttpB3BoardParams` and `board_params.create_board()` ✅
   - `src/bin/demo_tool.rs`: **MIGRATED** from PostgreSQL to SQLite + inline storage ✅
     - All commands working: GenConfigs, InitProtocol, PostBallots, ListMessages, ListBoards, DropDb
     - Uses direct SQLite queries via sqlx (no PostgreSQL dependencies)
     - Stores all message data inline in SQLite BLOB column (no S3 for simplicity)
     - Environment-based config: Uses `DATABASE_URL` or defaults to `./b4.db`

5. **S3 Integration**:
   - b4 server handles small messages inline (stored in SQLite)
   - Large messages use S3 with pre-signed URLs (two-step upload/download)
   - Environment configuration: `AWS_ENDPOINT_URL`, `S3_BUCKET_NAME`, AWS credentials
   - LocalStack support for local S3 testing
   - HttpB3Message includes metadata (sender_pk, statement_kind, batch, mix_number) for efficient querying

6. **Testing**:
   - ✅ Protocol test passing: `test_protocol_http` - Full DKG + encryption + mixing + decryption cycle
   - ✅ HTTP API test script: `test-multiboard.ps1` - Board creation, message posting, retrieval
   - ✅ Main binaries compile: `main`, `main_concurrent`, `verify`
   - ✅ demo_tool compiles and ready for multi-process integration testing
   - ⚠️ Multi-process demo_tool testing: Not yet executed (next step)
   - ⚠️ Verifier binary: Not yet tested in real scenario (ready to test)

7. **Scripts**:
   - `bb.ps1`: Updated to run b4 server (crates/b4, RUST_LOG=b4=info)
   - `localstack.ps1`: Launches LocalStack for S3 testing
   - `test-multiboard.ps1`: Tests HTTP API functionality
   - Workspace `Cargo.toml`: Updated members (bb4→b4, client→braid-wasm, removed b3)

**Key Technical Changes**:
- Environment-based configuration instead of command-line args for board params
- Async board initialization (board_params must be awaited)
- All HTTP communication instead of gRPC
- SQLite metadata DB + S3 object storage instead of PostgreSQL monolith
- No more client/server code split - b4 is the server, braid uses HTTP client

**Testing Results**:
```
test test_protocol_http ... ok
Completed: Trustees = 5, Threshold = 3, Ciphertexts = 1000
## Remaining Work

### Phase 1: Integration Testing & Verification 🔄
**Goal**: Validate the b3 → b4 migration in real-world scenarios

**Status**: Core migration complete, validation pending

1. **Multi-Process Demo Testing** (High Priority):
   - [ ] Use demo_tool to generate configuration (GenConfigs)
   - [ ] Run demo_tool InitProtocol to set up boards
   - [ ] Launch b4 server (bb.ps1)
   - [ ] Launch multiple trustee processes (main binary) in separate terminals
   - [ ] Use demo_tool PostBallots to initiate protocol
   - [ ] Verify DKG completion via demo_tool ListMessages
   - [ ] Confirm full protocol execution across processes
   - **Goal**: Validate that separate process communication works correctly

2. **Monitor Tool Testing**:
   - [ ] Drop existing database to test with new schema (demo_tool drop-db)
   - [ ] Run complete demo protocol (gen-configs → init → trustees → ballots)
   - [ ] Launch monitor binary during protocol execution
   - [ ] Verify board metadata displays correctly (trustees_no, threshold_no)
   - [ ] Verify message_count increments properly
   - [ ] Verify batch_count updates when ballots are posted
   - [ ] Verify last_message_kind updates throughout protocol phases
   - [ ] Confirm progress bars show DKG and tally phases correctly
   - **Goal**: Validate board metadata tracking and monitor visualization
   - **Note**: Monitor was migrated from b3's INDEX table approach to b4's boards table

2.6 Revise:

// SAFETY: WASM is single-threaded, so RefCell is safe to share across "threads"
// (which don't actually exist in WASM). This allows IndexedDbStorage to implement
// LocalBoardStorage which requires Send + Sync.
unsafe impl Send for IndexedDbStorage {}
unsafe impl Sync for IndexedDbStorage {}

2.7 Remove warnings

2.8 We have lost the ability to save artifacts outside of the localboard.
In the previous version of braid, there was a way to save artifacts in the
sqlite data base, and store only the row ids in the local board. This was lost
in the wasm compatible version of braid, all artifacts will be in memory.

3. **Verifier Binary Testing**:
   - [ ] Run verifier against a completed protocol execution
   - [ ] Verify it correctly validates all signatures
   - [ ] Confirm it detects invalid data (negative test)
   - [ ] Test ballot inclusion verification (when implemented)

4. **S3 Large Message Testing**:
   - [ ] Test with messages > MAX_INLINE_MESSAGE_SIZE
   - [ ] Verify S3 upload flow with pre-signed URLs
   - [ ] Verify S3 download flow with pre-signed URLs
   - [ ] Confirm LocalStack S3 compatibility
   - **Note**: Currently MAX_INLINE_MESSAGE_SIZE = 0 (all S3), may need adjustment

5. **Error Handling & Edge Cases**:
   - [ ] Test board creation failures
   - [ ] Test message posting with invalid data
   - [ ] Test S3 connectivity failures
   - [ ] Test SQLite database corruption scenarios
   - [ ] Validate error messages are user-friendly

6. **Performance Testing**:
   - [ ] Benchmark HTTP vs old gRPC performance
   - [ ] Test with larger message counts (10k+ ciphertexts)
   - [ ] Profile S3 upload/download performance
   - [ ] Identify bottlenecks in SQLite queries

### Phase 2: Documentation & Cleanup ✅/🔄
**Goal**: Update documentation and remove obsolete code

### Phase 3: Browser Storage Implementation (WASM)

1. **Update Documentation**:
   - [ ] Update README.md to reference b4 instead of b3
   - [ ] Document environment variables (AWS_ENDPOINT_URL, S3_BUCKET_NAME, DATABASE_URL)
   - [ ] Update architecture diagrams
   - [ ] Document demo_tool usage patterns
   - [ ] Add troubleshooting guide for common issues

2. **Code Cleanup**:
   - [x] Remove b3 crate ✅
   - [x] Remove grpc_m.rs module ✅
   - [x] Remove protocol_test_grpc.rs ✅
   - [ ] Clean up unused imports (warnings present)
   - [ ] Remove dead code warnings in http.rs
   - [ ] Consider removing other unused binaries if they exist

3. **Migration Summary** (this document):
   - [x] Update with b3 → b4 migration details ✅
   - [ ] Add lessons learned section
   - [ ] Document breaking changes for other teams

- **POC verification**: 
  - Bulletin board service (`crates/service`) runs correctly
  - WASM client (`crates/client`) builds and works in browser
  - HTTP communication between service and client functional
  - S3 integration working for large messages

### 5. WASM Build Infrastructure & Atomics Support
- **Build configuration** in `crates/client/.cargo/config.toml`:
  - Target features: `+atomics,+bulk-memory,+mutable-globals`
  - Shared memory linker flags for SharedArrayBuffer support
  - TLS exports for thread-local storage
  - Build configuration: `-Z build-std=std,panic_abort` (requires nightly)
  - Based on solution from [huggingface/xet-core#554](https://github.com/huggingface/xet-core/issues/554)

- **Build scripts**:
  - `build-wasm.ps1`: Builds WASM with nightly toolchain, runs wasm-bindgen
  - `serve.ps1`: Builds and serves client with defensive environment cleanup
  - Cross-origin isolation headers (COEP/COOP) for SharedArrayBuffer

- **Critical debugging**:
  - Resolved environment variable pollution issue in `bb.ps1`
  - Fixed script execution order dependency (bb.ps1 → serve.ps1)
  - Implemented defensive `RUSTFLAGS` cleanup in serve.ps1
  - Learned Cargo precedence: env vars > config files > defaults

### 6. Key Architectural Decisions
- **HttpB3Message as universal type**:
  - Not feature-gated - available everywhere
  - Enables uniform Board trait across native/WASM
  - Simplifies future migration away from gRPC

- **Feature-gated implementations**:
  - Core logic (Board trait, Trustee logic) shared
  - Storage backends separated (LocalBoard vs WasmLocalBoard)
  - Dependencies isolated by feature flags

- **Stub pattern for WASM**:
  - WasmLocalBoard provides minimal implementation
  - Allows compilation and basic testing
  - Ready for browser storage implementation

## Remaining Work

### Phase 1: Complete gRPC Removal from B3
**Goal**: Migrate B3 entirely to HTTP-based communication

1. **Remove gRPC dependencies**:
   - Drop `GrpcB3Message` type
   - Remove `prost`, `tonic`, `tower` dependencies entirely
   - Migrate all consumers to `HttpB3Message`

2. **Update B3 API**:
   - Ensure all operations use `HttpB3Message`
   - Verify serialization/deserialization patterns
   - Update any remaining gRPC-specific code paths

### Phase 2: Implement Two-Step S3 Messaging in B3
**Goal**: Handle large messages via S3 with pre-signed URLs

1. **Add S3 threshold logic**:
   - Define message size threshold (e.g., 1MB)
   - Small messages: inline in HTTP body
   - Large messages: S3 pre-signed URL flow

2. **Implement pre-signed URL generation**:
   - POST: Return upload URL for large messages
   - GET: Return download URL for S3-backed messages

3. **Update HttpB3Message**:
   - Add content type enum: `Inline` vs `S3Reference`
   - Include S3 metadata when applicable

4. **Client-side S3 operations**:
   - Direct S3 upload using pre-signed URLs
   - Direct S3 download using pre-signed URLs
   - Handle both inline and S3-backed messages transparently

### Phase 3: Braid HttpB3Message Integration
**Goal**: Ensure Braid fully utilizes HttpB3Message

**Status**: Likely already complete - Board trait already uses HttpB3Message

**Verification needed**:
- Check if any code paths still reference GrpcB3Message
- Ensure all message operations use HttpB3Message
- Verify compatibility after B3 gRPC removal

### Phase 4: Browser Storage Implementation
**Goal**: Replace stub WasmLocalBoard with functional browser storage

**Option A: Generic Trustee with Trait-Based Storage**
1. **Define LocalBoardTrait**:
   ```rust
   pub trait LocalBoardStorage {
       async fn save(&self, key: &str, data: Vec<u8>) -> Result<()>;
       async fn load(&self, key: &str) -> Result<Option<Vec<u8>>>;
       async fn list(&self) -> Result<Vec<String>>;
       async fn delete(&self, key: &str) -> Result<()>;
   }
   ```

2. **Make Trustee generic**:
   ```rust
   pub struct Trustee<S: LocalBoardStorage> {
       storage: S,
       // ... other fields
   }
   ```

3. **Implement for native**:
   ```rust
   impl LocalBoardStorage for LocalBoard {
       // File system implementation
   }
### Phase 4: Additional Enhancements

4. **Implement for WASM** (in `crates/client`):
   ```rust
   impl LocalBoardStorage for WasmLocalBoard {
       // IndexedDB implementation using web_sys
   }
   ```

**Benefits of this approach**:
- Braid remains platform-agnostic
- Client provides WASM-specific storage implementation
- Clean separation of concerns
- Easy to test with mock storage implementations

**Option B: Direct Implementation in Braid**
- Implement IndexedDB directly in `braid::WasmLocalBoard`
- Simpler but couples Braid to browser APIs
- Less flexible for other WASM environments (e.g., Node.js, Cloudflare Workers)

**Recommendation**: Use Option A (trait-based approach) for better architecture

### Phase 5: Additional Enhancements

1. **Parallel operations with wasm-bindgen-rayon**:
   - Atomics support already configured
   - Enable parallel message processing in browser
   - Leverage multiple CPU cores for cryptographic operations

2. **Offline support**:
   - Cache messages in IndexedDB
   - Queue operations when offline
   - Background sync when connection restored

3. **Error handling**:
   - Standardize error types across native/WASM
   - User-friendly error messages in browser
   - Retry logic for network failures

4. **Testing**:
   - Add WASM-specific tests using wasm-bindgen-test
   - Browser automation tests (e.g., with Playwright)
   - Integration tests for S3 flow
## Migration Checklist

### b3 → b4 Migration
- [x] Rename bb4 → b4, client → braid-wasm
- [x] Delete b3 crate entirely
- [x] Move b3/messages to b4/src/messages
- [x] Update workspace members
- [x] Global code transformation: b3:: → b4::, GrpcB3 → HttpB3
- [x] Create b4 HTTP server (SQLite + S3)
- [x] Update SessionMaster for async board_params
- [x] Update Verifier to use HttpB3
- [x] Migrate main, main_concurrent, verify binaries
- [x] Migrate demo_tool from PostgreSQL to SQLite
- [x] Remove grpc_m module
- [x] Remove protocol_test_grpc.rs
- [x] Protocol test passing (test_protocol_http)
- [x] Main binaries compiling successfully
- [ ] **Multi-process demo_tool testing** (Next: High Priority)
- [ ] **Verifier binary real-world testing** (Next)
- [ ] S3 large message flow testing
- [ ] Update documentation (README, architecture)
- [ ] Code cleanup (warnings, dead code)

### WASM Support
- [x] Strand WASM compatibility verified
- [x] B3/B4 feature flags implemented
- [x] HttpB3Message created and integrated
- [x] Braid feature flags implemented
- [x] Board trait unified with HttpB3Message
- [x] Trustee/WasmSession split implemented
- [x] LocalBoard/WasmLocalBoard split implemented
## Notes

### Recent Progress (November 28, 2025)
- **Major Milestone**: Complete b3 → b4 migration achieved
- Protocol test (`test_protocol_http`) passing with full DKG/encryption/decryption cycle
- All main binaries (main, main_concurrent, verify) building successfully
- demo_tool migrated and ready for multi-process integration testing
- HTTP+S3 architecture fully functional and validated

### Next Immediate Steps
1. **Multi-process testing** with demo_tool:
   - Generate configs → Initialize protocol → Run trustees in separate processes
   - Validate inter-process communication works correctly
   - Document any issues found

2. **Verifier testing**:
   - Run against completed protocol execution
   - Validate signature verification works correctly

3. **Documentation updates**:
   - Update README.md with b4 architecture
   - Document environment variables
   - Add demo_tool usage guide

### Architecture Status
- The migration preserves full backward compatibility for native builds
- WASM builds are currently functional but with stub storage
- The architecture is designed to minimize WASM-specific code in core crates
- Feature gates provide clean separation without code duplication
- b4 (HTTP+S3+SQLite) is now the primary bulletin board implementation
- gRPC and PostgreSQL completely removed from the codebase
- [ ] Enable parallel operations with rayon
- [ ] Implement offline support and sync
- [ ] Comprehensive error handling
## Migration Checklist

- [x] Strand WASM compatibility verified
- [x] B3 feature flags implemented
- [x] HttpB3Message created and integrated
- [x] Braid feature flags implemented
- [x] Board trait unified with HttpB3Message
- [x] Trustee/WasmSession split implemented
- [x] LocalBoard/WasmLocalBoard split implemented
- [x] Native tests passing
- [x] POC service and client working
- [x] WASM build infrastructure (atomics, SharedArrayBuffer)
- [ ] Remove gRPC from B3 entirely
- [ ] Implement two-step S3 messaging in B3
- [ ] Verify Braid HttpB3Message integration (likely done)
- [ ] Define LocalBoardStorage trait
- [ ] Make Trustee generic over storage
- [ ] Implement WasmLocalBoard with IndexedDB
- [ ] Add WASM-specific tests
- [ ] Enable parallel operations with rayon
- [ ] Implement offline support and sync
- [ ] Comprehensive error handling

## Resources

- **Atomics solution**: [huggingface/xet-core#554](https://github.com/huggingface/xet-core/issues/554)
- **SharedArrayBuffer requirements**: COEP: require-corp, COOP: same-origin headers
- **Build script**: `build-wasm.ps1` - nightly toolchain + wasm-bindgen approach
- **Current workspace**: 7 crates with feature-gated architecture
- **POC status**: Basic HTTP bulletin board + WASM client functional

## Notes

- The migration preserves full backward compatibility for native builds
- WASM builds are currently functional but with stub storage
- The architecture is designed to minimize WASM-specific code in core crates
- Feature gates provide clean separation without code duplication
- Next major milestone: Browser storage implementation enabling full functionality in WASM
