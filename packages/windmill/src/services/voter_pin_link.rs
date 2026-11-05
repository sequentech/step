// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The voter PIN link: a one-time URL where a voter trades their username for
//! their PIN. Keycloak serves it (`voter-pin-link` extension); this side
//! generates the link when a voter is created and when mail is sent.
//!
//! The token is an HMAC-SHA-256 of the PIN, keyed for one voter and truncated
//! to 130 bits of Crockford base32. A plain hash would let anyone who saw a
//! link recover a 12-digit PIN by brute force. Being deterministic, a repeated
//! send keeps the same link. `PinLinkToken.java` must stay in step with this
//! file; `voter-pin-link-v1.json` pins both.

use crate::services::database::get_keycloak_pool;
use crate::services::vault::vault::get_master_secret;
use anyhow::{anyhow, Context, Result};
use chrono::DateTime;
use deadpool_postgres::Transaction;
use ring::{digest, hkdf, hmac};
use sequent_core::types::keycloak::{
    REALM_ATTR_PIN_LINK_ATTEMPTS_ATTRIBUTE, REALM_ATTR_PIN_LINK_EXPIRES_AT,
    REALM_ATTR_PIN_LINK_MAX_ATTEMPTS, REALM_ATTR_PIN_LINK_PIN_ATTRIBUTE,
    REALM_ATTR_PIN_LINK_POLICY, REALM_ATTR_PIN_LINK_PUBLIC_BASE_URL,
    REALM_ATTR_PIN_LINK_TOKEN_HASH_ATTRIBUTE,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use strand::symm::SymmetricKey;
use uuid::Uuid;

/// The template variable with the token alone, for a voter typing it by hand.
pub const PIN_LINK_TOKEN_VARIABLE: &str = "pin_link_token";
/// The template variable with the whole link.
pub const PIN_LINK_URL_VARIABLE: &str = "pin_link_url";

pub const DEFAULT_PIN_ATTRIBUTE: &str = "pin";
pub const DEFAULT_TOKEN_HASH_ATTRIBUTE: &str = "pin-link-token-hash";
pub const DEFAULT_ATTEMPTS_ATTRIBUTE: &str = "pin-link-attempts";

const POLICY_ONE_TIME_LINK: &str = "ONE_TIME_LINK";
const DEFAULT_MAX_ATTEMPTS: u32 = 5;
const MAX_MAX_ATTEMPTS: u32 = 100;
const KEY_DERIVATION_DOMAIN: &[u8] = b"sequent-voter-pin-link-v1";
const TOKEN_LENGTH: usize = 26;
const TOKEN_GROUP: usize = 4;
const CROCKFORD_ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// True for the virtual attributes a template can ask for.
pub fn is_pin_link_variable(name: &str) -> bool {
    name == PIN_LINK_TOKEN_VARIABLE || name == PIN_LINK_URL_VARIABLE
}

/// The per-realm configuration. Anything missing or invalid leaves the feature
/// off, matching what `PinLinkConfig.java` decides.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PinLinkConfig {
    pub pin_attribute: String,
    pub token_hash_attribute: String,
    pub attempts_attribute: String,
    pub public_base_url: Option<String>,
}

fn attribute_name(attributes: &HashMap<String, String>, key: &str, default: &str) -> String {
    attributes
        .get(key)
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .unwrap_or(default)
        .to_string()
}

/// Whether a configured attribute name is usable: 1 to 128 ASCII letters,
/// digits, `.`, `_` or `-`, the grammar `PinLinkConfig.java` also applies.
fn is_valid_attribute_name(name: &str) -> bool {
    (1..=128).contains(&name.len())
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}

/// What a write of a voter's secret attributes does to their PIN.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PinWrite<'a> {
    Absent,
    Cleared,
    Set(&'a str),
}

/// What an edit does to a voter's link: an old link must not outlive its PIN, a
/// new PIN needs a link of its own, and a new password alone leaves a PIN the
/// link would show though it no longer logs in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PinLinkChange {
    /// The edit sets a PIN, which it also sets as the password.
    Rearm,
    Remove,
}

