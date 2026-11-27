// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The receiver half of messaging credentials (AWS key pairs, Infobip API
//! keys) that travel beside a signed delivery.
//!
//! Stage (direct Harvest intake only, never a Hasura action): the browser
//! puts the still-encrypted V2 `.ezip` in protected staging and sends only
//! the staging document and its transfer password to Harvest over the
//! first-party route. Only the subject recorded at staging may unwrap, once
//! ([`lock_staging`]). [`unwrap_delivery`] decrypts it server side;
//! [`stage_unwrapped`] stores the sidecar in the tenant vault, bound to the
//! unwrapped package's SHA-256 and an expiry, and the caller writes the plain
//! signed ZIP to a server-only sibling whose pointer it publishes in the same
//! transaction. [`sweep_expired`] removes expired entries. The sidecar never
//! reaches JavaScript, a document, a task row, a preview or a log line.
//!
//! Apply: a staged sidecar is read only for the same tenant and protected
//! staging document, before its expiry and for the same package bytes,
//! verified against the signed declarations' commitments, and its pairs
//! written as the reviewed accounts' vault credentials in the messaging-accounts
//! section's transaction. The staged secret is deleted in that same
//! transaction, so a failure rolls back keys, accounts and deletion together.

use crate::postgres::messaging::record_credentials_replaced;
use crate::services::messaging::keys::credential_secret_key;
use crate::services::vault::{delete_secret, read_secret, replace_secret};
use anyhow::{anyhow, ensure, Result};
use chrono::{DateTime, Duration, Utc};
use deadpool_postgres::Transaction;
use sequent_core::election_config::encrypted_delivery::{
    decrypt_signed_delivery_parts, is_encrypted_signed_delivery, EncryptedDeliveryError,
};
use sequent_core::election_config::messaging_credentials::{
    verify_sidecar, CredentialSidecar, DeliveredCredential, Zeroizing,
};
use sequent_core::election_config::portable_messaging::PortableMessagingAccounts;
use sequent_core::signing::sha256_hex;
use sequent_core::types::messaging::CredentialName;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use uuid::Uuid;

/// Public: allowlisted operator messages (they start like the other
/// portable-messaging blockers). None names a value or a vault key.
pub const NOT_ACCEPTED: &str = "portable messaging requires the messaging credentials delivered with this encrypted package; upload the original .ezip again";
pub const MISMATCH: &str =
    "portable messaging requires messaging credentials that match the signed package";
pub const EXPIRED: &str = "portable messaging requires messaging credentials staged in the last 12 hours; upload the original .ezip again";
/// Public intake refusals (fixed strings, no values).
pub const NOT_V2: &str =
    "This upload is not an encrypted configuration package with messaging credentials.";
pub const ALREADY_STAGED: &str =
    "Messaging credentials were already staged for this upload; upload the original .ezip again.";
pub const NOT_STAGING: &str = "This upload is not in protected configuration package staging.";
pub const HASH_MISMATCH: &str =
    "The entered SHA-256 does not match the signed ZIP in this encrypted package.";
pub const HASH_MALFORMED: &str = "The entered SHA-256 must be 64 hexadecimal digits.";

/// The operator's optional expected digest of the inner signed ZIP, checked
/// after decryption and before anything is staged or replaced.
pub fn check_expected(unwrapped: &Unwrapped, expected: Option<&str>) -> Result<()> {
    let Some(expected) = expected else {
        return Ok(());
    };
    ensure!(
        expected.len() == 64 && expected.bytes().all(|b| b.is_ascii_hexdigit()),
        HASH_MALFORMED
    );
    ensure!(
        unwrapped.package_sha256.eq_ignore_ascii_case(expected),
        HASH_MISMATCH
    );
    Ok(())
}

/// How long a staged sidecar may wait for Preview and Apply.
pub fn stage_ttl() -> Duration {
    Duration::hours(12)
}

/// What the vault holds for a staging document: the canonical sidecar,
/// bound to the exact unwrapped package bytes and an expiry.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StagedSidecar {
    package_sha256: String,
    expires_at: DateTime<Utc>,
    sidecar: CredentialSidecar,
}

