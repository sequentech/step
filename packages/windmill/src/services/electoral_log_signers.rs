// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Who signs an election event's electoral log boards: the platform's own
//! key in its vault (the default, and every event without the setting), or
//! a signer the installation provides (`external`), whose keys are held
//! outside the platform.
//!
//! - The setting is read from the event's presentation **as stored (JSON)**,
//!   which must be an object. Borsh skips it by design (it would otherwise
//!   change `ballot_style_hash`), so a presentation decoded from Borsh always
//!   reads as unset: it is never taken from a ballot style.
//! - When a board's key is made, the choice is recorded with it: an external
//!   board's public key and key version are kept in the vault
//!   ([`BoardKey`]). From then on the setting can't change for that board:
//!   a presentation that disagrees with what was recorded is refused, never
//!   followed.
//! - An `external` board is never signed with software keys. Where no signer
//!   is installed, everything that would sign with its key is refused; its
//!   public key, for verification, stays readable from the record.

use std::sync::{Arc, RwLock};

use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Transaction;
use electoral_log::signer::SharedSigner;
use sequent_core::ballot::ElectoralLogSignerPolicy;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use voting_crypto::signing::VerificationKey;

/// The board a key was made for.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoardOrigin {
    pub tenant_id: String,
    pub election_event_id: String,
    pub board_name: String,
}

/// An external board key, as recorded when it was made.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoardKey {
    /// The signer's own reference to the key, opaque here: the signer is
    /// asked for this key, never for "the key of a board name".
    pub key_ref: String,
    /// Standard base64 of the raw Ed25519 public key.
    pub public_key: String,
    /// The version of the key the signer holds; signing with another
    /// version is refused.
    pub key_version: u32,
    /// The board the key was made for: a key serves that board only, so a
    /// copy of the event under other ids can't reuse it.
    pub origin: BoardOrigin,
}

impl BoardKey {
    pub fn verification_key(&self) -> Result<VerificationKey> {
        VerificationKey::from_b64_string(&self.public_key)
    }
}

/// The signers an installation provides for `external` boards.
///
/// The methods are synchronous and are called from async code: an
/// implementation that waits on a device or another process must not block
/// the runtime (offload the call).
pub trait ExternalLogSigners: Send + Sync {
    /// Makes the board's key where the installation keeps its keys.
    /// Idempotent: called again for a board whose key exists (a retry after
    /// the database rolled back), it returns that key's record and makes
    /// no other.
    fn create_board_signer(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        board_name: &str,
    ) -> Result<BoardKey>;
    /// The board's signer for exactly `key`: it must sign with that key
    /// version and report that public key, or be refused. A board key is
    /// never rotated during an event: the recorded key verifies the board's
    /// whole log, so a signer whose key changed must refuse, not follow.
    fn board_signer(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        board_name: &str,
        key: &BoardKey,
    ) -> Result<SharedSigner>;
}

static INSTALLED: RwLock<Option<Arc<dyn ExternalLogSigners>>> = RwLock::new(None);

/// Installs the external signers; done once, at start, by a build that
/// provides them. Without it, `external` boards are refused.
pub fn install(signers: Arc<dyn ExternalLogSigners>) -> Result<()> {
    let mut installed = INSTALLED
        .write()
        .map_err(|_| anyhow!("the external signers' lock is poisoned"))?;
    if installed.is_some() {
        return Err(anyhow!("external log signers are installed already"));
    }
    *installed = Some(signers);
    Ok(())
}

pub fn installed() -> Result<Arc<dyn ExternalLogSigners>> {
    INSTALLED
        .read()
        .map_err(|_| anyhow!("the external signers' lock is poisoned"))?
        .clone()
        .ok_or_else(|| {
            anyhow!(
                "this board's electoral log is signed by an external signer, and this installation has none: refused"
            )
        })
}

