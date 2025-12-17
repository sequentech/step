# DKG Cryptography Migration - Remaining Tasks

## Critical Tasks

### 1. Decryption Factors Signature Alignment
- **Issue**: Participant position in decryption factors artifact must match signatures
- **Solution**: Removed position from wire format; position now derived from message signature
- **Changes**:
  - Created `DecryptionFactor<C, W>` (singular) without position - just crypto data
  - Created `DecryptionFactors<C, W, P>` (plural) with position - for combine operation
  - Created `PartialDecryption<C, W>` message artifact without position
  - Protocol layer reconstructs `DecryptionFactors` by adding `source` from message signer
  - Type parameters standardized to `C, W, P` order for consistency
- **Files Modified**:
  - `crates/cryptography/src/dkgd/recipient.rs`
  - `crates/b5/src/messages/artifact.rs`
  - `crates/b5/src/messages/message.rs`
  - `crates/braid_b5/src/protocol/action/decrypt.rs`
  - `crates/braid_b5/src/protocol/board/local_board.rs`
  - `crates/braid_b5/src/protocol/trustee.rs`
- **Priority**: High - correctness issue
- **Status**: ✅ COMPLETE - Security vulnerability eliminated

### 2. Artifact Type Parameter Standardization
- **Issue**: Inconsistent use of type parameters across artifacts
  - Some use `<C, W>`
  - Some use `<C, W, P>`
  - Some use `<C, W, T>`
- **Type Parameter Definitions**:
  - `C`: Cryptographic context (e.g., RistrettoCtx)
  - `W`: Ciphertext width (number of group element pairs)
  - `P`: Number of participants in DKG that produces the public key
  - `T`: Threshold number of participants needed for decryption