/// A server-unwrapped V2 delivery. Never printed.
pub struct Unwrapped {
    pub signed_zip: Vec<u8>,
    /// Lowercase hex, as Apply computes `package_sha256`.
    pub package_sha256: String,
    sidecar: CredentialSidecar,
}

impl fmt::Debug for Unwrapped {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Unwrapped")
            .field("package_sha256", &self.package_sha256)
            .finish_non_exhaustive()
    }
}

/// Decrypt a staged V2 `.ezip` with its transfer password. CPU bound
/// (PBKDF2); call off the async executor. Errors are fixed public strings.
pub fn unwrap_delivery(tenant: &Uuid, staged: &[u8], password: &str) -> Result<Unwrapped> {
    ensure!(is_encrypted_signed_delivery(staged), NOT_V2);
    let mut parts =
        decrypt_signed_delivery_parts(staged, password).map_err(|error| match error {
            EncryptedDeliveryError::InvalidEnvelope => anyhow!(NOT_V2),
            other => anyhow!("{other}"),
        })?;
    let credentials = parts.credentials.take().ok_or_else(|| anyhow!(NOT_V2))?;
    let sidecar = CredentialSidecar::parse(&credentials).map_err(|_| anyhow!(MISMATCH))?;
    ensure!(sidecar.tenant().ok() == Some(*tenant), MISMATCH);
    let signed_zip = std::mem::take(&mut parts.signed_zip);
    Ok(Unwrapped {
        package_sha256: sha256_hex(&signed_zip),
        signed_zip,
        sidecar,
    })
}

pub const NOT_UPLOADER: &str =
    "Only the administrator who uploaded this package can unlock it; upload it again.";
/// Preview/Apply by someone other than the uploader (allowlisted blocker).
pub const STAGED_BY_OTHER: &str = "portable messaging requires the administrator who uploaded this encrypted package to preview and apply it";

/// Who may unwrap a staging row: its recorded uploader, once. Rows staged
/// before uploaders were recorded can't be unwrapped server side.
fn may_unwrap(uploaded_by: Option<&str>, already_unwrapped: bool, actor: &str) -> Result<()> {
    ensure!(!already_unwrapped, ALREADY_STAGED);
    ensure!(
        !actor.is_empty() && uploaded_by == Some(actor),
        NOT_UPLOADER
    );
    Ok(())
}

/// Lock this tenant's own protected staging row for the intake transaction,
/// for the verified subject that staged it, before it is unwrapped.
pub async fn lock_staging(
    tx: &Transaction<'_>,
    tenant: &Uuid,
    document_id: &Uuid,
    actor: &str,
) -> Result<()> {
    let row = tx
        .query_opt(
            "SELECT uploaded_by, unwrapped_id IS NOT NULL
             FROM sequent_backend.configuration_package_staging
             WHERE tenant_id = $1 AND document_id = $2 FOR UPDATE",
            &[tenant, document_id],
        )
        .await?
        .ok_or_else(|| anyhow!(NOT_STAGING))?;
    let uploaded_by: Option<String> = row.try_get(0)?;
    may_unwrap(uploaded_by.as_deref(), row.try_get(1)?, actor)
}

/// Store the sidecar once per staging document (caller holds the lock).
/// Returns the expiry, which the caller publishes with the unwrapped object.
pub async fn stage_unwrapped(
    tx: &Transaction<'_>,
    tenant: &Uuid,
    document_id: &Uuid,
    unwrapped: &Unwrapped,
    now: DateTime<Utc>,
) -> Result<DateTime<Utc>> {
    let tenant_text = tenant.to_string();
    let key = staged_secret_key(document_id);
    ensure!(
        read_secret(tx, &tenant_text, None, &key).await?.is_none(),
        ALREADY_STAGED
    );
    let expires_at = now + stage_ttl();
    let record = Zeroizing::new(serde_json::to_string(&StagedSidecar {
        package_sha256: unwrapped.package_sha256.clone(),
        expires_at,
        sidecar: unwrapped.sidecar.clone(),
    })?);
    replace_secret(tx, &tenant_text, &key, &record).await?;
    Ok(expires_at)
}

