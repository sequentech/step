# Claude Code Project Guidelines

## Licensing

This project is licensed under the **GNU Affero General Public License v3.0 (AGPL-3.0)**. See the [LICENSE](LICENSE) file for full details.

- All new source files must include the AGPL-3.0 license header
- Contributions must be compatible with the AGPL-3.0 license

## General Rules

- Use `nix develop --command` prefix for all shell commands (e.g., `nix develop --command cargo build`, `nix develop --command cargo test`)

## Rust Coding Style

### Format Strings
- Use inline variable syntax in format strings: `format!("Bearer {token}")` instead of `format!("Bearer {}", token)`
- This applies to all format macros: `format!`, `println!`, `tracing::info!`, etc.

### Example
```rust
// Preferred
let msg = format!("User {user_id} created board '{board_name}'");

// Avoid
let msg = format!("User {} created board '{}'", user_id, board_name);
```

### Tracing Instrumentation
- All functions should have tracing instrumentation using `#[instrument]`
- Skip large arguments (claims, file contents, byte arrays, etc.) using `skip` parameter
- For functions returning `Result`, add `err` to automatically log errors

### Example
```rust
// Function with error logging
#[instrument(err)]
pub async fn fetch_data() -> Result<Data> { ... }

// Function skipping large arguments
#[instrument(skip(file_content, claims))]
pub fn process_file(name: &str, file_content: &[u8], claims: &JwtClaims) -> Result<()> { ... }

// Method skipping self
#[instrument(skip(self))]
fn read_from_cache(&self) -> Option<Vec<Key>> { ... }

// Recording dynamic fields
#[instrument(skip(token), fields(kid))]
pub fn verify_token(token: &str) -> Result<()> {
    let kid = extract_kid(token)?;
    tracing::Span::current().record("kid", &kid);
    // ...
}
```

## Testing Guidelines

### Use Defined Constants
- Always use defined constants from `sequent-core` in tests instead of hardcoded strings
- For permissions: use `Permissions::TRUSTEE_CEREMONY.to_string()` instead of `"trustee-ceremony"`
- For roles: use `SERVER_DEFAULT_ROLE` from `b4::auth` instead of `"server"`
- For test data: use `TEST_TENANT_ID`, `TEST_ELECTION_EVENT_ID`, `TEST_SLUG` from `sequent_core::services::test_utils`

### Example
```rust
use sequent_core::types::permissions::Permissions;
use sequent_core::services::test_utils::{TEST_TENANT_ID, TEST_ELECTION_EVENT_ID};
use b4::auth::SERVER_DEFAULT_ROLE;

// Preferred - use constants
let permission = Permissions::TRUSTEE_CEREMONY.to_string();
let token = builder.with_permissions(&[&permission])
    .with_default_role(SERVER_DEFAULT_ROLE)
    .build(&keypair);

// Avoid - hardcoded strings
let token = builder.with_permissions(&["trustee-ceremony"])
    .with_default_role("server")
    .build(&keypair);
```

### Test Token Patterns
- **Server trustees**: Use `create_trustee_token()` or `create_native_trustee_token()` - these set `default_role = "server"` to bypass board validation
- **Browser trustees**: Use `create_browser_trustee_token(tenant_id, event_ids)` - these require matching tenant/event for board access
- The `RequireConstraints` extractor enforces board access validation for non-server trustees