/// The setting in a stored (JSON) presentation; missing means software. A
/// presentation that is neither an object nor null is refused rather than
/// read as "unset".
pub fn policy_of(presentation: Option<&Value>) -> Result<ElectoralLogSignerPolicy> {
    let parsed;
    let presentation = match presentation {
        None | Some(Value::Null) => return Ok(ElectoralLogSignerPolicy::SOFTWARE),
        Some(Value::Object(map)) => map,
        // A presentation stored as a JSON string is read through, not
        // taken as "unset".
        Some(Value::String(text)) => {
            parsed = serde_json::from_str::<Value>(text)
                .context("the event's presentation is a string that is not JSON (refused)")?;
            return policy_of(Some(&parsed));
        }
        Some(_) => {
            return Err(anyhow!(
                "the event's presentation is not a JSON object: its electoral log signer can't be read (refused)"
            ))
        }
    };
    match presentation.get("electoral_log_signer_policy") {
        None | Some(Value::Null) => Ok(ElectoralLogSignerPolicy::SOFTWARE),
        Some(value) => serde_json::from_value(value.clone())
            .context("the event's electoral_log_signer_policy is not software or external"),
    }
}

/// The event's setting, from its stored presentation.
pub async fn event_policy(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<ElectoralLogSignerPolicy> {
    let event = crate::postgres::election_event::get_election_event_by_id(
        hasura_transaction,
        tenant_id,
        election_event_id,
    )
    .await
    .context("reading the event's electoral log signer setting")?;
    policy_of(event.presentation.as_ref())
}

/// The event's setting, read with `FOR SHARE`: while a board's key is made,
/// the setting can't change under it.
pub async fn event_policy_for_share(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<ElectoralLogSignerPolicy> {
    let row = hasura_transaction
        .query_opt(
            "SELECT presentation FROM sequent_backend.election_event \
             WHERE tenant_id = $1 AND id = $2 FOR SHARE",
            &[
                &sequent_core::services::uuid_validation::parse_uuid_v4(tenant_id)?,
                &sequent_core::services::uuid_validation::parse_uuid_v4(election_event_id)?,
            ],
        )
        .await
        .context("reading the event's electoral log signer setting")?
        .ok_or_else(|| anyhow!("the election event does not exist"))?;
    let presentation: Option<Value> = row
        .try_get("presentation")
        .context("reading the event's presentation")?;
    policy_of(presentation.as_ref())
}

/// Where a board's external key record is kept in the vault.
pub fn board_key_path(board_name: &str) -> String {
    format!("boards/{board_name}/external-signer")
}

/// What a board was made with, and whether the event's current setting still
/// agrees: a setting changed after the board's key exists is refused.
pub fn check_unchanged(
    policy: ElectoralLogSignerPolicy,
    external: Option<&BoardKey>,
    has_software_key: bool,
) -> Result<()> {
    match (policy, external, has_software_key) {
        (ElectoralLogSignerPolicy::EXTERNAL, Some(_), false) => Ok(()),
        (ElectoralLogSignerPolicy::SOFTWARE, None, _) => Ok(()),
        (ElectoralLogSignerPolicy::EXTERNAL, None, true) => Err(anyhow!(
            "the event's electoral log is set to an external signer, but this board's key was made in software: the setting can't change once a board's key exists (refused)"
        )),
        (ElectoralLogSignerPolicy::EXTERNAL, None, false) => Err(anyhow!(
            "this board has no external key record: it was not made, or not restored (refused)"
        )),
        (ElectoralLogSignerPolicy::EXTERNAL, Some(_), true) => Err(anyhow!(
            "this board has both an external key and a software key: refused"
        )),
        (ElectoralLogSignerPolicy::SOFTWARE, Some(_), _) => Err(anyhow!(
            "this board's key is held by an external signer, but the event's setting says software: the setting can't change once a board's key exists (refused)"
        )),
    }
}

#[cfg(test)]
#[path = "electoral_log_signers_tests.rs"]
mod tests;