/// Delete this tenant's expired staged credentials from the vault and clear
/// their expiry. Run at every intake; [`load`] refuses an expired entry
/// regardless. Rows another transaction holds are left for the next sweep.
/// The unwrapped signed ZIP stays (it holds no credentials).
pub async fn sweep_expired(
    tx: &Transaction<'_>,
    tenant: &Uuid,
    now: DateTime<Utc>,
) -> Result<usize> {
    let expired = tx
        .query(
            "SELECT document_id FROM sequent_backend.configuration_package_staging
             WHERE tenant_id = $1 AND credentials_expires_at <= $2
             ORDER BY document_id
             FOR UPDATE SKIP LOCKED",
            &[tenant, &now],
        )
        .await?;
    for row in &expired {
        let document_id: Uuid = row.try_get(0)?;
        clear_expired(tx, tenant, &document_id).await?;
    }
    Ok(expired.len())
}

/// The scheduled, tenant-independent sweep: at most `limit` expired rows
/// per pass, oldest expiry first, skipping rows a transaction holds.
pub async fn sweep_expired_everywhere(
    tx: &Transaction<'_>,
    now: DateTime<Utc>,
    limit: i64,
) -> Result<usize> {
    let expired = tx
        .query(
            "SELECT tenant_id, document_id FROM sequent_backend.configuration_package_staging
             WHERE credentials_expires_at <= $1
             ORDER BY credentials_expires_at, tenant_id, document_id
             LIMIT $2
             FOR UPDATE SKIP LOCKED",
            &[&now, &limit],
        )
        .await?;
    for row in &expired {
        let tenant: Uuid = row.try_get(0)?;
        let document_id: Uuid = row.try_get(1)?;
        clear_expired(tx, &tenant, &document_id).await?;
    }
    Ok(expired.len())
}

/// The staging row (already locked by the caller), then its vault entry.
async fn clear_expired(tx: &Transaction<'_>, tenant: &Uuid, document_id: &Uuid) -> Result<()> {
    tx.execute(
        "UPDATE sequent_backend.configuration_package_staging
         SET credentials_expires_at = NULL WHERE tenant_id = $1 AND document_id = $2",
        &[tenant, document_id],
    )
    .await?;
    delete_secret(tx, &tenant.to_string(), &staged_secret_key(document_id)).await?;
    Ok(())
}

/// The vault key of a staged sidecar. Vault keys are unique across tenants;
/// the staging document ID is too.
pub fn staged_secret_key(document_id: &Uuid) -> String {
    format!("configuration-package-staging-{document_id}-messaging-credentials")
}

/// Verified credentials by package reference key (`package:<reference>`).
pub struct StagedCredentials(BTreeMap<String, DeliveredCredential>);

impl fmt::Debug for StagedCredentials {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("StagedCredentials")
            .field(&self.0.keys().collect::<Vec<_>>())
            .finish()
    }
}

impl StagedCredentials {
    pub fn get(&self, reference_key: &str) -> Option<&DeliveredCredential> {
        self.0.get(reference_key)
    }
}

/// Pure decision over what was found, so the fail-closed cases are tested
/// without a database.
/// The task's verified actor and the staging row's recorded uploader.
#[derive(Clone, Copy)]
struct Who<'a> {
    actor: Option<&'a str>,
    uploader: Option<&'a str>,
}

impl Who<'_> {
    fn same(&self) -> bool {
        self.actor
            .is_some_and(|actor| !actor.is_empty() && self.uploader == Some(actor))
    }
}

