# WASM Storage Architecture - Current Status

## Problem Statement

We need **persistent, tamper-resistant storage** for trustee messages in the browser to protect against an adversarial bulletin board. The native implementation uses SQLite with AUTOINCREMENT IDs to ensure locally-controlled message ordering.

## Why Simple Solutions Don't Work

### Attempt 1: Direct OPFS Usage (`storage_browser.rs`)
**Status:** ❌ Not a real solution

- Tried to use OPFS (Origin Private File System) directly from main thread
- OPFS's `createSyncAccessHandle()` **requires Web Worker context**
- All async OPFS code fails on main thread
- Result: Just a NoOpStorage clone with broken persistence

### Attempt 2: SQLite WASM with OPFS (`storage_sqlite.rs`)
**Status:** ❌ Same problem

- `sqlite_wasm_rs` with OPFS VFS backend works great... in Workers
- But fails when called from main thread where `LocalBoard` runs
- Same `createSyncAccessHandle()` limitation

### Attempt 3: Storage Proxy (`storage_proxy.rs`)
**Status:** ⚠️ Architecturally sound but blocked

**The fundamental problem:**
```
LocalBoardStorage trait    →  Synchronous methods
Worker communication       →  Asynchronous (message passing)
WASM blocking primitives   →  Don't exist
```

We **cannot** implement true synchronous blocking in WASM:
- No `std::thread::sleep()` (not available in `wasm32-unknown-unknown`)
- No blocking `recv()` that yields to event loop
- Worker responses arrive via async callbacks only

## What We Built (For Future Use)

The storage proxy infrastructure is complete and ready for when the architecture supports it:

1. **`storage_proxy.rs`** - Proxy that forwards `LocalBoardStorage` calls to worker
   - Request/response serialization
   - Message protocol defined
   - Worker communication scaffolded
   - **Blocked on:** sync/async mismatch

2. **`storage_sqlite.rs`** - Full SQLite implementation with WASM bindings
   - Identical schema to native version
   - WASM-bindgen exports for JavaScript
   - **Works perfectly** in worker context
   - **Blocked on:** needs to run in worker, not main thread

3. **`init-storage-worker.js`** - Dedicated storage worker
   - Initializes OPFS VFS
   - Instantiates `SqliteStorage`
   - Handles storage operation requests
   - **Blocked on:** can't communicate synchronously with main thread

## Current Workaround

**Using `BrowserStorage`** (transient localStorage):
- ✅ Works on main thread (synchronous)
- ✅ Satisfies `LocalBoardStorage` interface
- ❌ Messages cleared after each retrieval
- ❌ No persistence across page reloads
- ❌ **Not suitable for production** (security requirement)

## Path Forward: Pick One

### Option 1: Make LocalBoardStorage Async (Recommended)

**Pros:**
- Clean architecture
- Enables true OPFS persistence
- Future-proof design

**Cons:**
- Large refactor (trait + all implementations)
- Propagates through `LocalBoard`, `Session`, `Trustee`
- Need to audit all call sites

**Effort:** Medium-Large (2-3 days)

### Option 2: Run Session Entirely in Worker

**Pros:**
- No trait changes needed
- Storage works immediately
- Clear thread boundary

**Cons:**
- Complicates UI communication
- All session state lives in worker
- Need message protocol for UI updates

**Effort:** Medium (2-3 days)

### Option 3: Implement IndexedDB Storage

**Pros:**
- IndexedDB has async API that works on main thread
- Could use `wasm-bindgen-futures` to await operations
- Still synchronous from Rust perspective (via blocking on promises)

**Cons:**
- Different from native (SQLite vs IndexedDB)
- Still hits sync/async mismatch
- May need custom AUTOINCREMENT logic

**Effort:** Medium (similar to async refactor)

### Option 4: Accept Transient Storage (Not Recommended)

**Pros:**
- Works today
- No code changes

**Cons:**
- **Security requirement not met**
- Not suitable for production
- Messages lost on reload

**Effort:** Zero (current state)

## Recommendation

**Option 1 (Make LocalBoardStorage Async)** is the best long-term solution:

1. More web applications are moving to WASM
2. Async storage is the web platform reality
3. Clean architecture that handles both native and web
4. Enables other async improvements (network, crypto)

**Estimated changes:**
```rust
// Before
trait LocalBoardStorage {
    fn store_messages(&self, ...) -> Result<()>;
    fn retrieve_messages(&self, ...) -> Result<Vec<...>>;
}

// After  
#[async_trait(?Send)]  // ?Send for WASM compatibility
trait LocalBoardStorage {
    async fn store_messages(&self, ...) -> Result<()>;
    async fn retrieve_messages(&self, ...) -> Result<Vec<...>>;
}
```

Then `StorageProxy` can use proper async worker communication, and `SqliteStorage` runs happily in its worker with full OPFS support.

## Files Modified (This Session)

- ✅ `crates/braid/src/wasm/board/storage_proxy.rs` - Created (scaffolded)
- ✅ `crates/braid/src/wasm/board/storage_sqlite.rs` - Added WASM bindings
- ✅ `crates/braid/src/wasm/board/storage_browser.rs` - Documented limitations
- ✅ `crates/braid/src/wasm/board/mod.rs` - Exports updated
- ✅ `crates/braid/src/wasm/session.rs` - Documented storage limitation
- ✅ `init-storage-worker.js` - Full worker implementation
- ✅ `trustee.html` - Reverted to BrowserStorage usage

All infrastructure is in place and waiting for the async refactor.