impl PinLinkChange {
    pub fn of_edit(write: PinWrite<'_>, password_changes: bool) -> Option<Self> {
        match write {
            PinWrite::Set(_) => Some(Self::Rearm),
            PinWrite::Cleared => Some(Self::Remove),
            PinWrite::Absent if password_changes => Some(Self::Remove),
            PinWrite::Absent => None,
        }
    }

    /// The PIN for [`rearm_voter`], given the password the edit sets.
    pub fn pin(self, password: Option<&str>) -> Result<Option<&str>> {
        match self {
            Self::Rearm => password
                .map(Some)
                .ok_or_else(|| anyhow!("An edit that sets a PIN must set it as the password too")),
            Self::Remove => Ok(None),
        }
    }
}

impl PinLinkConfig {
    /// The link's configuration from a realm's attributes, or `None` when it is
    /// off: the policy is not `ONE_TIME_LINK`, an attribute name is invalid or
    /// repeated, the attempt limit is not a number from 1 to 100, or the expiry
    /// is missing or invalid. These are the rules of `PinLinkConfig.java`,
    /// expiry included though only Keycloak enforces it, so no link is made
    /// that Keycloak would refuse.
    pub fn from_realm_attributes(attributes: &HashMap<String, String>) -> Option<Self> {
        if attributes.get(REALM_ATTR_PIN_LINK_POLICY)?.as_str() != POLICY_ONE_TIME_LINK {
            return None;
        }
        let (pin_attribute, token_hash_attribute, attempts_attribute) =
            generated_attribute_names(attributes);
        if !is_valid_attribute_name(&pin_attribute)
            || !is_valid_attribute_name(&token_hash_attribute)
            || !is_valid_attribute_name(&attempts_attribute)
            || pin_attribute == token_hash_attribute
            || pin_attribute == attempts_attribute
            || token_hash_attribute == attempts_attribute
        {
            return None;
        }
        let max_attempts = match attributes
            .get(REALM_ATTR_PIN_LINK_MAX_ATTEMPTS)
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
        {
            Some(value) => value.parse::<u32>().ok()?,
            None => DEFAULT_MAX_ATTEMPTS,
        };
        if !(1..=MAX_MAX_ATTEMPTS).contains(&max_attempts) {
            return None;
        }
        // Keycloak's parser needs the `T` that this one would let a space replace.
        let expires_at = attributes.get(REALM_ATTR_PIN_LINK_EXPIRES_AT)?.trim();
        if !expires_at.contains(['T', 't']) {
            return None;
        }
        DateTime::parse_from_rfc3339(expires_at).ok()?;
        Some(Self {
            pin_attribute,
            token_hash_attribute,
            attempts_attribute,
            public_base_url: attributes
                .get(REALM_ATTR_PIN_LINK_PUBLIC_BASE_URL)
                .map(|value| value.trim().trim_end_matches('/').to_string())
                .filter(|value| !value.is_empty()),
        })
    }

    /// What a write of secret attributes does to the PIN. A voter's link shows
    /// one PIN, so several values are refused.
    pub fn pin_write<'a>(
        &self,
        secret_attributes: &'a HashMap<String, Option<Vec<String>>>,
    ) -> Result<PinWrite<'a>> {
        let Some(values) = secret_attributes.get(&self.pin_attribute) else {
            return Ok(PinWrite::Absent);
        };
        match values.as_deref() {
            None | Some([]) => Ok(PinWrite::Cleared),
            Some([value]) if value.is_empty() => Ok(PinWrite::Cleared),
            Some([value]) => Ok(PinWrite::Set(value)),
            Some(_) => Err(anyhow!(
                "`{}` needs exactly one value, because a voter's link shows one PIN",
                self.pin_attribute
            )),
        }
    }

    /// `<public-base-url>/<token>` when the realm has a short host in front of
    /// Keycloak, because a realm name alone is about 86 characters. The token
    /// is grouped so that the link can be read out and typed.
    pub fn link_url(
        &self,
        keycloak_public_url: &str,
        realm_name: &str,
        canonical_token: &str,
    ) -> String {
        let token = display_token(canonical_token);
        match &self.public_base_url {
            Some(base) => format!("{base}/{token}"),
            None => format!(
                "{}/realms/{realm_name}/pin-link/{token}",
                keycloak_public_url.trim_end_matches('/')
            ),
        }
    }
}