/// `needs`: this plan creates an account whose keys must come from the
/// sidecar. A staged sidecar is always checked when present (so a resumed
/// Apply still consumes it), but only required when `needs`.
#[allow(clippy::too_many_arguments)]
fn decide(
    definitions: &PortableMessagingAccounts,
    tenant: &Uuid,
    package_sha256: &str,
    staged_document: Option<&Uuid>,
    stored: Option<&str>,
    needs: bool,
    who: Who<'_>,
    now: DateTime<Utc>,
) -> Result<Option<StagedCredentials>> {
    let committed = definitions
        .accounts
        .iter()
        .any(|account| account.credential_commitment.is_some());
    match (staged_document, stored) {
        // Nothing staged: fine unless a new account needs its keys. Every V1
        // delivery is here, with no actor requirement at all.
        (_, None) if needs => Err(anyhow!(NOT_ACCEPTED)),
        (_, None) => Ok(None),
        (None, Some(_)) => Err(anyhow!(NOT_ACCEPTED)),
        // Delivered keys the signed package never committed to.
        (Some(_), Some(_)) if !committed => Err(anyhow!(MISMATCH)),
        // Staged credentials are only for the user who uploaded them.
        (Some(_), Some(_)) if !who.same() => Err(anyhow!(STAGED_BY_OTHER)),
        (Some(_), Some(text)) => {
            let staged: StagedSidecar =
                serde_json::from_str(text).map_err(|_| anyhow!(MISMATCH))?;
            ensure!(now < staged.expires_at, EXPIRED);
            ensure!(
                staged.package_sha256.eq_ignore_ascii_case(package_sha256),
                MISMATCH
            );
            staged.sidecar.validate().map_err(|_| anyhow!(MISMATCH))?;
            verify_sidecar(definitions, tenant, &staged.sidecar)
                .map(|pairs| Some(StagedCredentials(pairs)))
                .map_err(|_| anyhow!(MISMATCH))
        }
    }
}

/// Prevent expiry, consumption or restaging between the verified Apply
/// read and its atomic install/discard. Preview never takes this write lock.
pub async fn lock_for_apply(tx: &Transaction<'_>, tenant: &Uuid, document_id: &Uuid) -> Result<()> {
    let staging = tx
        .query_opt(
            "SELECT 1 FROM sequent_backend.configuration_package_staging
         WHERE tenant_id = $1 AND document_id = $2 FOR UPDATE NOWAIT",
            &[tenant, document_id],
        )
        .await
        .map_err(|_| anyhow!(NOT_ACCEPTED))?;
    ensure!(staging.is_some(), NOT_ACCEPTED);
    Ok(())
}

/// The staged pairs, verified against the signed commitments. `None` when
/// nothing is staged and nothing needs it. A needed sidecar that is missing,
/// or a staged one the package never committed to, is refused.
/// `actor` is the task's verified actor (the JWT-derived audit user ID the
/// server put in the task body), never anything from the package or request.
///
/// The vault is read only when the staging row says credentials are pending
/// (server unwrapped, not yet consumed or swept). Ordinary deliveries never
/// touch it.
pub async fn load(
    tx: &Transaction<'_>,
    tenant: &Uuid,
    package_sha256: &str,
    staged_document: Option<&Uuid>,
    definitions: &PortableMessagingAccounts,
    needs: bool,
    actor: Option<&str>,
) -> Result<Option<StagedCredentials>> {
    let (uploader, pending): (Option<String>, bool) = match staged_document {
        Some(document_id) => tx
            .query_opt(
                "SELECT uploaded_by, credentials_expires_at IS NOT NULL
                 FROM sequent_backend.configuration_package_staging
                 WHERE tenant_id = $1 AND document_id = $2",
                &[tenant, document_id],
            )
            .await?
            .map(|row| -> Result<_> { Ok((row.try_get(0)?, row.try_get(1)?)) })
            .transpose()?
            .unwrap_or((None, false)),
        None => (None, false),
    };
    let stored = match (staged_document, pending) {
        (Some(document_id), true) => read_secret(
            tx,
            &tenant.to_string(),
            None,
            &staged_secret_key(document_id),
        )
        .await
        .map_err(|_| anyhow!(NOT_ACCEPTED))?
        .map(Zeroizing::new),
        _ => None,
    };
    decide(
        definitions,
        tenant,
        package_sha256,
        staged_document,
        stored.as_ref().map(|text| text.as_str()),
        needs,
        Who {
            actor,
            uploader: uploader.as_deref(),
        },
        Utc::now(),
    )
}

