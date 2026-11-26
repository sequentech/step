// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Receiver-side roster authority. A package never writes this trust state.

use anyhow::{ensure, Result};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::{DateTime, Utc};
use deadpool_postgres::Transaction;
use openssl::{
    bn::BigNumContext,
    ec::{EcGroup, EcKey, EcPoint},
    nid::Nid,
};
use sequent_core::{
    election_config::webauthn_proof::{PinnedRoster, Role, Roster, MAX_REQUIRED_APPROVALS},
    services::uuid_validation::parse_uuid_v4,
};
use serde::{Deserialize, Serialize};

use super::browser_package_policy::{BrowserPolicy, REQUIRED_APPROVALS};

pub const MODE: &str = "portal-managed";
pub const EXPECTED_RP_ID: &str = "WINDMILL_WEBAUTHN_PACKAGES_EXPECTED_RP_ID";
pub const EXPECTED_ORIGIN: &str = "WINDMILL_WEBAUTHN_PACKAGES_EXPECTED_ORIGIN";
pub const MAX_ROSTER_BYTES: usize = 256 * 1024;
// The former roster administrator allowlist is deliberately ignored.
pub const MANAGED_VARS: [&str; 2] = [EXPECTED_RP_ID, EXPECTED_ORIGIN];

#[derive(Debug, Clone)]
pub struct Config {
    tenant_id: String,
    rp_id: String,
    origin: String,
    pub required: usize,
}

pub struct Candidate {
    pub raw: String,
    pub sha256: String,
    pub approver_count: usize,
    pub final_signer_count: usize,
    pub rp_id: String,
    pub origins: Vec<String>,
    pinned: PinnedRoster,
}

pub struct Installed {
    pub raw: String,
    pub sha256: String,
    pub generation: i64,
    pub activated_by: String,
    pub activated_at: DateTime<Utc>,
}

/// The exact receiver authority of a preview, application or resume.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Binding {
    pub sha256: String,
    pub generation: i64,
    pub required_approvals: usize,
}

fn host_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 253
        && name.split('.').all(|part| {
            !part.is_empty()
                && part.len() <= 63
                && !part.starts_with('-')
                && !part.ends_with('-')
                && part
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        })
}

fn exact_https_origin(origin: &str) -> bool {
    let Some(authority) = origin.strip_prefix("https://") else {
        return false;
    };
    if let Some((host, port)) = authority.split_once(':') {
        return host_name(host)
            && port
                .parse::<u16>()
                .is_ok_and(|value| value > 0 && value.to_string() == port);
    }
    host_name(authority)
}

fn valid_subject(subject: &str) -> bool {
    !subject.is_empty() && subject.len() <= 255 && subject.bytes().all(|b| b.is_ascii_graphic())
}

pub fn config(tenant_id: &str) -> std::result::Result<Option<Config>, String> {
    match BrowserPolicy::load(tenant_id) {
        BrowserPolicy::Managed(config) => Ok(Some(config)),
        BrowserPolicy::Unusable(reason) => Err(reason),
        _ => Ok(None),
    }
}

impl Config {
    pub(super) fn from_sources(
        tenant_id: String,
        var: impl Fn(&str) -> Option<String>,
    ) -> std::result::Result<Self, String> {
        let rp_id = var(EXPECTED_RP_ID)
            .filter(|value| host_name(value))
            .ok_or_else(|| format!("{EXPECTED_RP_ID} must be a lowercase host name"))?;
        let origin = var(EXPECTED_ORIGIN)
            .filter(|value| exact_https_origin(value))
            .ok_or_else(|| format!("{EXPECTED_ORIGIN} must be one exact HTTPS origin"))?;
        let required = match var(REQUIRED_APPROVALS) {
            None => 2,
            Some(value) => value.parse::<usize>().ok()
                .filter(|count| (1..=MAX_REQUIRED_APPROVALS).contains(count) && count.to_string() == value)
                .ok_or_else(|| format!("{REQUIRED_APPROVALS} must be an integer from 1 to {MAX_REQUIRED_APPROVALS}"))?,
        };
        Ok(Self {
            tenant_id,
            rp_id,
            origin,
            required,
        })
    }

