// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Who may do what with which key. The policy is a read-only file the
//! installation writes: callers are named by the SHA-256 of their mTLS
//! certificate's public key, and each key names its users, purposes and gate.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::provider::Algorithm;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    pub callers: Vec<Caller>,
    /// Callers that may create, rotate, retire and destroy keys.
    pub key_administrators: BTreeSet<String>,
    pub keys: Vec<KeyRule>,
    #[serde(default)]
    pub officials: Option<OfficialIssuer>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Caller {
    pub name: String,
    /// Hex SHA-256 of the DER SubjectPublicKeyInfo of the caller's certificate.
    pub spki_sha256: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Operation {
    Sign,
    Ecdh,
    Encrypt,
    Decrypt,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KeyRule {
    /// An exact key id, or a prefix ending in `*` (`official/*`).
    pub key_id: String,
    pub algorithm: Algorithm,
    pub operations: BTreeSet<Operation>,
    pub purposes: BTreeSet<String>,
    pub users: BTreeSet<String>,
    pub gate: Gate,
}

/// What a signature needs besides an authorized caller.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Gate {
    /// System keys used by their service (log entries, checkpoints, HMAC).
    None,
    /// An executor acting on a request whose quorum is complete: the caller
    /// must be one of `executors` and must name the request and the digest
    /// of the exact message, and the service records both.
    CompletedExecution { executors: BTreeSet<String> },
    /// One official's own approval of a pending request: the official's
    /// fresh step-up token must come with it, and its subject must own the
    /// key (`<prefix><subject>`). A completed quorum is not required: this
    /// signature is one of the approvals that make it up.
    OfficialApproval { key_id_prefix: String },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OfficialIssuer {
    pub issuer: String,
    pub jwks_url: String,
    /// The audience the official's step-up token must carry.
    pub audience: String,
    pub max_auth_age_seconds: i64,
    /// Accepted `acr` values for a step-up login.
    pub acr_values: BTreeSet<String>,
    #[serde(default)]
    pub tenant_id: Option<String>,
}

impl KeyRule {
    pub fn matches(&self, key_id: &str) -> bool {
        match self.key_id.strip_suffix('*') {
            Some(prefix) => key_id.starts_with(prefix) && key_id.len() > prefix.len(),
            None => self.key_id == key_id,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PolicyError {
    #[error("the policy is invalid: {0}")]
    Invalid(String),
}

impl Policy {
    pub fn parse(text: &str) -> Result<Self, PolicyError> {
        let policy: Policy =
            serde_json::from_str(text).map_err(|e| PolicyError::Invalid(e.to_string()))?;
        policy.check()?;
        Ok(policy)
    }

    fn check(&self) -> Result<(), PolicyError> {
        let mut names = BTreeSet::new();
        let mut keys = BTreeSet::new();
        for caller in &self.callers {
            if !names.insert(caller.name.clone()) {
                return Err(PolicyError::Invalid(format!(
                    "caller '{}' twice",
                    caller.name
                )));
            }
            if hex::decode(&caller.spki_sha256).map(|b| b.len()) != Ok(32) {
                return Err(PolicyError::Invalid(format!(
                    "caller '{}' has no SHA-256 key pin",
                    caller.name
                )));
            }
            if !keys.insert(caller.spki_sha256.to_lowercase()) {
                return Err(PolicyError::Invalid("two callers share a key".to_string()));
            }
        }
        for name in &self.key_administrators {
            if !names.contains(name) {
                return Err(PolicyError::Invalid(format!(
                    "unknown administrator '{name}'"
                )));
            }
        }
        for rule in &self.keys {
            if rule.purposes.is_empty() || rule.operations.is_empty() {
                return Err(PolicyError::Invalid(format!(
                    "key '{}' allows nothing",
                    rule.key_id
                )));
            }
            for user in &rule.users {
                if !names.contains(user) {
                    return Err(PolicyError::Invalid(format!("unknown user '{user}'")));
                }
            }
            let allowed: &[Operation] = match rule.algorithm {
                Algorithm::Ed25519 | Algorithm::EcdsaP256 | Algorithm::HmacSha256 => {
                    &[Operation::Sign]
                }
                Algorithm::EcdhP256 => &[Operation::Ecdh],
                Algorithm::Aes256Gcm => &[Operation::Encrypt, Operation::Decrypt],
            };
            if rule.operations.iter().any(|op| !allowed.contains(op)) {
                return Err(PolicyError::Invalid(format!(
                    "key '{}' allows an operation its algorithm can't do",
                    rule.key_id
                )));
            }
            match &rule.gate {
                Gate::CompletedExecution { executors } => {
                    if executors.is_empty() || !executors.is_subset(&rule.users) {
                        return Err(PolicyError::Invalid(format!(
                            "key '{}': executors must be users of the key",
                            rule.key_id
                        )));
                    }
                }
                Gate::OfficialApproval { key_id_prefix } => {
                    if self.officials.is_none() {
                        return Err(PolicyError::Invalid(
                            "official keys need an `officials` issuer".to_string(),
                        ));
                    }
                    if rule.key_id.strip_suffix('*') != Some(key_id_prefix.as_str()) {
                        return Err(PolicyError::Invalid(format!(
                            "key '{}' must be the wildcard '{key_id_prefix}*'",
                            rule.key_id
                        )));
                    }
                }
                Gate::None => {}
            }
        }
        Ok(())
    }

    pub fn caller_by_spki(&self, spki_sha256: &str) -> Option<&Caller> {
        self.callers
            .iter()
            .find(|c| c.spki_sha256.eq_ignore_ascii_case(spki_sha256))
    }

    /// The most specific rule for a key: an exact id wins over a wildcard.
    pub fn rule(&self, key_id: &str) -> Option<&KeyRule> {
        self.keys
            .iter()
            .filter(|rule| rule.matches(key_id))
            .max_by_key(|rule| (!rule.key_id.ends_with('*'), rule.key_id.len()))
    }

    pub fn is_administrator(&self, caller: &str) -> bool {
        self.key_administrators.contains(caller)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pin(n: u8) -> String {
        hex::encode([n; 32])
    }

    fn sample() -> String {
        serde_json::json!({
            "callers": [
                {"name": "windmill", "spki_sha256": pin(1)},
                {"name": "admin", "spki_sha256": pin(2)}
            ],
            "key_administrators": ["admin"],
            "keys": [
                {"key_id": "log/*", "algorithm": "ed25519", "operations": ["sign"],
                 "purposes": ["electoral-log-entry"], "users": ["windmill"], "gate": {"type": "none"}},
                {"key_id": "log/system", "algorithm": "ed25519", "operations": ["sign"],
                 "purposes": ["electoral-log-checkpoint"], "users": ["windmill"], "gate": {"type": "none"}},
                {"key_id": "official/*", "algorithm": "ecdsa-p256", "operations": ["sign"],
                 "purposes": ["step-signing-approval"], "users": ["windmill"],
                 "gate": {"type": "official-approval", "key_id_prefix": "official/"}}
            ],
            "officials": {"issuer": "https://kc/realms/t", "jwks_url": "https://kc/certs",
                          "audience": "hsm-service", "max_auth_age_seconds": 60, "acr_values": ["gold"]}
        })
        .to_string()
    }

    #[test]
    fn exact_rules_win_over_wildcards() {
        let policy = Policy::parse(&sample()).unwrap();
        assert!(policy
            .rule("log/system")
            .unwrap()
            .purposes
            .contains("electoral-log-checkpoint"));
        assert!(policy
            .rule("log/tenant-1")
            .unwrap()
            .purposes
            .contains("electoral-log-entry"));
        assert!(policy.rule("log/").is_none());
        assert!(policy.rule("other").is_none());
    }

    #[test]
    fn rejects_unknown_users_and_bad_operations() {
        let mut value: serde_json::Value = serde_json::from_str(&sample()).unwrap();
        value["keys"][0]["users"] = serde_json::json!(["nobody"]);
        assert!(Policy::parse(&value.to_string()).is_err());

        let mut value: serde_json::Value = serde_json::from_str(&sample()).unwrap();
        value["keys"][0]["operations"] = serde_json::json!(["decrypt"]);
        assert!(Policy::parse(&value.to_string()).is_err());
    }

    #[test]
    fn official_keys_need_an_issuer_and_a_wildcard() {
        let mut value: serde_json::Value = serde_json::from_str(&sample()).unwrap();
        value.as_object_mut().unwrap().remove("officials");
        assert!(Policy::parse(&value.to_string()).is_err());

        let mut value: serde_json::Value = serde_json::from_str(&sample()).unwrap();
        value["keys"][2]["key_id"] = serde_json::json!("official/alice");
        assert!(Policy::parse(&value.to_string()).is_err());
    }

    #[test]
    fn executors_must_be_users() {
        let mut value: serde_json::Value = serde_json::from_str(&sample()).unwrap();
        value["keys"][0]["gate"] =
            serde_json::json!({"type": "completed-execution", "executors": ["admin"]});
        assert!(Policy::parse(&value.to_string()).is_err());
    }

    #[test]
    fn callers_need_distinct_pins() {
        let mut value: serde_json::Value = serde_json::from_str(&sample()).unwrap();
        value["callers"][1]["spki_sha256"] = serde_json::json!(pin(1));
        assert!(Policy::parse(&value.to_string()).is_err());
    }
}
