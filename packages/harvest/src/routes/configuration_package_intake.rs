// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Direct, first-party intake of an encrypted configuration package that
//! carries messaging credentials (`.ezip` V2). Never a Hasura action: the
//! transfer password and anything decrypted with it must not become logged
//! GraphQL variables.
//!
//! The browser has already put the encrypted bytes in this tenant's
//! protected staging (`get_upload_url` with `configuration_package`). It
//! then sends only the staging document ID and the transfer password here,
//! through the Admin Portal's own origin. Only the verified subject that
//! staged the upload may unwrap it, once. Harvest decrypts server side,
//! stores the credentials sidecar in the tenant vault (bound to the unwrapped
//! package and an expiry), writes the signed ZIP to a server-only sibling the
//! presigned upload URL can't reach, and publishes the sibling's pointer in
//! the same transaction. It answers with that ZIP's SHA-256 only. Preview and
//! Apply, with their signature, WebAuthn roster and tenant checks, are
//! unchanged; they read the published sibling.
//!
//! Authentication is the verified bearer guard; authorization is the same
//! as protected staging plus package messaging provisioning. The transport
//! must be HTTPS through the first-party proxy, or plain HTTP to loopback.
//! Browsers can't call this cross-origin: there is no CORS answer, and a
//! present `Origin` or `Sec-Fetch-Site` must say same origin.

use crate::services::audit;
use crate::services::authorization::authorize;
use deadpool_postgres::Client as DbClient;
use rocket::data::{Data, ToByteUnit};
use rocket::http::Status;
use rocket::request::{FromRequest, Outcome, Request};
use rocket::serde::json::Json;
use sequent_core::election_config::messaging_credentials::Zeroizing;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use std::fmt;
use tracing::instrument;
use windmill::services::database::get_hasura_pool;
use windmill::services::documents;
use windmill::services::import::configuration_package_credentials as package_credentials;

const GENERIC: &str = "Could not unwrap the configuration package.";

/// The protected-staging permissions, plus provisioning package messaging
/// accounts, exactly as Preview and Apply of such a package require.
pub fn intake_permissions() -> Vec<Permissions> {
    let mut permissions =
        super::apply_configuration_package::PACKAGE_PERMISSIONS.to_vec();
    permissions.extend([
        Permissions::MESSAGING_ACCOUNT_READ,
        Permissions::MESSAGING_ACCOUNT_WRITE,
        Permissions::MESSAGING_CONFIG_WRITE,
    ]);
    permissions
}

fn loopback(host: &str) -> bool {
    let name = match host.strip_prefix('[') {
        Some(rest) => rest.split(']').next().unwrap_or_default(),
        None => host.split(':').next().unwrap_or_default(),
    };
    matches!(name, "localhost" | "127.0.0.1" | "::1")
}

/// The request's transport and origin, as the first-party proxy and the
/// browser state them. Pure, so every case is tested.
pub fn first_party(
    host: Option<&str>,
    forwarded_proto: Option<&str>,
    origin: Option<&str>,
    fetch_site: Option<&str>,
) -> bool {
    let Some(host) = host.filter(|h| !h.is_empty()) else {
        return false;
    };
    let scheme = match forwarded_proto {
        Some("https") => "https",
        Some("http") | None if loopback(host) => "http",
        _ => return false,
    };
    let origin_ok = origin.is_none_or(|o| o == format!("{scheme}://{host}"));
    let site_ok = fetch_site.is_none_or(|s| s == "same-origin");
    origin_ok && site_ok
}

/// Request guard: refuses anything but the first-party, secure transport.
pub struct FirstParty;