    pub fn candidate(&self, bytes: &[u8]) -> std::result::Result<Candidate, String> {
        if bytes.is_empty() || bytes.len() > MAX_ROSTER_BYTES {
            return Err("Roster must be between 1 and 262144 UTF-8 bytes".into());
        }
        let raw =
            std::str::from_utf8(bytes).map_err(|_| "Roster must be valid UTF-8".to_owned())?;
        let roster = Roster::read(raw).map_err(|error| error.to_string())?;
        if roster.tenant != self.tenant_id
            || roster.rp_id != self.rp_id
            || roster.origins.as_slice() != [self.origin.as_str()]
        {
            return Err(
                "Roster does not match the receiver's tenant, relying party and origin".into(),
            );
        }
        let sha256 = Roster::sha256(raw);
        let pinned = PinnedRoster::load(raw, &sha256, &self.tenant_id, self.required)
            .map_err(|error| error.to_string())?;
        // The shared portable verifier checks the SEC1 framing. For local
        // provisioning also ensure each advertised key is an actual point
        // on the selected curve rather than activate unusable trust.
        let group = EcGroup::from_curve_name(Nid::X9_62_PRIME256V1)
            .map_err(|_| "The P-256 verifier is unavailable".to_owned())?;
        let mut context =
            BigNumContext::new().map_err(|_| "The P-256 verifier is unavailable".to_owned())?;
        for key in &roster.keys {
            let key_id =
                parse_uuid_v4(&key.key).map_err(|_| "A signer key ID is not a UUID".to_owned())?;
            if key_id.to_string() != key.key || key.credential_id.is_empty() {
                return Err("A signer key ID or credential is not canonical".into());
            }
            let public = URL_SAFE_NO_PAD
                .decode(&key.public_key)
                .map_err(|_| "A signer key is not a valid P-256 public key".to_owned())?;
            EcPoint::from_bytes(&group, &public, &mut context)
                .and_then(|point| EcKey::from_public_key(&group, &point))
                .and_then(|key| key.check_key())
                .map_err(|_| "A signer key is not a valid P-256 public key".to_owned())?;
        }
        Ok(Candidate {
            raw: raw.to_owned(),
            sha256,
            approver_count: roster
                .keys
                .iter()
                .filter(|key| key.role == Role::Approver)
                .count(),
            final_signer_count: roster
                .keys
                .iter()
                .filter(|key| key.role == Role::FinalSigner)
                .count(),
            rp_id: roster.rp_id,
            origins: roster.origins,
            pinned,
        })
    }

    pub(super) fn active(
        &self,
        installed: Installed,
    ) -> std::result::Result<(PinnedRoster, Binding), String> {
        let candidate = self.candidate(installed.raw.as_bytes())?;
        if candidate.sha256 != installed.sha256 || installed.generation <= 0 {
            return Err("Active roster digest or generation is invalid".into());
        }
        let binding = Binding {
            sha256: installed.sha256,
            generation: installed.generation,
            required_approvals: self.required,
        };
        Ok((candidate.pinned, binding))
    }
}

fn installed(row: &tokio_postgres::Row) -> Installed {
    Installed {
        raw: row.get(0),
        sha256: row.get(1),
        generation: row.get(2),
        activated_by: row.get(3),
        activated_at: row.get(4),
    }
}

pub async fn read(tx: &Transaction<'_>, tenant_id: &str) -> Result<Option<Installed>> {
    let tenant_id = parse_uuid_v4(tenant_id)?.to_string();
    Ok(tx
        .query_opt(
            "SELECT roster_json, sha256, generation, activated_by, activated_at
         FROM sequent_backend.managed_webauthn_roster WHERE tenant_id = $1::text::uuid",
            &[&tenant_id],
        )
        .await?
        .as_ref()
        .map(installed))
}

/// The caller holds both package-work locks and stages the audit before
/// committing the same transaction. Every activation advances the token,
/// even if reactivating the same bytes, so rotations cannot cause ABA.
pub async fn install(
    tx: &Transaction<'_>,
    config: &Config,
    candidate: &Candidate,
    expected_generation: i64,
    actor_subject: &str,
) -> Result<Option<Installed>> {
    ensure!(
        valid_subject(actor_subject),
        "Roster actor subject is invalid"
    );
    let next = config
        .candidate(candidate.raw.as_bytes())
        .map_err(anyhow::Error::msg)?;
    ensure!(
        next.sha256 == candidate.sha256,
        "Roster candidate digest differs"
    );
    let tenant_id = &config.tenant_id;
    let previous = read(tx, tenant_id).await?;
    if previous.as_ref().map_or(0, |row| row.generation) != expected_generation {
        return Ok(None);
    }
    let generation = expected_generation
        .checked_add(1)
        .ok_or_else(|| anyhow::anyhow!("Roster generation exhausted"))?;
    let row = if expected_generation == 0 {
        tx.query_opt(
            "INSERT INTO sequent_backend.managed_webauthn_roster
             (tenant_id, roster_json, sha256, generation, activated_by)
             VALUES ($1::text::uuid, $2, $3, $4, $5)
             ON CONFLICT DO NOTHING
             RETURNING roster_json, sha256, generation, activated_by, activated_at",
            &[
                tenant_id,
                &next.raw,
                &next.sha256,
                &generation,
                &actor_subject,
            ],
        )
        .await?
    } else {
        tx.query_opt(
            "UPDATE sequent_backend.managed_webauthn_roster
             SET roster_json = $2, sha256 = $3, generation = $4, activated_by = $5,
                 activated_at = now()
             WHERE tenant_id = $1::text::uuid AND generation = $6
             RETURNING roster_json, sha256, generation, activated_by, activated_at",
            &[
                tenant_id,
                &next.raw,
                &next.sha256,
                &generation,
                &actor_subject,
                &expected_generation,
            ],
        )
        .await?
    };
    Ok(row.as_ref().map(installed))
}