fn generated_attribute_names(attributes: &HashMap<String, String>) -> (String, String, String) {
    (
        attribute_name(
            attributes,
            REALM_ATTR_PIN_LINK_PIN_ATTRIBUTE,
            DEFAULT_PIN_ATTRIBUTE,
        ),
        attribute_name(
            attributes,
            REALM_ATTR_PIN_LINK_TOKEN_HASH_ATTRIBUTE,
            DEFAULT_TOKEN_HASH_ATTRIBUTE,
        ),
        attribute_name(
            attributes,
            REALM_ATTR_PIN_LINK_ATTEMPTS_ATTRIBUTE,
            DEFAULT_ATTEMPTS_ATTRIBUTE,
        ),
    )
}

/// The attributes only the link's machinery may read or write. Returned even
/// while the policy is off, so that switching it off doesn't expose them.
pub fn generated_names(attributes: &HashMap<String, String>) -> HashSet<String> {
    let (_, hash, attempts) = generated_attribute_names(attributes);
    HashSet::from([hash, attempts])
}

/// Derives tokens with one copy of the master key, for loops over thousands of
/// voters.
pub struct PinLinkTokens {
    master_secret: SymmetricKey,
}

struct OneBlock;

impl hkdf::KeyType for OneBlock {
    fn len(&self) -> usize {
        32
    }
}

impl PinLinkTokens {
    pub async fn new() -> Result<Self> {
        Ok(Self {
            master_secret: get_master_secret().await?,
        })
    }

    #[cfg(test)]
    pub(crate) fn with_master_secret(master_secret: SymmetricKey) -> Self {
        Self { master_secret }
    }

    pub fn derive(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        user_id: &str,
        pin: &str,
    ) -> Result<String> {
        derive_token(
            self.master_secret.as_slice(),
            tenant_id,
            election_event_id,
            user_id,
            pin,
        )
    }
}

/// Derives a voter's token from their PIN: an HMAC-SHA-256 of it under a key
/// HKDF-derived from the master secret for this tenant, event and voter,
/// truncated and encoded by `encode_token`. `PinLinkToken.derive` does the same
/// in Keycloak.
fn derive_token(
    master_secret: &[u8],
    tenant_id: &str,
    election_event_id: &str,
    user_id: &str,
    pin: &str,
) -> Result<String> {
    let scope = format!("tenant={tenant_id};event={election_event_id};user={user_id}");
    let info = [scope.as_bytes()];
    let mut key = [0_u8; 32];
    hkdf::Salt::new(hkdf::HKDF_SHA256, KEY_DERIVATION_DOMAIN)
        .extract(master_secret)
        .expand(&info, OneBlock)
        .map_err(|_| anyhow!("Failed to derive the PIN link key"))?
        .fill(&mut key)
        .map_err(|_| anyhow!("Failed to materialize the PIN link key"))?;
    let tag = hmac::sign(&hmac::Key::new(hmac::HMAC_SHA256, &key), pin.as_bytes());
    Ok(encode_token(tag.as_ref()))
}

/// The first 130 bits of the tag as 26 Crockford base32 characters, five bits
/// at a time.
fn encode_token(tag: &[u8]) -> String {
    let mut token = String::with_capacity(TOKEN_LENGTH);
    let mut buffer: u32 = 0;
    let mut bits = 0;
    let mut bytes = tag.iter();
    while token.len() < TOKEN_LENGTH {
        if bits < 5 {
            buffer = (buffer << 8) | u32::from(*bytes.next().expect("a SHA-256 tag has 32 bytes"));
            bits += 8;
        }
        bits -= 5;
        token.push(char::from(
            CROCKFORD_ALPHABET[((buffer >> bits) & 0x1f) as usize],
        ));
        buffer &= (1 << bits) - 1;
    }
    token
}

