# WBraid Migration Summary

## Overview

This document summarizes the migration of the Braid crate to support WebAssembly (WASM) targets alongside native builds, enabling bulletin board operations in browser environments.

## Completed Work

### 1. Strand WASM Compatibility
- **Verified** that the `strand` crate can build for `wasm32-unknown-unknown` target
- No modifications were required - strand was already WASM-compatible

### 2. B3 Feature Flags & HTTP Message Type
- **Added feature flags** to separate native and WASM dependencies:
  - `native` (default): Includes all dependencies including gRPC
  - `wasm-core`: WASM-compatible subset without gRPC or native-only dependencies
  
- **Created `HttpB3Message`**: A new message type for HTTP-based communication
  - Universal type (not feature-gated) - works in both native and WASM contexts
  - Serializable with serde for JSON/HTTP transport
  - Alternative to `GrpcB3Message` which requires Tonic (native-only)

- **Updated dependencies**:
  - Made `prost`, `tonic`, and `tower` conditional on `native` feature
  - Ensured `serde`, `serde_json`, and WASM-compatible deps available in `wasm-core`

### 3. Braid Feature Flags & WASM Support
- **Implemented feature gates** matching B3:
  - `native` (default): Full feature set including local storage
  - `wasm-core`: Browser-compatible subset

- **Refactored Board trait**:
  - Made uniform across native and WASM modes
  - Uses `HttpB3Message` as the universal message type
  - Removed native-specific dependencies from trait definition

- **Split implementations**:
  - **Native**: `Trustee` + `LocalBoard` (file-based storage)
  - **WASM**: `WasmTrustee` + `WasmLocalBoard` (stub implementations)
  - Separated to allow different storage backends (filesystem vs browser APIs)

- **Updated dependencies**:
  - Made `reqwest/native-tls` conditional on `native` feature
  - Made filesystem dependencies (`fs_extra`, etc.) conditional on `native` feature
  - Enabled `reqwest/json` for both modes

### 4. Testing & Validation
- **Native tests**: All existing tests continue to pass
  - `cargo test` runs successfully with default `native` feature
  - No regression in native functionality

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
   ```

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

## Technical Debt & Considerations

### Build Configuration
- **Current**: Atomics configuration in `crates/client/.cargo/config.toml`
- **Consideration**: May need to document minimum browser requirements for SharedArrayBuffer

### Environment Variable Management
- **Lesson learned**: Session-wide env vars in PowerShell can create hidden dependencies
- **Best practice**: Use scoped env vars or defensive cleanup in scripts
- **Current solution**: serve.ps1 clears RUSTFLAGS before building

### Feature Flag Complexity
- **Current**: Two features (`native` and `wasm-core`)
- **Future**: Consider if more granular features needed (e.g., `s3`, `indexeddb`)
- **Trade-off**: Simplicity vs flexibility

### Storage Abstraction
- **Current**: Separate LocalBoard and WasmLocalBoard types
- **Future**: Trait-based abstraction enables testing and alternative backends
- **Consideration**: Async trait methods may need `async-trait` crate or native async in trait

## Migration Checklist

- [x] Strand WASM compatibility verified
- [x] B3 feature flags implemented
- [x] HttpB3Message created and integrated
- [x] Braid feature flags implemented
- [x] Board trait unified with HttpB3Message
- [x] Trustee/WasmTrustee split implemented
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
