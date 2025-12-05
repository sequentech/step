# IndexedDB Storage Implementation

## Overview

Browser-based persistent storage using a **metadata-only approach** that achieves the same security properties as SQLite AUTOINCREMENT without storing full messages.

## Security Model

### Three Security Properties

1. **Append-Only**: Ordered hash list provides tamper detection
2. **Tamper-Resistant**: Any modification to historical messages detected via hash mismatch  
3. **Locally-Controlled Ordering**: Hash list position = local ID (equivalent to AUTOINCREMENT)

### How It Works

Instead of storing full messages (like SQLite), we store:
- `hash_list: Vec<Hash>` - Ordered hashes (insertion order)
- `metadata_set: HashSet<MessageMetadata>` - Duplicate detection

This provides ~380x storage reduction while maintaining security:
- **SQLite**: 1000 messages × 50KB = 50MB
- **IndexedDB**: 1000 messages × 132 bytes = 132KB

## Verification Algorithm

Given:
- **S** = Total hashes in metadata store
- **B** = Messages already in LocalBoard (`last_local_board_id`)
- **Messages** = Response from bulletin board

### Verification Rule

```
if B > S:
    ERROR: Corruption (LocalBoard ahead of metadata)

verify_count = S - B

for i in 0..verify_count:
    if hash(messages[i]) != hash_list[B + i]:
        ERROR: Tamper detected!

for msg in messages[verify_count..]:
    if metadata_set.contains(msg.metadata):
        if ignore_existing: skip
        else: ERROR: Duplicate
    
    hash_list.push(hash(msg))
    metadata_set.insert(msg.metadata)
```

### Handles All Cases

1. **Fresh restart (B=0, S>0)**: Verify all S messages, accept new ones
2. **Partial restart (0 < B < S)**: Verify remaining (S-B) messages, accept new ones
3. **Normal operation (B=S)**: No verification needed, just append new messages
4. **Corruption (B > S)**: Reject, force reload

## Storage Strategy

### Persistent (IndexedDB)
- `hash_list` - Tamper detection across sessions
- `metadata_set` - Duplicate prevention across sessions

### Transient (In-Memory, NOT Persisted)
- `last_external_id` - Optimization within session only
- `message_buffer` - Messages between store/retrieve calls

### Why No Persistent last_external_id?

Unlike native SQLite which stores full messages, this implementation only stores metadata. On session restart:
1. All messages must be re-fetched from bulletin board
2. Messages verified against stored hashes
3. LocalBoard rebuilt from fetched messages

Therefore `last_external_id` only optimizes fetching **within a session**.

## Usage

### Initialization (Async, Session Start)

```rust
use braid::wasm::board::IndexedDbStorage;

// Create storage
let storage = IndexedDbStorage::new("trustee_db".to_string());

// Initialize: open IndexedDB, load metadata
storage.init().await?;

// Create trustee with storage
let trustee = Trustee::new(storage);
```

### Protocol Step (Synchronous)

```rust
// Fetch messages from bulletin board
let messages = board.get_messages(&board_name, last_external_id).await?;

// Process via trustee (storage operations are synchronous)
let step_result = trustee.step(&messages)?;
```

### Persistence (Async, After Step)

```rust
// Persist metadata to IndexedDB
storage.persist().await?;
```

## Implementation Details

### Data Structures

```rust
struct MessageMetadata {
    sender_pk: String,
    statement_kind: String,
    batch: i32,
    mix_number: i32,
}

struct PersistentMetadata {
    hash_list: Vec<Hash>,              // Ordered hashes
    metadata_set: HashSet<MessageMetadata>,  // Unique constraint
}

struct TransientState {
    last_external_id: i64,             // Optimization
    message_buffer: Vec<HttpB3Message>,  // Temporary buffer
}
```

### Hash Computation

Uses Strand's built-in cryptographic hash:
```rust
fn compute_hash(msg: &HttpB3Message) -> Result<Hash> {
    let bytes = msg.strand_serialize()?;
    Ok(Hash::compute(&bytes))
}
```

### IndexedDB Operations

**Single Key-Value Store:**
```javascript
Database: "trustee_db"
Store: "metadata"
Key: "persistent"
Value: bincode::serialize(PersistentMetadata)
```

**Load:**
```rust
let metadata = load_metadata(&db).await?;
```

**Save:**
```rust
let bytes = bincode::serialize(&metadata)?;
save_metadata(&db, &metadata).await?;
```

## Advantages Over Async Refactor

| Aspect | Async Refactor | IndexedDB Metadata |
|--------|---------------|-------------------|
| Code changes | ~50+ signatures | Single new module |
| Complexity | Cascading async | Isolated to storage |
| Storage size | 50MB (full messages) | 132KB (metadata only) |
| Trait changes | All methods async | Trait stays sync |
| Test changes | All tests async | No changes needed |
| Runtime deps | Tokio everywhere | None |

## Security Guarantees

### Tamper Detection

Bulletin board **cannot**:
- ❌ Delete messages (hash mismatch detected)
- ❌ Reorder messages (hash mismatch at position)
- ❌ Inject old messages (hash list is append-only)
- ❌ Modify messages (hash changes)

Trustee **will detect**:
- ✅ Any modification to historical messages
- ✅ Any deletion from history
- ✅ Any reordering of messages
- ✅ Duplicate message attempts

### Persistence

Browser trustee can:
- ✅ Survive page reloads
- ✅ Resume from last state
- ✅ Verify entire history on restart
- ✅ Detect tampering across sessions

## Comparison: Native vs Browser Storage

### Native (SqliteStorage)

```
Store: Full messages in SQLite
Size: 50MB for 1000 messages
Startup: Load metadata, reconstruct LocalBoard from disk
Fetch: Only new messages (after last_external_id)
Security: AUTOINCREMENT ID controls order
```

### Browser (IndexedDbStorage)

```
Store: Hashes + metadata in IndexedDB  
Size: 132KB for 1000 messages
Startup: Load metadata, fetch ALL messages from BB, verify hashes
Fetch: All messages on restart, incremental within session
Security: Hash list position controls order
```

Both achieve the same security properties through different mechanisms.

## Future Optimizations

1. **Incremental verification**: Only verify new messages if hash_list prefix matches
2. **Compression**: gzip metadata before storing in IndexedDB
3. **Bloom filter**: Fast duplicate detection before checking HashSet
4. **Pagination**: Handle very large message sets (>10,000 messages)

## Files

- `crates/braid/src/wasm/board/storage_indexeddb.rs` - Implementation
- `crates/braid/src/wasm/board/mod.rs` - Module registration
- `crates/braid/Cargo.toml` - Dependencies (bincode, web-sys features)