#[rocket::async_trait]
impl<'r> FromRequest<'r> for FirstParty {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, ()> {
        let headers = request.headers();
        if first_party(
            headers.get_one("Host"),
            headers.get_one("X-Forwarded-Proto"),
            headers.get_one("Origin"),
            headers.get_one("Sec-Fetch-Site"),
        ) {
            Outcome::Success(FirstParty)
        } else {
            Outcome::Error((Status::Forbidden, ()))
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnwrapInput {
    document_id: String,
    password: String,
    /// The operator's optional SHA-256 of the inner signed ZIP. Checked
    /// after decryption, before anything is staged or replaced.
    #[serde(default)]
    expected_sha256: Option<String>,
}

impl fmt::Debug for UnwrapInput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UnwrapInput")
            .field("document_id", &self.document_id)
            .field("expected_sha256", &self.expected_sha256)
            .finish_non_exhaustive()
    }
}

#[derive(Serialize, Debug, PartialEq)]
pub struct UnwrapOutput {
    /// SHA-256 of the signed ZIP now staged: the `encrypted_payload_sha256`
    /// Preview and Apply check.
    sha256: String,
}

/// Only fixed public refusals reach the caller; anything else is generic.
fn refusal(error: &anyhow::Error) -> (Status, String) {
    let message = error.to_string();
    // Distinct status: the client tells a digest mismatch from a bad password.
    if message == package_credentials::HASH_MISMATCH {
        return (Status::Conflict, message);
    }
    if message == package_credentials::NOT_UPLOADER {
        return (Status::Forbidden, message);
    }
    // A concurrent unwrap published first.
    if message == "this staging document was already unwrapped" {
        return (
            Status::BadRequest,
            package_credentials::ALREADY_STAGED.into(),
        );
    }
    let public = [
        package_credentials::HASH_MALFORMED,
        package_credentials::NOT_V2,
        package_credentials::ALREADY_STAGED,
        package_credentials::NOT_STAGING,
        package_credentials::MISMATCH,
    ];
    if public.contains(&message.as_str()) {
        return (Status::BadRequest, message);
    }
    // The envelope's own fixed messages (wrong password, size, malformed).
    if message.starts_with("The package password ")
        || message.starts_with("The encrypted configuration package ")
        || message.starts_with("This is not a supported encrypted ")
        || message.starts_with("Enter a non-blank package password ")
    {
        return (Status::BadRequest, message);
    }
    (Status::InternalServerError, GENERIC.into())
}

fn internal<E>(_: E) -> (Status, String) {
    (Status::InternalServerError, GENERIC.into())
}

/// A document ID, a password of at most 1024 bytes and an optional digest.
const MAX_BODY_BYTES: usize = 8 * 1024;
const MALFORMED: &str = "The request is not a valid package unwrap request.";

/// Parse the request body without any error that could carry its bytes:
/// Rocket's `Json` guard logs a failed parse with the raw body (`{:?}` of
/// `Error::Parse(&raw, _)`), which here would include the password.
fn parse_input(bytes: &[u8]) -> Result<UnwrapInput, (Status, String)> {
    if bytes.is_empty() || bytes.len() > MAX_BODY_BYTES {
        return Err((Status::BadRequest, MALFORMED.into()));
    }
    serde_json::from_slice(bytes)
        .map_err(|_| (Status::BadRequest, MALFORMED.into()))
}

// One server-side PBKDF2 at a time: refuse rather than queue.
static UNWRAPS: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(1);

#[instrument(skip_all)]
#[post(
    "/configuration-package-intake/unwrap",
    format = "json",
    data = "<body>"
)]
pub async fn unwrap_configuration_package(
    _first_party: FirstParty,
    claims: JwtClaims,
    // Raw and unparsed: `Data` never fails as a guard, so nothing logs it.
    body: Data<'_>,
) -> Result<Json<UnwrapOutput>, (Status, String)> {
    let tenant_id = claims.hasura_claims.tenant_id.clone();
    // Authorize before the body is even read.
    authorize(&claims, true, Some(tenant_id.clone()), intake_permissions())?;
    let capped = body
        .open((MAX_BODY_BYTES + 1).bytes())
        .into_bytes()
        .await
        .map_err(|_| (Status::BadRequest, MALFORMED.to_string()))?;
    let complete = capped.is_complete();
    let raw = Zeroizing::new(capped.into_inner());
    if !complete {
        return Err((Status::PayloadTooLarge, MALFORMED.into()));
    }
    let UnwrapInput {
        document_id,
        password,
        expected_sha256,
    } = parse_input(&raw)?;
    drop(raw);
    let password = Zeroizing::new(password);
    let tenant = uuid::Uuid::parse_str(&tenant_id).map_err(internal)?;
    let document = uuid::Uuid::parse_str(&document_id).map_err(|_| {
        (
            Status::BadRequest,
            package_credentials::NOT_STAGING.to_string(),
        )
    })?;
    let permit = UNWRAPS.try_acquire().map_err(|_| {
        (
            Status::TooManyRequests,
            "Another package is being unwrapped; try again.".to_string(),
        )
    })?;

    let mut db: DbClient =
        get_hasura_pool().await.get().await.map_err(internal)?;
    let tx = db.transaction().await.map_err(internal)?;
    audit::act_for(&tx, &claims).await?;
    let now = chrono::Utc::now();
    // TTL cleanup: this tenant's expired staged credentials leave the vault.
    package_credentials::sweep_expired(&tx, &tenant, now)
        .await
        .map_err(internal)?;
    // Only the verified user that staged this upload (the same JWT-derived
    // ID the audit actor and Apply's task carry), and only once.
    package_credentials::lock_staging(
        &tx,
        &tenant,
        &document,
        &claims.hasura_claims.user_id,
    )
    .await
    .map_err(|error| refusal(&error))?;
    let staged = Zeroizing::new(
        documents::read_protected_configuration_package(
            &tx,
            tenant,
            &document_id,
        )
        .await
        .map_err(internal)?,
    );
    let unwrapped = tokio::task::spawn_blocking(move || {
        let _permit = permit;
        package_credentials::unwrap_delivery(&tenant, &staged, &password)
    })
    .await
    .map_err(internal)?
    .map_err(|error| refusal(&error))?;
    // Before the vault or the staged object change: a mismatch stages nothing.
    package_credentials::check_expected(&unwrapped, expected_sha256.as_deref())
        .map_err(|error| refusal(&error))?;
    let expires_at = package_credentials::stage_unwrapped(
        &tx, &tenant, &document, &unwrapped, now,
    )
    .await
    .map_err(|error| refusal(&error))?;
    // A server-only sibling: the browser's presigned upload URL can't reach
    // it. Unreferenced until the pointer below commits.
    let (unwrapped_id, original_key) =
        documents::write_unwrapped_configuration_package(
            &tx,
            tenant,
            &document_id,
            &unwrapped.signed_zip,
        )
        .await
        .map_err(internal)?;
    // The vault entry, the sibling pointer and the expiry publish together.
    documents::publish_unwrapped_pointer(
        &tx,
        tenant,
        document,
        unwrapped_id,
        expires_at,
    )
    .await
    .map_err(|error| refusal(&error))?;
    tx.commit().await.map_err(internal)?;
    windmill::tasks::audit_log_outbox::kick_audit_log_outbox();
    // Best effort, after commit: the encrypted original is no longer read.
    if documents::remove_encrypted_original(original_key)
        .await
        .is_err()
    {
        tracing::warn!(
            "could not remove an unwrapped package's encrypted original"
        );
    }
    Ok(Json(UnwrapOutput {
        sha256: unwrapped.package_sha256.clone(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_first_party_https_or_loopback_http_is_accepted() {
        // Annotated so each call may borrow for its own lifetime (`origin`).
        let ok =
            |h: &str, p: Option<&str>, o: Option<&str>, s: Option<&str>| {
                first_party(Some(h), p, o, s)
            };
        // Through the first-party HTTPS proxy, same origin.
        assert!(ok(
            "admin.example.invalid",
            Some("https"),
            Some("https://admin.example.invalid"),
            Some("same-origin")
        ));
        assert!(ok("admin.example.invalid", Some("https"), None, None));
        // Plain HTTP only to loopback.
        for host in ["localhost:3002", "127.0.0.1", "[::1]:8080", "localhost"] {
            let origin = format!("http://{host}");
            assert!(ok(
                host,
                Some("http"),
                Some(origin.as_str()),
                Some("same-origin")
            ));
            assert!(ok(host, None, None, None));
        }
        for (host, proto) in [
            ("admin.example.invalid", Some("http")),
            ("admin.example.invalid", None),
            ("localhost.example.invalid", None),
            ("127.0.0.1.example.invalid", Some("http")),
            ("admin.example.invalid", Some("ftp")),
        ] {
            assert!(!ok(host, proto, None, None), "{host} {proto:?}");
        }
        // Cross-origin or cross-site browser requests.
        assert!(!ok(
            "admin.example.invalid",
            Some("https"),
            Some("https://evil.example.invalid"),
            None
        ));
        assert!(!ok(
            "admin.example.invalid",
            Some("https"),
            Some("http://admin.example.invalid"),
            None
        ));
        assert!(!ok(
            "admin.example.invalid",
            Some("https"),
            None,
            Some("cross-site")
        ));
        assert!(!ok(
            "admin.example.invalid",
            Some("https"),
            None,
            Some("same-site")
        ));
        assert!(!first_party(None, Some("https"), None, None));
        assert!(!first_party(Some(""), Some("https"), None, None));
    }

    #[test]
    fn a_malformed_body_is_refused_without_echoing_any_of_it() {
        const SENTINEL: &str = "synthetic-password-sentinel";
        let good = format!(
            "{{\"document_id\":\"00000000-0000-4000-8000-000000000007\",\"password\":\"{SENTINEL}\"}}"
        );
        assert!(parse_input(good.as_bytes()).is_ok());
        let oversize = format!(
            "{{\"document_id\":\"d\",\"password\":\"{SENTINEL}{}\"}}",
            "x".repeat(MAX_BODY_BYTES)
        );
        for malformed in [
            // Truncated JSON: exactly what Rocket's Json guard would log raw.
            format!("{{\"document_id\":\"d\",\"password\":\"{SENTINEL}"),
            format!("{{\"document_id\":1,\"password\":\"{SENTINEL}\"}}"),
            format!("{{\"password\":\"{SENTINEL}\",\"messaging_credentials\":\"x\"}}"),
            format!("[\"{SENTINEL}\"]"),
            String::new(),
            oversize,
        ] {
            let refused = match parse_input(malformed.as_bytes()) {
                Ok(_) => panic!("a malformed body was accepted"),
                Err(refused) => refused,
            };
            assert_eq!(refused, (Status::BadRequest, MALFORMED.to_string()));
            assert!(!refused.1.contains(SENTINEL));
        }
    }

    #[test]
    fn intake_needs_staging_and_messaging_permissions() {
        let needed = intake_permissions();
        for permission in
            super::super::apply_configuration_package::PACKAGE_PERMISSIONS
        {
            assert!(needed.contains(&permission));
        }
        for permission in [
            Permissions::MESSAGING_ACCOUNT_READ,
            Permissions::MESSAGING_ACCOUNT_WRITE,
            Permissions::MESSAGING_CONFIG_WRITE,
        ] {
            assert!(needed.contains(&permission));
        }
    }

    #[test]
    fn the_password_is_never_printed_and_only_fixed_refusals_are_answered() {
        const CANARY: &str = "synthetic-transfer-password";
        let input: UnwrapInput = serde_json::from_value(serde_json::json!({
            "document_id": "00000000-0000-4000-8000-000000000007", "password": CANARY,
        }))
        .unwrap();
        assert!(!format!("{input:?}").contains(CANARY));
        assert!(serde_json::from_value::<UnwrapInput>(serde_json::json!({
            "document_id": "x", "password": "y", "messaging_credentials": "z",
        }))
        .is_err());
        for public in [
            package_credentials::NOT_V2,
            package_credentials::ALREADY_STAGED,
            package_credentials::NOT_STAGING,
            package_credentials::MISMATCH,
            package_credentials::HASH_MALFORMED,
        ] {
            assert_eq!(refusal(&anyhow::anyhow!(public)).0, Status::BadRequest);
        }
        assert_eq!(
            refusal(&anyhow::anyhow!(package_credentials::HASH_MISMATCH)),
            (
                Status::Conflict,
                package_credentials::HASH_MISMATCH.to_string()
            )
        );
        assert_eq!(
            refusal(&anyhow::anyhow!(package_credentials::NOT_UPLOADER)).0,
            Status::Forbidden
        );
        assert_eq!(
            refusal(&anyhow::anyhow!(
                "this staging document was already unwrapped"
            )),
            (
                Status::BadRequest,
                package_credentials::ALREADY_STAGED.to_string()
            )
        );
        // The optional expected digest is accepted and may be absent.
        let with_hash: UnwrapInput = serde_json::from_value(serde_json::json!({
            "document_id": "d", "password": CANARY, "expected_sha256": "ab".repeat(32),
        }))
        .unwrap();
        assert_eq!(with_hash.expected_sha256, Some("ab".repeat(32)));
        assert!(!format!("{with_hash:?}").contains(CANARY));
        assert_eq!(input.expected_sha256, None);
        let internal = refusal(&anyhow::anyhow!("vault detail {CANARY}"));
        assert_eq!(
            internal,
            (Status::InternalServerError, GENERIC.to_string())
        );
    }
}