#[cfg(test)]
mod tests {
    use super::*;
    use openssl::ec::PointConversionForm;
    use serde_json::{json, Value};

    const TENANT: &str = "00000000-0000-4000-8000-00000000000a";
    const HOST: &str = "architect.example";
    const ORIGIN: &str = "https://architect.example";

    fn cfg() -> Config {
        Config::from_sources(TENANT.into(), |name| match name {
            EXPECTED_RP_ID => Some(HOST.into()),
            EXPECTED_ORIGIN => Some(ORIGIN.into()),
            _ => None,
        })
        .unwrap()
    }

    fn roster() -> Value {
        let key = |id: u8, role: &str| {
            let group = EcGroup::from_curve_name(Nid::X9_62_PRIME256V1).unwrap();
            let key = EcKey::generate(&group).unwrap();
            let mut context = BigNumContext::new().unwrap();
            let public_key = key
                .public_key()
                .to_bytes(&group, PointConversionForm::UNCOMPRESSED, &mut context)
                .unwrap();
            json!({
                "key": format!("00000000-0000-4000-8000-{id:012x}"), "subject": format!("subject-{id}"),
                "role": role, "algorithm": "es256", "record_sha256": "a".repeat(64),
                "credential_id": URL_SAFE_NO_PAD.encode([id; 32]),
                "public_key": URL_SAFE_NO_PAD.encode(public_key),
            })
        };
        json!({
            "format": sequent_core::election_config::webauthn_proof::ROSTER_FORMAT,
            "tenant": TENANT, "rp_id": HOST, "origins": [ORIGIN],
            "keys": [key(1, "approver"), key(2, "approver"), key(3, "final-signer")],
        })
    }

    #[test]
    fn exact_roster_bytes_produce_the_pin_and_only_the_configured_scope_can_activate() {
        let text = format!("{}\n", roster());
        let candidate = cfg().candidate(text.as_bytes()).unwrap();
        assert_eq!(candidate.raw, text);
        assert_eq!(candidate.sha256, Roster::sha256(&text));
        assert_ne!(candidate.sha256, Roster::sha256(text.trim_end()));
        assert_eq!(
            (candidate.approver_count, candidate.final_signer_count),
            (2, 1)
        );
        assert!(cfg().candidate(&[0xff]).is_err());
        assert!(cfg().candidate(&vec![b' '; MAX_ROSTER_BYTES + 1]).is_err());

        let mut wrong = roster();
        for (field, value) in [
            ("unexpected", json!(true)),
            ("rp_id", json!("other.example")),
            ("origins", json!([ORIGIN, "https://other.example"])),
            ("tenant", json!("00000000-0000-4000-8000-00000000000b")),
        ] {
            wrong[field] = value;
            assert!(cfg().candidate(wrong.to_string().as_bytes()).is_err());
            wrong = roster();
        }
        wrong["keys"][2]["subject"] = json!("subject-1");
        assert!(cfg().candidate(wrong.to_string().as_bytes()).is_err());
        wrong = roster();
        wrong["keys"][2]["credential_id"] = json!("");
        assert!(cfg().candidate(wrong.to_string().as_bytes()).is_err());
        wrong = roster();
        wrong["keys"][2]["key"] = json!("not-a-signer-id");
        assert!(cfg().candidate(wrong.to_string().as_bytes()).is_err());
        wrong = roster();
        wrong["keys"].as_array_mut().unwrap().remove(1);
        assert!(cfg().candidate(wrong.to_string().as_bytes()).is_err());
    }

    #[test]
    fn stored_pin_and_generation_are_never_inferred_from_current_bytes() {
        let raw = roster().to_string();
        let active = |sha256, generation| Installed {
            raw: raw.clone(),
            sha256,
            generation,
            activated_by: "custodian-reader".into(),
            activated_at: Utc::now(),
        };
        assert!(cfg().active(active("f".repeat(64), 1)).is_err());
        assert!(cfg().active(active(Roster::sha256(&raw), 0)).is_err());
        let a = cfg().active(active(Roster::sha256(&raw), 1)).unwrap().1;
        let b = cfg().active(active(Roster::sha256(&raw), 2)).unwrap().1;
        assert_ne!(a, b);
    }
}