- **Proposed Principle: Structural vs. Operational Parameters**
  - **Include type parameter ⟺ it determines compile-time memory layout**
  - **Rule**: If removing the type parameter would require changing the struct definition, include it. If the struct could remain identical, exclude it.
  - **Examples**:
    - `W` in `DkgCiphertext<C, W>`: ✅ Include (determines `[C::Element; W]` array size)
    - `P` in `DkgCommitments<C, P>`: ✅ Include if struct has `[...; P]` arrays
    - `T` in `DkgPublicKey<C, T>`: ❌ Exclude (doesn't affect structure, only operation constraints)
    - `C`: ✅ Always include (fundamentally affects types)
- **Validation Strategy**:
  - Runtime validation via `election_id: u128` in artifacts (references `Configuration.id`)
  - Configuration remains single source of truth for P/T/W values
  - Protocol layer validates `artifact.election_id == configuration.id`
  - Prevents cross-election artifact mixing in multi-election deployments
- **Serialization Approach**:
  - Context parameter `C`: Use `PhantomData<C>` (zero-byte serialization, following serde pattern)
  - Const generics (W/P/T): Natural validation through array sizes where structural
  - Note: `Marker<C>` infrastructure exists in `crates/vsc/src/utils/serialization/variable.rs` for runtime type validation if needed, with tests demonstrating usage. Currently unused but retained for potential future use.
- **Action Required**: Audit each artifact struct to apply Structural vs. Operational principle
- **Candidates for Review**:
  - `DkgPublicKey<C, T>` → likely should be `DkgPublicKey<C>`
  - `DecryptionFactors<C, W, P>` → check if P appears in structure
  - All artifacts in `crates/b5/src/messages/artifact.rs`
  - All artifacts in `crates/cryptography/src/dkgd/`
- **Priority**: Medium - maintainability and API clarity
- **Status**: 🔄 IN PROGRESS - Principle defined, awaiting systematic audit

### 3. Context Genericity Cleanup
- **Issue**: Serialization-as-conversion code smell in signature handling
- **Solution Implemented**:
  - Made `Message<C>`, `Sender<C>`, `VerifiedMessage<C>` fully generic over Context
  - Removed serialize/deserialize conversion in `Signer::sign()` and `Message::verify()`
  - Added serialization methods to `SignatureScheme` trait:
    - `verifier_to_base64_string(verifier: &Self::Verifier) -> Result<String, String>`
    - `verifier_from_base64_string(s: &str) -> Result<Self::Verifier, String>`
  - Updated all board traits to be generic: `Board<C>`, `BoardMulti<C>`, `BoardFactory<C, B>`, `BoardFactoryMulti<C, B>`
  - `HttpB5` board now fully generic over any Context while supporting HTTP serialization
  - Updated storage backends with generic `retrieve_messages<C: Context>()`
  - Cleaned up `HttpB5Message` (removed unused metadata fields: sender_pk, statement_kind, batch, mix_number)
  - Consolidated API types in `b5/src/api_types.rs`
- **Architectural Decision**:
  - **b5 crate**: Message types are now **generic over Context**
    - `Message<C: Context>` with generic signatures
    - Direct generic signature creation and verification (no conversion)
    - Artifacts remain generic `<C: Context>`
  - **braid_b5 crate**: Fully generic over `C: Context`
    - Protocol layer works with any Context
    - Board implementations generic (including HttpB5)
    - Can be tested with different contexts
  - **cryptography crate**: Hashing is **intentionally fixed infrastructure**
    - `CryptographicHasher` type alias = SHA3-512 globally
    - `Context` trait is a configuration menu for most components (Element, Scalar, SignatureScheme, Rng) but only aggregates hashing (not configurable)
    - Each `CryptographicGroup` specifies `type Hasher = crate::context::CryptographicHasher` (would be default when Rust supports it)
    - Eliminates "two contexts" confusion from previous design
    - Hashing is architectural infrastructure, not a choice parameter
- **Benefits Achieved**:
  - Eliminated code smell: no more wasteful serialize/deserialize conversion
  - Clean generic architecture throughout
  - HttpB5 supports wire protocol via trait methods (elegant solution)
  - Full type safety at compile time
  - More flexible and testable code
  - Honest design: clear separation between configurable components and fixed infrastructure
  - Single decision point for hash function (`CryptographicHasher`)
- **Files Modified**:
  - `crates/cryptography/src/context.rs` - Added CryptographicHasher type alias, documented Context trait
  - `crates/cryptography/src/groups/ristretto255/group.rs` - Uses CryptographicHasher with explanatory comment
  - `crates/cryptography/src/groups/p256/group.rs` - Uses CryptographicHasher with explanatory comment
  - `crates/cryptography/src/utils/signatures.rs` - Added serialization methods to SignatureScheme trait
  - `crates/b5/src/lib.rs` - Re-exports Hasher and defines CryptographicHash output type
  - `crates/b5/src/messages/message.rs` - Made Message, Sender generic
  - `crates/b5/src/messages/newtypes.rs` - Uses concrete Hash type from b5 crate
  - `crates/b5/src/messages/http_message.rs` - Cleaned up HttpB5Message structure
  - `crates/b5/src/api_types.rs` - Consolidated API types
  - `crates/b5/src/handlers.rs` - Updated to use consolidated API types
  - `crates/braid_b5/src/protocol/board.rs` - All board traits generic, documented cfg gating
  - `crates/braid_b5/src/protocol/trustee.rs` - StepResult generic
  - `crates/braid_b5/src/native/board/http.rs` - HttpB5 fully generic (struct still named HttpB3)
  - `crates/braid_b5/src/native/storage/` - All storage backends updated
  - `crates/braid_b5/src/wasm/board/http.rs` - WASM HttpB5 board updated
  - `crates/braid_b5/src/wasm/board/storage_indexeddb.rs` - WASM storage updated
  - `crates/braid_b5/src/wasm/session.rs` - WASM session updated
  - `crates/braid_b5/src/wasm/verify.rs` - WASM verifier updated
  - Various test files - Type annotations updated
- **Priority**: Medium - developer experience
- **Status**: ✅ COMPLETE - Full genericity achieved with proper separation between configurable and fixed components

### 4. W Parameter Generalization Review
- **Issue**: Some code assumes `W = 2`, needed generalization for flexible ciphertext width
- **Solution Overview**:
  - Created `dispatch_ciphertext_width!` macro to bridge runtime config → compile-time const generic
  - Made all production code generic over `W` (ciphertext width parameter)
  - Supports W ∈ {1, 2, 3, 4} via dispatch macro
  - Updated test infrastructure to support configurable W
- **Completed Actions**:
  - ✅ Core W parameter implementation across all protocol code
  - ✅ Configuration struct with ciphertext_width field (default: 2)
  - ✅ dispatch_ciphertext_width! macro in lib.rs
  - ✅ LocalBoard accessors generic over W
  - ✅ All action implementations (DKG, mix, decrypt) dispatch correctly
  - ✅ Test helper functions made generic (get_plaintexts_nohash, etc.)
  - ✅ Created demo-multi-b5.ps1 with -CiphertextWidth parameter
  - ✅ Created demo-browser-b5.ps1 with W parameter support
  - ✅ Created B5_TESTING_SCRIPTS.md documentation
  - ✅ Tested demo-multi-b5.ps1 with native multi-board protocol (W=2 verified)
  - ✅ All production code generic, no hardcoded W assumptions
  - ✅ Fixed-type boundary pattern working correctly
- **Remaining Actions**:
  - ⏭️ Create serve-b5.ps1 script to build and serve WASM braid_b5 trustee (reuse trustee.html)
  - ⏭️ Test demo-browser-b5.ps1 with serve-b5.ps1 for full browser+native protocol with W parameter
  - ⏭️ Test webassembly verifier for braid_b5 (reuse verifier.html frontend)
  - ⏭️ Fix test data generation code that currently hardcodes W=2 in:
    - `crates/braid_b5/src/native/test/protocol_test_http.rs`
    - `crates/braid_b5/src/native/test/protocol_test_memory.rs`
    - `crates/braid_b5/src/native/test/dbg.rs`
  - ⏭️ Test protocol execution with W=1, 3, 4 to validate full genericity
- **Files Modified**:
  - `crates/braid_b5/src/lib.rs` - dispatch_ciphertext_width! macro
  - `crates/braid_b5/src/protocol/board/local_board.rs` - Generic accessors
  - `crates/braid_b5/src/protocol/action/*.rs` - All actions dispatch on W
  - `crates/braid_b5/src/protocol/trustee.rs` - Generic test helpers
  - Various test files - Updated with turbofish syntax for W=2
  - `demo-multi-b5.ps1` - Multi-board testing with W parameter
  - `demo-browser-b5.ps1` - Browser testing with W parameter
  - `B5_TESTING_SCRIPTS.md` - Complete testing documentation
- **Priority**: Medium - flexibility for future use cases, testing infrastructure
- **Status**: 🔄 IN PROGRESS - Core implementation complete, WASM/browser testing remains

### 5. WASM Support Analysis
- **Reference**: Previous `strand`, `b4`, `braid` implementation
- **Action**:
  - ✅ Test compilation to `wasm32-unknown-unknown` - **COMPLETE**
  - ✅ Fix dependency issues (removed ring/aws-lc-sys from WASM build)
  - ✅ Configure getrandom with wasm_js/js features
  - ✅ Update WASM code to use b5 instead of b4
  - ✅ Fix b5 helper function usage in WASM code
  - ⏭️ Test browser integration
  - ⏭️ Validate trustee.html still works
- **Files to check**:
  - `crates/strand/` - WASM reference implementation
  - `trustee.html` - browser integration
  - Feature flags for std/no_std
- **Priority**: High - required functionality
- **Status**: ✅ WASM compilation successful, browser testing pending

### 6. Parallelization Audit
- **Issue**: Migration may have introduced sequential operations where parallel was used before
- **Action**:
  - Review collection processing (`.iter()` vs `.par_iter()`)
  - Check for extra wrapping/unwrapping in loops
  - Compare with original braid implementation
- **Key areas**:
  - Batch processing
  - Trustee operations
  - Artifact verification
- **Priority**: Medium - performance
- **Status**: TODO

## Additional Considerations

### 7. Error Handling Consistency
- **Action**: Review error types across migration
  - String errors vs anyhow vs custom types
  - Consistent `.map_err()` usage
- **Priority**: Low - code quality
- **Status**: TODO

### 8. Integration Testing
- **Action**: Run full protocol end-to-end tests
  - Multi-board scenarios
  - DKG round-trip
  - Encryption/decryption pipeline
  - Browser trustee compatibility
- **Priority**: High - validation
- **Status**: In Progress (compilation errors resolved)

### 9. Serialization Format Compatibility
- **Issue**: Ensure wire format hasn't changed
- **Action**:
  - Compare serialized output with pre-migration version
  - Verify backward compatibility if needed
- **Priority**: ~~High~~ - deployment compatibility
- **Status**: ❌ DROPPED - No backwards compatibility requirement. Versioning/schema concerns covered by Task 13.

### 10. Performance Benchmarks
- **Action**: Compare performance with previous implementation
  - DKG setup time
  - Encryption throughput
  - Decryption performance
- **Tools**: Use `criterion` benchmarks in `crates/cryptography/benches/`
- **Priority**: Medium - regression detection
- **Status**: TODO

### 11. Documentation Updates
- **Action**: Update high-level documentation
  - README files
  - API documentation
  - Migration guide for users
- **Priority**: Low - user experience
- **Status**: TODO

### 12. Cleanup
- **Action**: Remove migration artifacts
  - Unused imports warnings
  - Dead code warnings
  - Commented-out code
  - Temporary helper functions
- **Priority**: Low - code hygiene

### 13. Version Field Strategy Review
- **Issue**: Version field propagation and validation for schema compatibility
- **Implementation Status**: ✅ Version propagation complete, validation pending
- **Completed**:
  - ✅ **HttpB5Message Symmetric Usage**: Board trait now uses `HttpB5Message` for both `get_messages()` and `post_messages()`, establishing clean architectural boundary between protocol layer (Message<C>) and wire format (HttpB5Message)
  - ✅ **Version Field Propagation**: Version now flows bidirectionally through entire system:
    - **Posting**: Client sends actual version from `HttpB5Message::from_protocol_message()` which calls `get_schema_version()` (currently returns "1")
    - **Retrieval**: Server returns version from database in all API responses (`api_types::Message`, `MessageWithUrl`)
    - **Construction**: Client uses returned version when creating `HttpB5Message` (no hardcoded "1")
  - ✅ **API Updates**:
    - Added `version: String` to `api_types::Message`
    - Added `version: String` to `ConfirmMessageRequest` (single-board API)
    - Added `version: String` to `MessageConfirmation` (multi-board API)
  - ✅ **Database**: Version field already existed, now properly populated and returned
  - ✅ **All Implementations Updated**:
    - Native single-board and multi-board posting/retrieval
    - WASM board posting/retrieval
    - WASM session message fetching
- **Infrastructure Complete**: ✅ Version storage and retrieval in place
  - ✅ SQLite schema updated with `version TEXT NOT NULL` column
  - ✅ `store_messages()` saves version to database
  - ✅ `retrieve_messages()` returns version from database (in SqliteStoreMessageRow)
  - ✅ Version flows through all APIs (client ↔ server ↔ database)
- **Remaining: Version Validation Logic**
  - **Purpose**: Prevent schema incompatibility between clients, servers, and storage
  - **Note**: All infrastructure is now in place; validation is deliberately deferred for separate design discussion
  - **Tentative Validation Points**:
    - **(a) Server receives message from client**: When b5 handler processes `ConfirmMessageRequest` or `MessageConfirmation`, validate `request.version == get_schema_version()` before deserializing message bytes
      - Location: `crates/b5/src/handlers.rs` - `confirm_message()`, `confirm_messages_multi()`
      - Both S3 and inline message paths
      - **Rationale**: Reject incompatible messages at ingestion boundary
    - **(b) Client receives message from server**: When client constructs `HttpB5Message` from API response, validate `message.version == get_schema_version()` before processing
      - Location: `crates/braid_b5/src/native/board/http.rs` - `get_messages()`, `get_messages_multi()`
      - Location: `crates/braid_b5/src/wasm/board/http.rs` - `fetch_messages_internal()`
      - Location: `crates/braid_b5/src/wasm/session.rs` - `fetch_messages()`
      - Both S3 and inline message paths
      - **Rationale**: Client shouldn't process messages it can't understand
      - **Note**: Current validation in `store_messages()` is misplaced - it's case (b) logic in storage layer
    - **(c) Server reads from database**: When b5 retrieves messages from SQLite, validate version matches current schema
      - Location: `crates/b5/src/db.rs` - `get_messages_after()`, `get_message_by_id()`, `list_messages()`
      - **Rationale**: Handle schema upgrades - old database, new server code
      - Could reject mismatched versions or implement migration logic
    - **(d) Native client reads from storage**: When native client retrieves messages from SQLite after app upgrade, validate version
      - Location: `crates/braid_b5/src/native/board/storage_sqlite.rs` - `retrieve_messages()`
      - **Rationale**: Handle schema upgrades - old database, new client code
      - **Infrastructure**: ✅ Version now stored in database and returned in SqliteStoreMessageRow
      - Could reject mismatched versions or implement migration logic
    - **Note on WASM**: WASM clients use IndexedDB to store only message hashes (not serialized content), so version mismatches naturally cause hash verification failures - no explicit version check needed
- **Design Decisions Pending**:
  - Fail fast vs graceful degradation on version mismatch?
  - Migration path when schema changes (auto-upgrade, manual conversion, reject old messages)?
  - Should validation log warnings before errors?
  - Consider semantic versioning (major.minor.patch) vs simple incrementing?
- **Files Modified**:
  - `crates/b5/src/api_types.rs` - Added version fields to Message, ConfirmMessageRequest, MessageConfirmation
  - `crates/b5/src/db.rs` - Return version from queries
  - `crates/b5/src/handlers.rs` - Use version from requests instead of hardcoding
  - `crates/braid_b5/src/native/board/http.rs` - Send/receive version in all operations
  - `crates/braid_b5/src/wasm/board/http.rs` - Send/receive version in WASM board
  - `crates/braid_b5/src/wasm/session.rs` - Use version from API responses
- **Priority**: Medium - architectural clarity and maintainability
- **Status**: 🔄 IN PROGRESS - Propagation complete ✅, validation design and implementation pending

### 14. B3 Nomenclature Audit and Cleanup
- **Issue**: Despite HttpB3Message → HttpB5Message rename, many B3 references remain
- **Known Instances**:
  - `HttpB3` struct name in `crates/braid_b5/src/native/board/http.rs` (should be HttpB5)
  - `HttpB3BoardParams` struct and type name
  - `HttpB3Index` type name
  - Comment in `crates/b5/src/messages/http_message.rs` references "GrpcB3Message" (should be GrpcB5Message)
  - Multiple references in MIGRATION_TODO.md itself
  - Various imports and type annotations throughout braid_b5
  - Legacy references in `summary.md` (decide: historical documentation or update?)
- **Scope**:
  - Update struct names: `HttpB3` → `HttpB5`, `HttpB3BoardParams` → `HttpB5BoardParams`, etc.
  - Update comments: `GrpcB3Message` → `GrpcB5Message`
  - Update all imports and type annotations
  - Decide on handling historical documentation references
- **Files Affected** (at least 20+ files):
  - `crates/braid_b5/src/native/board/http.rs`
  - `crates/braid_b5/src/native/board/mod.rs`
  - `crates/braid_b5/src/native/session/session_master.rs`
  - `crates/braid_b5/src/native/test/protocol_test_http.rs`
  - `crates/braid_b5/src/bin/main.rs`
  - `crates/braid_b5/src/bin/verify.rs`
  - `crates/b5/src/messages/http_message.rs`
  - `MIGRATION_TODO.md`
  - `summary.md` (if updating historical docs)
  - Various other files with imports/type annotations
- **Priority**: Low-Medium - consistency and clarity (no functional impact)
- **Status**: TODO

### 15. Cleanup
## Notes

- **Migration Status**: Core compilation complete (0 errors), WASM compilation successful ✅
- **Next Steps**: Focus on items 1, 8, 9 (critical path: browser testing, integration tests, compatibility)
- **Testing Strategy**: Browser integration testing, then full protocol integration tests
- **WASM Achievement**: Successfully resolved dependency conflicts, b4→b5 migration in WASM code
- **Next Steps**: Focus on items 1, 5, 8, 9 (critical path)
- **Testing Strategy**: Start with WASM compilation, then integration tests