/// The lookup value: the token has 130 bits of entropy, so a plain hash is
/// safe here where it would not be for the PIN.
pub fn token_hash(canonical_token: &str) -> String {
    hex::encode(digest::digest(&digest::SHA256, canonical_token.as_bytes()))
}

/// Groups a token for reading aloud. Keycloak ignores the hyphens.
pub fn display_token(canonical_token: &str) -> String {
    canonical_token
        .as_bytes()
        .chunks(TOKEN_GROUP)
        .map(|chunk| String::from_utf8_lossy(chunk).into_owned())
        .collect::<Vec<_>>()
        .join("-")
}

/// Re-arms or disarms one voter's link after their PIN changed: any previous
/// link and its failed attempts go away, and a non-blank PIN arms a new one.
/// Plain SQL because Keycloak's REST API treats undeclared attributes as
/// read-only, and the same reason keeps voters from reaching them.
pub async fn sync_token_hash(
    transaction: &Transaction<'_>,
    config: &PinLinkConfig,
    user_id: &str,
    new_hash: Option<&str>,
) -> Result<()> {
    transaction
        .execute(
            "DELETE FROM user_attribute WHERE user_id = $1 AND name = ANY($2)",
            &[
                &user_id,
                &vec![
                    config.token_hash_attribute.as_str(),
                    config.attempts_attribute.as_str(),
                ],
            ],
        )
        .await
        .context("Error removing the previous PIN link")?;
    if let Some(hash) = new_hash {
        transaction
            .execute(
                "INSERT INTO user_attribute (id, user_id, name, value) VALUES ($1, $2, $3, $4)",
                &[
                    &Uuid::new_v4().to_string(),
                    &user_id,
                    &config.token_hash_attribute,
                    &hash,
                ],
            )
            .await
            .context("Error storing the PIN link")?;
    }
    Ok(())
}