/// The vault credentials a delivered credential becomes: an AWS pair's two,
/// or an Infobip account's API key. Never printed.
fn vault_credentials(credential: &DeliveredCredential) -> Vec<(CredentialName, &str)> {
    match credential {
        DeliveredCredential::AwsKeyPair(pair) => vec![
            (
                CredentialName::AWS_ACCESS_KEY_ID,
                pair.aws_access_key_id.as_str(),
            ),
            (
                CredentialName::AWS_SECRET_ACCESS_KEY,
                pair.aws_secret_access_key.as_str(),
            ),
        ],
        DeliveredCredential::ApiKey(key) => vec![(CredentialName::API_KEY, key.as_str())],
    }
}

/// The reviewed account's vault credentials, plus their replacement
/// metadata (names and time only), in the caller's transaction.
pub async fn install(
    tx: &Transaction<'_>,
    tenant: &Uuid,
    account_id: &Uuid,
    credential: &DeliveredCredential,
) -> Result<()> {
    let tenant_text = tenant.to_string();
    let credentials = vault_credentials(credential);
    for (name, value) in &credentials {
        replace_secret(
            tx,
            &tenant_text,
            &credential_secret_key(account_id, *name),
            value,
        )
        .await?;
    }
    let names: Vec<CredentialName> = credentials.iter().map(|(name, _)| *name).collect();
    record_credentials_replaced(tx, tenant, account_id, &names, Utc::now()).await
}

