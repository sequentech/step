# B3 Migration Analysis: gRPC Removal & S3 Integration

## Executive Summary

**Recommendation: Hybrid Approach** - Use Service POC's HTTP/S3 pattern as the foundation, then port B3's business logic incrementally with refactoring.

This analysis compares B3's current architecture against the Service POC to identify what to preserve, what to discard, and what to modernize.

---

## Current B3 Architecture

### Components

#### 1. **Transport Layers** (gRPC-specific - TO REMOVE)
- **`src/grpc/mod.rs`** - Protocol buffer definitions, gRPC message types
- **`src/grpc/server.rs`** - gRPC server implementation with PostgreSQL backend
- **`src/client/grpc.rs`** - gRPC client with chunking, timeout, connection management

**Dependencies to Remove:**
- `tonic` - gRPC framework
- `prost` - Protocol buffer serialization
- Message chunking logic (gRPC 2MB limit workaround)

#### 2. **Database Layer** (PRESERVE with refactoring)
- **`src/client/pgsql.rs`** - PostgreSQL client for bulletin board storage
  - Schema management (INDEX table, message tables per board)
  - Message insertion, retrieval, pagination
  - Board listing and management
  - Transaction handling

**What to Keep:**
- Schema design for message metadata
- Validation logic
- Query patterns for message retrieval by ID range

**What to Modernize:**
- Replace gRPC message types with HttpB3Message
- Simplify for REST API patterns (no need for multi-board batching)
- Consider SQLite option (like Service POC) for simpler deployments

#### 3. **Message Types** (ALREADY MIGRATED ✓)
- **`src/messages/http_message.rs`** - HttpB3Message (universal type)
- **`src/messages/message.rs`** - Core Message type
- **`src/messages/statement.rs`** - Statement types
- **`src/messages/artifact.rs`** - Artifact handling
- **`src/messages/newtypes.rs`** - Type definitions

**Status:** Already feature-gated and compatible with both native/WASM

#### 4. **Core Business Logic** (PRESERVE - Critical!)

Located in Braid crate, but B3 provides infrastructure:
- Message validation and schema versioning
- Statement/artifact separation
- Timestamp management
- Board name validation

---

## Service POC Architecture (Target Pattern)

### What Service POC Does Right

#### 1. **Two-Phase Message Upload** ✓
```rust
// Phase 1: Initiate - decide inline vs S3
POST /messages/initiate { size }
→ { message_id, upload_url?, should_upload }

// Phase 2a: Large message - client uploads to S3
PUT <upload_url> (S3 pre-signed)

// Phase 2b: Confirm with metadata
POST /messages/confirm/:id { data? }
```

**Benefits:**
- Client knows upfront if S3 upload needed
- Server generates pre-signed URLs (security)
- No message size limits on server
- Works with both LocalStack and AWS S3

#### 2. **Simple SQLite Schema**
```sql
CREATE TABLE messages (
    id TEXT PRIMARY KEY,
    timestamp INTEGER NOT NULL,
    size INTEGER NOT NULL,
    content_type TEXT NOT NULL,
    inline_data BLOB,
    s3_key TEXT
)
```

**Benefits:**
- Single file database (easy dev/test)
- Simple schema (no complex joins)
- Content type determines inline vs S3

#### 3. **Clean Separation**
- `handlers.rs` - HTTP endpoints only
- `db.rs` - Database operations only
- `s3.rs` - S3 pre-signed URL generation only
- `state.rs` - Shared app state

---

## Gap Analysis: B3 vs Service POC

### What B3 Has (Service POC Lacks)

1. **Multi-Board Support**
   - B3 manages multiple boards in one database
   - Service POC is single-board
   - **Action:** Add board name to schema and endpoints

2. **Message Retrieval by Range**
   ```rust
   get_messages(board: &str, last_id: i64) -> Vec<Message>
   ```
   - Essential for trustee synchronization
   - Service POC only has get by ID and list all
   - **Action:** Add `GET /boards/:board/messages?last_id=N` endpoint