/// [`sync_token_hash`] for a voter whose PIN was set by an administrator; the
/// new PIN arms its own link and kills the old one in one transaction.
pub async fn rearm_voter(
    config: &PinLinkConfig,
    tenant_id: &str,
    election_event_id: &str,
    user_id: &str,
    pin: Option<&str>,
) -> Result<()> {
    let new_hash = match pin.filter(|pin| !pin.is_empty()) {
        Some(pin) => Some(token_hash(&PinLinkTokens::new().await?.derive(
            tenant_id,
            election_event_id,
            user_id,
            pin,
        )?)),
        None => None,
    };
    let mut client = get_keycloak_pool()
        .await
        .get()
        .await
        .context("Error getting the Keycloak DB client")?;
    let transaction = client
        .transaction()
        .await
        .context("Error starting a Keycloak transaction")?;
    sync_token_hash(&transaction, config, user_id, new_hash.as_deref()).await?;
    transaction
        .commit()
        .await
        .context("Error committing the PIN link")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    const FIXTURE: &str = include_str!(
        "../../../keycloak-extensions/voter-pin-link/src/test/resources/voter-pin-link-v1.json"
    );

    #[test]
    fn an_edit_changes_the_link_when_it_changes_the_pin_or_the_password() {
        use PinLinkChange::{Rearm, Remove};
        let set = PinWrite::Set("123456789012");
        assert_eq!(PinLinkChange::of_edit(set, false), Some(Rearm));
        assert_eq!(PinLinkChange::of_edit(set, true), Some(Rearm));
        assert_eq!(
            PinLinkChange::of_edit(PinWrite::Cleared, false),
            Some(Remove)
        );
        assert_eq!(
            PinLinkChange::of_edit(PinWrite::Cleared, true),
            Some(Remove)
        );
        assert_eq!(PinLinkChange::of_edit(PinWrite::Absent, true), Some(Remove));
        assert_eq!(PinLinkChange::of_edit(PinWrite::Absent, false), None);
    }

    #[test]
    fn a_new_link_needs_the_pin_the_edit_set_as_the_password() {
        use PinLinkChange::{Rearm, Remove};
        assert_eq!(
            Rearm.pin(Some("123456789012")).unwrap(),
            Some("123456789012")
        );
        assert!(Rearm.pin(None).is_err());
        assert_eq!(Remove.pin(Some("123456789012")).unwrap(), None);
        assert_eq!(Remove.pin(None).unwrap(), None);
    }

    fn enabled() -> HashMap<String, String> {
        HashMap::from([
            (
                REALM_ATTR_PIN_LINK_POLICY.to_string(),
                "ONE_TIME_LINK".to_string(),
            ),
            (
                REALM_ATTR_PIN_LINK_EXPIRES_AT.to_string(),
                "2099-01-01T00:00:00Z".to_string(),
            ),
        ])
    }

    /// Windmill generates the link and Keycloak checks it, so both must derive
    /// the same token and lookup hash for the vectors in the shared fixture.
    #[test]
    fn matches_the_vectors_shared_with_keycloak() {
        let fixture: Value = serde_json::from_str(FIXTURE).unwrap();
        let master = hex::decode(fixture["master_secret_hex"].as_str().unwrap()).unwrap();
        for vector in fixture["vectors"].as_array().unwrap() {
            let token = derive_token(
                &master,
                vector["tenant"].as_str().unwrap(),
                vector["event"].as_str().unwrap(),
                vector["user"].as_str().unwrap(),
                vector["pin"].as_str().unwrap(),
            )
            .unwrap();
            assert_eq!(token, vector["token"].as_str().unwrap());
            assert_eq!(token_hash(&token), vector["lookup_hash"].as_str().unwrap());
        }
    }

    #[test]
    fn a_repeated_send_keeps_the_same_link() {
        let master = [7_u8; 32];
        assert_eq!(
            derive_token(&master, "t", "e", "u", "123456789012").unwrap(),
            derive_token(&master, "t", "e", "u", "123456789012").unwrap()
        );
    }

    #[test]
    fn the_token_is_scoped_and_keyed_and_does_not_leak_the_pin() {
        let master = [7_u8; 32];
        let token = derive_token(&master, "t", "e", "u", "123456789012").unwrap();
        assert_eq!(token.len(), TOKEN_LENGTH);
        assert!(TOKEN_LENGTH * 5 >= 128);
        assert_ne!(
            token,
            derive_token(&master, "x", "e", "u", "123456789012").unwrap()
        );
        assert_ne!(
            token,
            derive_token(&master, "t", "x", "u", "123456789012").unwrap()
        );
        assert_ne!(
            token,
            derive_token(&master, "t", "e", "x", "123456789012").unwrap()
        );
        assert_ne!(
            token,
            derive_token(&[8_u8; 32], "t", "e", "u", "123456789012").unwrap()
        );
        assert_ne!(
            token_hash(&token),
            hex::encode(digest::digest(&digest::SHA256, b"123456789012"))
        );
    }

    /// Anything but the exact policy and a valid expiry leaves the link off, as
    /// do a bad attempt limit or attribute name.
    #[test]
    fn is_off_unless_the_policy_and_a_valid_expiry_are_set() {
        assert_eq!(PinLinkConfig::from_realm_attributes(&HashMap::new()), None);
        for (key, value) in [
            (REALM_ATTR_PIN_LINK_POLICY, "DISABLED"),
            (REALM_ATTR_PIN_LINK_POLICY, "one_time_link"),
            (REALM_ATTR_PIN_LINK_EXPIRES_AT, "tomorrow"),
            (REALM_ATTR_PIN_LINK_MAX_ATTEMPTS, "0"),
            (REALM_ATTR_PIN_LINK_MAX_ATTEMPTS, "1000"),
            (REALM_ATTR_PIN_LINK_PIN_ATTRIBUTE, "bad name"),
            (REALM_ATTR_PIN_LINK_PIN_ATTRIBUTE, "pin-link-token-hash"),
        ] {
            let mut attributes = enabled();
            attributes.insert(key.to_string(), value.to_string());
            assert_eq!(
                PinLinkConfig::from_realm_attributes(&attributes),
                None,
                "{key}={value}"
            );
        }
        let mut attributes = enabled();
        attributes.remove(REALM_ATTR_PIN_LINK_EXPIRES_AT);
        assert_eq!(PinLinkConfig::from_realm_attributes(&attributes), None);
    }

    /// Windmill and Keycloak accept and refuse the same expiry values, which
    /// the shared fixture lists. If they differed, voters could be mailed links
    /// that never work.
    #[test]
    fn reads_the_expiry_like_keycloak() {
        let fixture: Value = serde_json::from_str(FIXTURE).unwrap();
        let read = |value: &Value| {
            let mut attributes = enabled();
            attributes.insert(
                REALM_ATTR_PIN_LINK_EXPIRES_AT.to_string(),
                value.as_str().unwrap().to_string(),
            );
            PinLinkConfig::from_realm_attributes(&attributes)
        };
        for value in fixture["expires_at"]["valid"].as_array().unwrap() {
            assert!(read(value).is_some(), "{value}");
        }
        for value in fixture["expires_at"]["invalid"].as_array().unwrap() {
            assert!(read(value).is_none(), "{value}");
        }
    }

    /// A write that doesn't mention the PIN is `Absent`, one that removes it or
    /// leaves it blank is `Cleared`, one with a single value is `Set`, and
    /// several values are an error.
    #[test]
    fn tells_what_a_write_does_to_the_pin() {
        let config = PinLinkConfig::from_realm_attributes(&enabled()).unwrap();
        let write = |values: Option<Vec<&str>>| {
            HashMap::from([(
                "pin".to_string(),
                values.map(|values| values.iter().map(|value| value.to_string()).collect()),
            )])
        };
        assert_eq!(config.pin_write(&HashMap::new()).unwrap(), PinWrite::Absent);
        assert_eq!(
            config
                .pin_write(&HashMap::from([("other".to_string(), None)]))
                .unwrap(),
            PinWrite::Absent
        );
        for cleared in [write(None), write(Some(vec![])), write(Some(vec![""]))] {
            assert_eq!(config.pin_write(&cleared).unwrap(), PinWrite::Cleared);
        }
        let one = write(Some(vec!["123456789012"]));
        assert_eq!(
            config.pin_write(&one).unwrap(),
            PinWrite::Set("123456789012")
        );
        assert!(config.pin_write(&write(Some(vec!["1", "2"]))).is_err());
    }

    #[test]
    fn uses_the_defaults_and_keeps_the_generated_names_hidden_when_off() {
        let config = PinLinkConfig::from_realm_attributes(&enabled()).unwrap();
        assert_eq!(config.pin_attribute, "pin");
        assert_eq!(config.token_hash_attribute, "pin-link-token-hash");
        assert_eq!(config.attempts_attribute, "pin-link-attempts");
        assert_eq!(
            generated_names(&HashMap::new()),
            HashSet::from([
                "pin-link-token-hash".to_string(),
                "pin-link-attempts".to_string()
            ])
        );
    }

    #[test]
    fn builds_a_short_link_from_the_public_base_url() {
        let mut attributes = enabled();
        let token = "4G89H1GVW3JYW0PR9NGBAHCABP";
        let config = PinLinkConfig::from_realm_attributes(&attributes).unwrap();
        assert_eq!(
            config.link_url("https://login.example/", "tenant-a-event-b", token),
            "https://login.example/realms/tenant-a-event-b/pin-link/4G89-H1GV-W3JY-W0PR-9NGB-AHCA-BP"
        );
        attributes.insert(
            REALM_ATTR_PIN_LINK_PUBLIC_BASE_URL.to_string(),
            "https://pin.example/".to_string(),
        );
        let config = PinLinkConfig::from_realm_attributes(&attributes).unwrap();
        assert_eq!(
            config.link_url("https://login.example", "tenant-a-event-b", token),
            "https://pin.example/4G89-H1GV-W3JY-W0PR-9NGB-AHCA-BP"
        );
    }
}