/// Remove the staged sidecar, in the transaction that installed its keys.
///
/// Lock order, everywhere: the staging row, then its vault entry (as
/// [`lock_staging`] then [`stage_unwrapped`], and [`sweep_expired`]).
pub async fn discard(tx: &Transaction<'_>, tenant: &Uuid, document_id: &Uuid) -> Result<()> {
    tx.execute(
        "UPDATE sequent_backend.configuration_package_staging
         SET credentials_expires_at = NULL WHERE tenant_id = $1 AND document_id = $2",
        &[tenant, document_id],
    )
    .await?;
    delete_secret(tx, &tenant.to_string(), &staged_secret_key(document_id)).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sequent_core::election_config::messaging_credentials::{
        commitment, AwsStaticKeyPair, SidecarEntry, SidecarFormat,
    };
    use sequent_core::election_config::portable_messaging::parse as parse_accounts;

    const TENANT: &str = "11111111-1111-4111-8111-111111111111";
    const KEY_ID: &str = "AKIASYNTHETICEXAMPLE";
    const SECRET: &str = "SyntheticSecretValueForTestsOnly00000000";

    fn entry() -> SidecarEntry {
        SidecarEntry {
            reference: "email".into(),
            nonce: "07".repeat(32),
            aws_access_key_id: KEY_ID.into(),
            aws_secret_access_key: SECRET.into(),
            infobip_api_key: None,
        }
    }

    const API_KEY: &str = "synthetic-infobip-api-key-0123456789abcdef";

    #[test]
    fn a_staged_api_key_is_verified_and_becomes_the_accounts_api_key() {
        let tenant = Uuid::parse_str(TENANT).unwrap();
        let key = SidecarEntry {
            reference: "viber".into(),
            nonce: "07".repeat(32),
            aws_access_key_id: String::new(),
            aws_secret_access_key: String::new(),
            infobip_api_key: Some(API_KEY.into()),
        };
        let account = serde_json::json!({"reference":"viber","name":"Demo Viber",
            "sender":{"provider":"VIBER_INFOBIP","base_url":"https://abc-1.api.infobip.com","sender":"Demo"},
            "credential_commitment": commitment(&tenant, &key).unwrap()});
        let committed = parse_accounts(
            &serde_json::to_vec(&serde_json::json!({
                "format":"sequent.tenant-messaging-accounts/1","accounts":[account]}))
            .unwrap(),
        )
        .unwrap();
        let stored = serde_json::json!({
            "package_sha256": PACKAGE.repeat(32),
            "expires_at": now() + stage_ttl(),
            "sidecar": CredentialSidecar {
                format: SidecarFormat::V1,
                tenant_id: TENANT.into(),
                accounts: vec![key],
            },
        })
        .to_string();
        let staged = decide(
            &committed,
            &tenant,
            &PACKAGE.repeat(32),
            Some(&Uuid::from_u128(7)),
            Some(&stored),
            true,
            UPLOADER,
            now(),
        )
        .unwrap()
        .unwrap();
        let credential = staged.get("package:viber").unwrap();
        let names: Vec<_> = vault_credentials(credential)
            .into_iter()
            .map(|(name, value)| {
                assert_eq!(value, API_KEY);
                name
            })
            .collect();
        assert_eq!(names, [CredentialName::API_KEY]);
        assert!(!format!("{staged:?} {credential:?}").contains(API_KEY));
    }

    fn sidecar(tenant: &str) -> String {
        let doc = CredentialSidecar {
            format: SidecarFormat::V1,
            tenant_id: tenant.into(),
            accounts: vec![entry()],
        };
        serde_json::to_string(&doc).unwrap()
    }

    fn definitions(commit: Option<String>) -> PortableMessagingAccounts {
        let mut account = serde_json::json!({"reference":"email","name":"Demo Email",
            "sender":{"provider":"AWS_SES","from_address":"demo@example.invalid","region":"us-east-1"}});
        if let Some(c) = commit {
            account["credential_commitment"] = c.into();
        }
        let doc = serde_json::json!({"format":"sequent.tenant-messaging-accounts/1","accounts":[account]});
        parse_accounts(&serde_json::to_vec(&doc).unwrap()).unwrap()
    }

    const UPLOADER: Who<'static> = Who {
        actor: Some("user-a"),
        uploader: Some("user-a"),
    };
    const OTHER: Who<'static> = Who {
        actor: Some("user-b"),
        uploader: Some("user-a"),
    };
    const NOBODY: Who<'static> = Who {
        actor: None,
        uploader: Some("user-a"),
    };
    const PACKAGE: &str = "ab";
    const PASSWORD: &str = "synthetic transfer password";
    const ZIP: &[u8] = b"PK\x03\x04synthetic-signed-zip";

    fn now() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-11-25T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc)
    }

    fn record(tenant: &str, package: &str, expires_at: DateTime<Utc>) -> String {
        let parsed = CredentialSidecar::parse(sidecar(tenant).as_bytes());
        let value = match parsed {
            Ok(parsed) => serde_json::to_value(parsed).unwrap(),
            // A foreign tenant still parses; anything else is left raw.
            Err(_) => serde_json::from_str::<serde_json::Value>(&sidecar(tenant)).unwrap(),
        };
        serde_json::json!({
            "package_sha256": package.repeat(32),
            "expires_at": expires_at,
            "sidecar": value,
        })
        .to_string()
    }

    #[test]
    fn without_a_stage_every_committed_account_fails_closed() {
        let tenant = Uuid::parse_str(TENANT).unwrap();
        let document = Uuid::from_u128(7);
        let sha = PACKAGE.repeat(32);
        let committed = definitions(Some(commitment(&tenant, &entry()).unwrap()));
        for staged in [None, Some(&document)] {
            let refused = decide(
                &committed,
                &tenant,
                &sha,
                staged,
                None,
                true,
                UPLOADER,
                now(),
            )
            .unwrap_err()
            .to_string();
            assert_eq!(refused, NOT_ACCEPTED);
        }
        // Omitted (every V1 delivery): unchanged, and no actor needed at all,
        // not even for an actor-less legacy task or another administrator.
        let plain = definitions(None);
        for who in [UPLOADER, NOBODY, OTHER] {
            assert!(decide(&plain, &tenant, &sha, None, None, false, who, now())
                .unwrap()
                .is_none());
            assert!(decide(
                &plain,
                &tenant,
                &sha,
                Some(&document),
                None,
                false,
                who,
                now()
            )
            .unwrap()
            .is_none());
        }
        for message in [NOT_ACCEPTED, MISMATCH, EXPIRED, STAGED_BY_OTHER] {
            assert!(message.starts_with("portable messaging requires "));
        }
    }

    #[test]
    fn a_staged_sidecar_must_match_commitments_tenant_package_and_time() {
        let tenant = Uuid::parse_str(TENANT).unwrap();
        let document = Uuid::from_u128(7);
        let sha = PACKAGE.repeat(32);
        let committed = definitions(Some(commitment(&tenant, &entry()).unwrap()));
        let later = now() + stage_ttl();
        let ours = record(TENANT, PACKAGE, later);
        let staged = decide(
            &committed,
            &tenant,
            &sha,
            Some(&document),
            Some(&ours),
            true,
            UPLOADER,
            now(),
        )
        .unwrap()
        .unwrap();
        // A resumed Apply (its accounts exist, nothing needs keys) still
        // consumes and checks a staged sidecar; with none left, nothing to do.
        assert!(decide(
            &committed,
            &tenant,
            &sha,
            Some(&document),
            Some(&ours),
            false,
            UPLOADER,
            now()
        )
        .unwrap()
        .is_some());
        assert!(decide(
            &committed,
            &tenant,
            &sha,
            Some(&document),
            None,
            false,
            UPLOADER,
            now()
        )
        .unwrap()
        .is_none());
        let Some(DeliveredCredential::AwsKeyPair(pair)) = staged.get("package:email") else {
            panic!("the staged AWS pair");
        };
        assert_eq!(pair.aws_access_key_id.as_str(), KEY_ID);
        // Only the uploader's own Preview/Apply: another authorized account
        // of the tenant, an actor-less task, or a row without an uploader.
        for who in [
            OTHER,
            NOBODY,
            Who {
                actor: Some("user-a"),
                uploader: None,
            },
            Who {
                actor: Some(""),
                uploader: Some(""),
            },
        ] {
            assert_eq!(
                decide(
                    &committed,
                    &tenant,
                    &sha,
                    Some(&document),
                    Some(&ours),
                    true,
                    who,
                    now()
                )
                .unwrap_err()
                .to_string(),
                STAGED_BY_OTHER
            );
        }
        // Never without the server-set staging document.
        assert!(decide(
            &committed,
            &tenant,
            &sha,
            None,
            Some(&ours),
            true,
            UPLOADER,
            now()
        )
        .is_err());
        // Expired: refused (TTL), at and after the deadline.
        assert_eq!(
            decide(
                &committed,
                &tenant,
                &sha,
                Some(&document),
                Some(&ours),
                true,
                UPLOADER,
                later
            )
            .unwrap_err()
            .to_string(),
            EXPIRED
        );
        // Bound to the unwrapped package bytes: another package is refused.
        let other = record(TENANT, "cd", later);
        let foreign = record("22222222-2222-4222-8222-222222222222", PACKAGE, later);
        let garbage = format!("{{\"aws_secret_access_key\":\"{SECRET}\"}}");
        let bare_sidecar = sidecar(TENANT);
        for text in [&other, &foreign, &garbage, &bare_sidecar] {
            let message = decide(
                &committed,
                &tenant,
                &sha,
                Some(&document),
                Some(text),
                true,
                UPLOADER,
                now(),
            )
            .unwrap_err()
            .to_string();
            assert_eq!(message, MISMATCH);
            assert!(!message.contains(SECRET) && !message.contains(KEY_ID));
        }
        // Delivered keys the signed package never committed to.
        assert_eq!(
            decide(
                &definitions(None),
                &tenant,
                &sha,
                Some(&document),
                Some(&ours),
                false,
                UPLOADER,
                now()
            )
            .unwrap_err()
            .to_string(),
            MISMATCH
        );
    }

    #[test]
    fn server_unwrap_takes_only_this_tenants_v2_delivery_and_reveals_nothing() {
        use sequent_core::election_config::encrypted_delivery::{
            encrypt_signed_delivery, encrypt_signed_delivery_with_credentials,
        };
        let tenant = Uuid::parse_str(TENANT).unwrap();
        let ours = sidecar(TENANT);
        let wire =
            encrypt_signed_delivery_with_credentials(ZIP, ours.as_bytes(), PASSWORD).unwrap();
        let unwrapped = unwrap_delivery(&tenant, &wire, PASSWORD).unwrap();
        assert_eq!(unwrapped.signed_zip, ZIP);
        assert_eq!(unwrapped.package_sha256, sha256_hex(ZIP));
        // The optional expected inner digest: absent, exact (any case), or refused.
        let inner = sha256_hex(ZIP);
        assert!(check_expected(&unwrapped, None).is_ok());
        assert!(check_expected(&unwrapped, Some(&inner)).is_ok());
        assert!(check_expected(&unwrapped, Some(&inner.to_uppercase())).is_ok());
        assert_eq!(
            check_expected(&unwrapped, Some(&"0".repeat(64)))
                .unwrap_err()
                .to_string(),
            HASH_MISMATCH
        );
        let non_hex = "g".repeat(64);
        let too_long = format!("{inner}0");
        for malformed in ["", "abc", non_hex.as_str(), too_long.as_str()] {
            assert_eq!(
                check_expected(&unwrapped, Some(malformed))
                    .unwrap_err()
                    .to_string(),
                HASH_MALFORMED
            );
        }
        assert!(!format!("{unwrapped:?}").contains(SECRET));
        let errors = [
            unwrap_delivery(&tenant, &wire, "wrong password").unwrap_err(),
            unwrap_delivery(&tenant, ZIP, PASSWORD).unwrap_err(),
            unwrap_delivery(
                &tenant,
                &encrypt_signed_delivery(ZIP, PASSWORD).unwrap(),
                PASSWORD,
            )
            .unwrap_err(),
            unwrap_delivery(&Uuid::from_u128(9), &wire, PASSWORD).unwrap_err(),
        ];
        let [wrong, plain, v1, foreign] = errors.map(|error| error.to_string());
        assert!(wrong.contains("password is incorrect"));
        assert_eq!(plain, NOT_V2);
        assert_eq!(v1, NOT_V2);
        assert_eq!(foreign, MISMATCH);
        for message in [wrong, plain, v1, foreign] {
            assert!(!message.contains(SECRET) && !message.contains(PASSWORD));
        }
    }

    #[test]
    fn only_the_recorded_uploader_may_unwrap_and_only_once() {
        assert!(may_unwrap(Some("subject-a"), false, "subject-a").is_ok());
        for (uploaded_by, unwrapped, actor, refusal) in [
            // Another administrator of the same tenant.
            (Some("subject-a"), false, "subject-b", NOT_UPLOADER),
            // Rows staged before uploaders were recorded.
            (None, false, "subject-a", NOT_UPLOADER),
            // An empty subject never matches.
            (Some(""), false, "", NOT_UPLOADER),
            // Already unwrapped: refused even for the uploader.
            (Some("subject-a"), true, "subject-a", ALREADY_STAGED),
        ] {
            assert_eq!(
                may_unwrap(uploaded_by, unwrapped, actor)
                    .unwrap_err()
                    .to_string(),
                refusal
            );
        }
    }

    #[test]
    fn staged_key_is_per_document_and_debug_shows_only_references() {
        let a = Uuid::from_u128(1);
        let b = Uuid::from_u128(2);
        assert_ne!(staged_secret_key(&a), staged_secret_key(&b));
        let staged = StagedCredentials(BTreeMap::from([(
            "package:email".to_string(),
            DeliveredCredential::AwsKeyPair(AwsStaticKeyPair {
                aws_access_key_id: Zeroizing::new(KEY_ID.into()),
                aws_secret_access_key: Zeroizing::new(SECRET.into()),
            }),
        )]));
        let printed = format!("{staged:?}");
        assert!(printed.contains("package:email"));
        assert!(!printed.contains(KEY_ID) && !printed.contains(SECRET));
    }
}