3. **Batch Operations**
   ```rust
   put_messages_multi(requests: Vec<(String, Vec<Message>)>)
   get_messages_multi(requests: &Vec<(String, i64)>)
   ```
   - gRPC optimization for multiple boards at once
   - **Decision:** Probably not needed for HTTP (can make parallel requests)

4. **Blob Store for Artifacts**
   - B3 server can store large artifacts in filesystem
   - Separate from database for efficiency
   - Service POC uses S3 instead
   - **Decision:** S3 is better (scalable, durable, works with WASM)

5. **Schema Versioning**
   ```rust
   pub fn get_schema_version() -> String { "1".to_string() }
   ```
   - Every message includes version field
   - **Action:** Keep this pattern

6. **Board Index Table**
   - Tracks active vs archived boards
   - Board creation timestamps
   - **Action:** Add to Service schema

### What Service POC Has (B3 Lacks)

1. **S3 Integration** ✓
   - Pre-signed URLs for upload/download
   - Client-side S3 operations (reduces server load)
   - **Action:** This is what we're adding to B3!

2. **Size Threshold Logic**
   ```rust
   if size > MAX_INLINE_MESSAGE_SIZE {
       // Use S3
   } else {
       // Store inline
   }
   ```
   - **Action:** Port to B3

3. **Axum HTTP Server**
   - Modern async Rust HTTP framework
   - Better than gRPC for browser compatibility
   - **Action:** Replace B3's Tonic server with Axum

---

## Migration Strategy: Hybrid Approach

### Phase 1: Setup Foundation (Week 1)

**Goal:** Service POC with multi-board support

1. **Extend Service POC schema:**
   ```sql
   CREATE TABLE boards (
       name TEXT PRIMARY KEY,
       created_at INTEGER NOT NULL,
       status TEXT NOT NULL  -- 'active' or 'archived'
   );
   
   CREATE TABLE messages (
       id INTEGER PRIMARY KEY AUTOINCREMENT,
       board_name TEXT NOT NULL,
       timestamp INTEGER NOT NULL,
       size INTEGER NOT NULL,
       content_type TEXT NOT NULL,
       inline_data BLOB,
       s3_key TEXT,
       version TEXT NOT NULL,
       FOREIGN KEY (board_name) REFERENCES boards(name)
   );
   
   CREATE INDEX idx_messages_board_id ON messages(board_name, id);
   ```

2. **Add board management endpoints:**
   ```
   GET  /boards                    # List all boards
   POST /boards                    # Create board
   GET  /boards/:board/messages    # List messages for board
   ```

3. **Add range-based retrieval:**
   ```
   GET /boards/:board/messages?last_id=123
   ```

### Phase 2: Port B3 Logic (Week 2)

**Goal:** Replace gRPC types, keep business logic

1. **Update B3's PostgreSQL client:**
   - Replace `GrpcB3Message` with `HttpB3Message` ✓ (already done)
   - Add S3 key field to schema
   - Add content_type field (inline vs S3)

2. **Port validation logic from B3:**
   - Board name validation
   - Message validation
   - Statement/artifact validation
   - Schema version checks

3. **Keep B3's query patterns:**
   - Efficient range-based retrieval
   - Pagination logic
   - Transaction handling

### Phase 3: S3 Integration (Week 2-3)

**Goal:** Add S3 support to B3 database layer

1. **Add S3 client to `pgsql.rs`:**
   ```rust
   pub struct PgsqlB3Client {
       pool: Pool<...>,
       s3_client: Option<S3Client>,  // NEW
       bucket_name: Option<String>,   // NEW
   }
   ```

2. **Implement two-phase insert:**
   ```rust
   async fn initiate_message(board: &str, size: usize) 
       -> (i64, Option<String>)  // (id, upload_url)
   
   async fn confirm_message(board: &str, id: i64, data: Option<Vec<u8>>)
       -> Result<()>
   ```

3. **Update retrieval to handle S3:**
   ```rust
   async fn get_messages(board: &str, last_id: i64) 
       -> Vec<HttpB3Message>  // inline data or download URLs
   ```

### Phase 4: HTTP Server (Week 3)

**Goal:** Replace gRPC server with Axum

1. **Port Service's handlers to B3:**
   - Copy `handlers.rs`, `s3.rs` patterns
   - Adapt for multi-board support
   - Use B3's PostgreSQL client instead of SQLite

2. **Remove gRPC server:**
   - Delete `src/grpc/server.rs`
   - Delete `src/grpc/mod.rs`
   - Remove tonic/prost dependencies

3. **Update configuration:**
   - HTTP port instead of gRPC port
   - S3 bucket configuration
   - PostgreSQL connection params

### Phase 5: Client Update (Week 4)

**Goal:** Update Braid to use new HTTP API

1. **Implement HTTP Board client:**
   ```rust
   pub struct HttpB3 {
       client: reqwest::Client,
       base_url: String,
       s3_client: S3Client,
   }
   ```

2. **Implement Board trait:**
   ```rust
   async fn get_messages(board: &str, last_id: i64) -> Vec<HttpB3Message>
   async fn insert_messages(board: &str, messages: Vec<Message>)
   ```

3. **Handle S3 transparently:**
   - Check message size before insert
   - Upload large messages to S3
   - Download large messages from S3 on retrieval

### Phase 6: Remove gRPC Client (Week 4)

1. **Delete `src/client/grpc.rs`**
2. **Remove Braid's `grpc_m.rs` board implementations**
3. **Update all tests to use HTTP**
4. **Remove tonic/prost from dependencies**

---

## Critical Code to Preserve from B3

### 1. Database Schema & Queries (pgsql.rs)

```rust
// Message row structure - Keep this!
pub struct B3MessageRow {
    pub id: i64,
    pub created: Timestamp,
    pub statement_kind: String,
    pub sender_pk: String,
    pub batch: i64,
    pub mix_number: i64,
    pub message: Vec<u8>,
}

// Query pattern - Keep this!
pub async fn get_messages(
    &self,
    board_name: &str,
    last_id: i64,
) -> Result<Vec<B3MessageRow>> {
    // Efficient pagination by ID
}

// Board index - Keep this!
pub async fn get_boards(&self) -> Result<Vec<B3IndexRow>>
```

### 2. Validation Logic

```rust
// Board name validation - Keep!
fn validate_board_name(name: &str) -> Result<()> {
    // Security: prevent path traversal, SQL injection
}

// Schema versioning - Keep!
pub fn get_schema_version() -> String {
    "1".to_string()
}
```

### 3. Message Metadata Extraction

B3 extracts metadata from Message for database indexing:
- Statement type
- Sender public key  
- Batch number
- Mix number

This allows efficient querying without deserializing all messages.

---

## Code to Discard from B3

### 1. Entire gRPC Layer
- `src/grpc/mod.rs` - Protocol buffers
- `src/grpc/server.rs` - gRPC server
- `src/client/grpc.rs` - gRPC client
- `GrpcB3Message` type
- Message chunking logic (2MB limit)

### 2. Multi-Board Batching
- `put_messages_multi` - Not needed with HTTP
- `get_messages_multi` - Can use parallel HTTP requests
- Chunker struct - gRPC artifact

### 3. Blob Store Filesystem Code
- B3 stores large artifacts in local filesystem
- Replace with S3 (better for cloud, WASM, scalability)

---

## Size Threshold Decisions

### Service POC
```rust
pub const MAX_INLINE_MESSAGE_SIZE: usize = 1024 * 1024; // 1MB
```

### B3 gRPC
```rust
pub const MESSAGE_CHUNK_SIZE: usize = 2 * 1000 * 1000; // 2MB
```

### Recommendation
**Use 1MB threshold:**
- S3 has no practical size limit
- Keeps database small
- 1MB is reasonable for inline data
- Avoids gRPC's 2MB chunking complexity

---

## API Design: New HTTP Endpoints

### Board Management
```
GET    /boards
POST   /boards
GET    /boards/:board
DELETE /boards/:board
```

### Messages (Two-Phase Upload)
```
POST   /boards/:board/messages/initiate
       Request:  { size: usize }
       Response: { id: i64, upload_url?: String, should_upload: bool }

POST   /boards/:board/messages/:id/confirm
       Request:  { data?: Vec<u8> }
       Response: { success: bool }

GET    /boards/:board/messages?last_id=N
       Response: { messages: Vec<HttpB3Message>, truncated: bool }

GET    /boards/:board/messages/:id
       Response: { message: HttpB3Message, download_url?: String }
```

**Note:** For large messages, `HttpB3Message.message` field would be empty, client uses `download_url` to fetch from S3.

---

## Migration Checklist

### Week 1: Foundation
- [ ] Extend Service POC with multi-board schema
- [ ] Add board management endpoints
- [ ] Add range-based message retrieval
- [ ] Port board name validation from B3
- [ ] Add schema versioning

### Week 2: B3 Database Layer
- [ ] Update B3 `pgsql.rs` to use HttpB3Message
- [ ] Add S3 key and content_type to schema
- [ ] Implement two-phase insert (initiate/confirm)
- [ ] Add S3 client to PgsqlB3Client
- [ ] Update get_messages to handle S3 references

### Week 3: HTTP Server
- [ ] Create Axum server in B3 (based on Service POC)
- [ ] Implement all new endpoints
- [ ] Add CORS support
- [ ] Remove gRPC server code
- [ ] Update configuration

### Week 4: Client & Cleanup
- [ ] Implement HttpB3 board client in Braid
- [ ] Update all Braid code to use HTTP
- [ ] Remove gRPC client from B3
- [ ] Remove tonic/prost dependencies
- [ ] Update all tests
- [ ] Update documentation

---

## Risk Assessment

### Low Risk
- ✅ Service POC proves HTTP/S3 pattern works
- ✅ HttpB3Message already implemented
- ✅ Feature gates isolate changes
- ✅ Can develop in parallel with existing code

### Medium Risk
- ⚠️ Need to ensure all B3 business logic ported
- ⚠️ Database migration for existing deployments
- ⚠️ S3 configuration complexity

### Mitigation Strategies
1. **Keep B3 gRPC code until HTTP fully tested**
   - Feature flag: `grpc-backend` vs `http-backend`
   - Run both in parallel during migration

2. **Comprehensive test coverage**
   - Port all B3 tests to HTTP
   - Add integration tests with LocalStack
   - Test large message flows

3. **Database migration script**
   - Provide SQL to migrate existing boards
   - Add S3 fields to existing schema
   - Preserve all message data

---

## Success Criteria

### Functional
- [ ] All B3 operations work via HTTP
- [ ] Large messages (>1MB) stored in S3
- [ ] Small messages stored inline
- [ ] Multi-board support maintained
- [ ] Range-based message retrieval works
- [ ] WASM client can use bulletin board

### Performance
- [ ] Message retrieval as fast as gRPC
- [ ] Large message upload doesn't block server
- [ ] Pre-signed URLs expire appropriately

### Code Quality
- [ ] Zero gRPC dependencies
- [ ] Clean separation (handlers/db/s3)
- [ ] 100% test coverage
- [ ] Documentation updated

---

## Conclusion

**Recommended Approach:** Start with Service POC's clean HTTP/S3 pattern, then incrementally add B3's battle-tested business logic.

**Key Insight:** Service POC already demonstrates the architecture we need. The work is primarily:
1. Adding multi-board support to Service
2. Porting B3's database schema and validation logic
3. Removing all gRPC code from B3

This is **primarily additive work** (adding to Service) rather than risky deletion (removing from B3), which makes it lower risk than trying to retrofit Service's patterns into B3's existing codebase.

**Estimated Timeline:** 4 weeks for complete migration with testing.
