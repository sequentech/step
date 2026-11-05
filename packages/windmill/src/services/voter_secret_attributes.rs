// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::vault::vault::get_master_secret;
use crate::services::voter_pin_link::{self, PinLinkConfig};
use anyhow::{anyhow, Context, Result};
use base64::prelude::{Engine as _, BASE64_URL_SAFE_NO_PAD};
use keycloak::types::UPConfig;
use once_cell::sync::Lazy;
use ring::hkdf;
use sequent_core::services::keycloak::{get_event_realm, KeycloakAdminClient};
use sequent_core::types::keycloak::{User, UserProfileAttribute};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};
use strand::serialization::{StrandDeserialize, StrandSerialize};
use strand::symm::{decrypt, encrypt, EncryptionData, SymmetricKey};
use tokio::sync::RwLock;

pub const SECRET_ATTRIBUTE_ANNOTATION: &str = "sequent.secret";
pub const ENCRYPTED_VALUE_PREFIX: &str = "seqenc:v1:";
pub const REDACTED_SECRET_VALUE: &str = "<redacted>";
/// Keycloak keeps attribute values in `user_attribute.value`, a 255-character
/// column, and moves longer values to `long_value`. The bulk voter import
/// writes that table directly and the user listing queries read only `value`,
/// so an envelope must fit the column to be usable everywhere.
pub const MAX_ENCRYPTED_VALUE_CHARS: usize = 255;
/// Largest plaintext whose `seqenc:v1:` envelope fits
/// [`MAX_ENCRYPTED_VALUE_CHARS`]: 10 prefix characters plus the unpadded
/// base64 of the serialized ciphertext (4-byte length, plaintext, 12-byte
/// nonce, 16-byte tag).
pub const MAX_SECRET_VALUE_BYTES: usize = 150;

const KEY_DERIVATION_DOMAIN: &[u8] = b"sequent-voter-secret-attribute-v1";
/// How long a realm's secret-attribute configuration is reused before the
/// Keycloak user profile is read again. Voter list and detail requests are
/// polled by the Admin Portal, so they must not hit the Keycloak admin API
/// on every call.
const CONFIG_CACHE_TTL: Duration = Duration::from_secs(30);
const CIPHERTEXT_COMPATIBLE_VALIDATORS: [&str; 1] = ["person-name-prohibited-characters"];
const VOTER_ROLE: &str = "user";
/// Identity and operational fields that other components read in plaintext.
/// The first and last name are included: they live in Keycloak's top-level
/// user fields, which every voter-level output copies verbatim.
const FORBIDDEN_SECRET_ATTRIBUTES: [&str; 17] = [
    "area-id",
    "authorized-election-ids",
    "authorized-to-election-alias",
    "dateOfBirth",
    "disable-comment",
    "email",
    "firstName",
    "first_name",
    "lastName",
    "last_name",
    "permission_labels",
    "sequent.read-only.id-card-number-validated",
    "sequent.read-only.mobile-number",
    "tenant-id",
    "username",
    "vote-weight",
    "voted-channel",
];

static CONFIG_CACHE: Lazy<RwLock<HashMap<String, (Instant, SecretAttributeConfig)>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VoterAttributeStoragePolicy {
    Plaintext,
    Encrypted,
}

#[derive(Clone, Debug)]
pub struct VoterSecretAttributeScope<'a> {
    pub tenant_id: &'a str,
    pub election_event_id: &'a str,
    pub user_id: &'a str,
    pub attribute_name: &'a str,
}

struct VoterSecretKeyLength;

impl hkdf::KeyType for VoterSecretKeyLength {
    fn len(&self) -> usize {
        32
    }
}

pub fn storage_policy(attribute: &UserProfileAttribute) -> VoterAttributeStoragePolicy {
    let encrypted = attribute
        .annotations
        .as_ref()
        .and_then(|annotations| annotations.get(SECRET_ATTRIBUTE_ANNOTATION))
        .is_some_and(|value| match value {
            Value::Bool(value) => *value,
            Value::String(value) => value.eq_ignore_ascii_case("true"),
            _ => false,
        });

    if encrypted {
        VoterAttributeStoragePolicy::Encrypted
    } else {
        VoterAttributeStoragePolicy::Plaintext
    }
}

/// The secret-attribute configuration of one election-event realm.
///
/// Read paths must redact every attribute annotated as secret even when the
/// profile is misconfigured, otherwise a configuration mistake would turn the
/// voter list into an error page. Paths that store, reveal or decrypt values
/// refuse to work on a misconfigured profile instead.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SecretAttributeConfig {
    names: HashSet<String>,
    error: Option<String>,
    /// `names` plus the PIN link's generated attributes.
    redacted: HashSet<String>,
    generated: HashSet<String>,
    pin_link: Option<PinLinkConfig>,
}

impl SecretAttributeConfig {
    /// The attributes a user profile annotates as encrypted. A misconfigured
    /// one is kept as the config's first error, not returned, so read paths can
    /// still redact the names.
    pub fn from_profile(attributes: &[UserProfileAttribute]) -> Self {
        let mut names = HashSet::new();
        let mut error = None;
        for attribute in attributes
            .iter()
            .filter(|attribute| storage_policy(attribute) == VoterAttributeStoragePolicy::Encrypted)
        {
            let Some(name) = attribute.name.clone() else {
                error.get_or_insert_with(|| {
                    "An encrypted user-profile attribute has no name".to_string()
                });
                continue;
            };
            if FORBIDDEN_SECRET_ATTRIBUTES.contains(&name.as_str()) {
                error.get_or_insert_with(|| {
                    format!("User-profile attribute `{name}` cannot be configured as encrypted")
                });
            } else if attribute.validations.as_ref().is_some_and(|validations| {
                validations
                    .keys()
                    .any(|name| !CIPHERTEXT_COMPATIBLE_VALIDATORS.contains(&name.as_str()))
            }) {
                error.get_or_insert_with(|| {
                    format!(
                        "Encrypted user-profile attribute `{name}` cannot use Keycloak value validators"
                    )
                });
            }
            names.insert(name);
        }
        Self {
            redacted: names.clone(),
            names,
            error,
            generated: HashSet::new(),
            pin_link: None,
        }
    }

    /// Adds what the realm attributes say about the voter PIN link. A PIN that
    /// isn't a declared secret is a configuration problem, reported like any
    /// other one to the paths that store or decrypt values.
    pub fn with_realm_attributes(mut self, attributes: &HashMap<String, String>) -> Self {
        self.generated = voter_pin_link::generated_names(attributes);
        self.redacted.extend(self.generated.iter().cloned());
        self.pin_link = PinLinkConfig::from_realm_attributes(attributes);
        if let Some(config) = &self.pin_link {
            if !self.names.contains(&config.pin_attribute) {
                self.error.get_or_insert_with(|| {
                    format!(
                        "The PIN link's attribute `{}` is not configured as encrypted",
                        config.pin_attribute
                    )
                });
                self.pin_link = None;
            }
        }
        self
    }

    /// Turns the link off when Keycloak would answer it with a 404, so that
    /// importing or sending fails rather than mailing dead links. Takes the
    /// whole profile, because `from_profile` gets only what the portals show.
    pub fn with_pin_link_gate(mut self, profile: &UPConfig) -> Self {
        if let Some(config) = &self.pin_link {
            if let Some(problem) =
                pin_link_gate_problem(profile, &config.pin_attribute, &self.generated)
            {
                self.error.get_or_insert(problem);
                self.pin_link = None;
            }
        }
        self
    }

    /// The PIN link's configuration, when the realm has it on.
    pub fn pin_link(&self) -> Option<&PinLinkConfig> {
        self.pin_link.as_ref()
    }

    /// Attributes only the PIN link may read or write; no template, file or
    /// administrator supplies them.
    pub fn generated_names(&self) -> &HashSet<String> {
        &self.generated
    }

    /// Every attribute annotated as secret, for redaction and column filtering.
    pub fn redacted_names(&self) -> &HashSet<String> {
        &self.redacted
    }

    /// The secret attributes, or the first configuration problem found.
    pub fn validated_names(&self) -> Result<HashSet<String>> {
        match &self.error {
            Some(error) => Err(anyhow!("{error}")),
            None => Ok(self.names.clone()),
        }
    }
}

impl SecretAttributeConfig {
    /// The names a voter report keeps out of its variables unless declared.
    /// A report can only declare encrypted attributes: the PIN link's generated
    /// ones aren't envelopes, so decrypting them would fail for every voter
    /// with a link.
    pub fn report_names(&self, declared_names: &HashSet<String>) -> Result<HashSet<String>> {
        let mut names = if declared_names.is_empty() {
            self.redacted.clone()
        } else {
            self.validated_names()?
        };
        if let Some(name) = declared_names.iter().find(|name| !names.contains(*name)) {
            return Err(anyhow!(
                "Report declares `{name}`, which is not configured as an encrypted voter attribute"
            ));
        }
        names.extend(self.generated.iter().cloned());
        Ok(names)
    }
}

/// What makes Keycloak keep the PIN link off, other than the PIN not being a
/// secret. The same conditions as `pinIsSecretAndAdminOnly` in the Keycloak
/// extension, but for the unmanaged attributes policy: a realm that sets it
/// can't be read here at all, so the job has already failed.
fn pin_link_gate_problem(
    profile: &UPConfig,
    pin: &str,
    generated: &HashSet<String>,
) -> Option<String> {
    let attributes = profile.attributes.as_deref().unwrap_or_default();
    let voters_reach_pin = attributes
        .iter()
        .filter(|attribute| attribute.name.as_deref() == Some(pin))
        .filter_map(|attribute| attribute.permissions.as_ref())
        .any(|permissions| {
            [&permissions.view, &permissions.edit]
                .into_iter()
                .flatten()
                .flatten()
                .any(|role| role == VOTER_ROLE)
        });
    if voters_reach_pin {
        return Some(format!(
            "Voters must not be able to view or edit the PIN link's attribute `{pin}`"
        ));
    }
    if let Some(name) = attributes
        .iter()
        .filter_map(|attribute| attribute.name.as_deref())
        .find(|name| generated.contains(*name))
    {
        return Some(format!(
            "The PIN link's generated attribute `{name}` must stay out of the user profile"
        ));
    }
    None
}

pub fn secret_attribute_names(attributes: &[UserProfileAttribute]) -> Result<HashSet<String>> {
    SecretAttributeConfig::from_profile(attributes).validated_names()
}

/// Reads the election-event realm's secret-attribute configuration, reusing a
/// recent copy for [`CONFIG_CACHE_TTL`]. A profile change therefore takes up
/// to that long to be observed by the read and redaction paths.
pub async fn get_secret_attribute_config(
    tenant_id: &str,
    election_event_id: &str,
) -> Result<SecretAttributeConfig> {
    let realm = get_event_realm(tenant_id, election_event_id);
    if let Some((cached_at, config)) = CONFIG_CACHE.read().await.get(&realm) {
        if cached_at.elapsed() < CONFIG_CACHE_TTL {
            return Ok(config.clone());
        }
    }
    let client = KeycloakAdminClient::new()
        .await
        .context("Error connecting to Keycloak")?;
    let profile = client
        .get_user_profile(&realm)
        .await
        .context("Error reading the Keycloak user profile")?;
    let attributes =
        KeycloakAdminClient::get_formatted_user_profile_configuration(profile.clone()).attributes;
    let realm_attributes = client
        .get_realm_attributes(&realm)
        .await
        .context("Error reading the Keycloak realm attributes")?;
    let config = SecretAttributeConfig::from_profile(&attributes)
        .with_realm_attributes(&realm_attributes)
        .with_pin_link_gate(&profile);
    CONFIG_CACHE
        .write()
        .await
        .insert(realm, (Instant::now(), config.clone()));
    Ok(config)
}

/// Drops the cached configuration of one realm, for callers that just changed
/// the user profile.
pub async fn invalidate_secret_attribute_config(tenant_id: &str, election_event_id: &str) {
    CONFIG_CACHE
        .write()
        .await
        .remove(&get_event_realm(tenant_id, election_event_id));
}

fn derive_key(
    master_secret: &SymmetricKey,
    scope: &VoterSecretAttributeScope<'_>,
) -> Result<SymmetricKey> {
    let salt = hkdf::Salt::new(hkdf::HKDF_SHA256, KEY_DERIVATION_DOMAIN);
    let pseudorandom_key = salt.extract(master_secret.as_slice());
    let scope_info = format!(
        "tenant={};event={};user={};attribute={}",
        scope.tenant_id, scope.election_event_id, scope.user_id, scope.attribute_name
    );
    let info = [scope_info.as_bytes()];
    let output_key_material = pseudorandom_key
        .expand(&info, VoterSecretKeyLength)
        .map_err(|_| anyhow!("Failed to derive voter secret-attribute key"))?;
    let mut key_bytes = [0_u8; 32];
    output_key_material
        .fill(&mut key_bytes)
        .map_err(|_| anyhow!("Failed to materialize voter secret-attribute key"))?;
    Ok(SymmetricKey::from_slice(&key_bytes).to_owned())
}

/// Encrypts a value into a `seqenc:v1:` envelope bound to the tenant, event,
/// voter and attribute in `scope`. Refuses a value, or an envelope, too long
/// for the column Keycloak stores attribute values in.
pub(crate) fn encrypt_with_master_secret(
    master_secret: &SymmetricKey,
    scope: &VoterSecretAttributeScope<'_>,
    value: &str,
) -> Result<String> {
    if value.len() > MAX_SECRET_VALUE_BYTES {
        return Err(anyhow!(
            "Voter secret attribute `{}` exceeds the {MAX_SECRET_VALUE_BYTES}-byte plaintext limit",
            scope.attribute_name
        ));
    }
    let key = derive_key(master_secret, scope)?;
    let encrypted =
        encrypt(key, value.as_bytes()).context("Failed to encrypt voter secret attribute")?;
    let serialized = encrypted
        .strand_serialize()
        .context("Failed to serialize voter secret attribute")?;
    let envelope = format!(
        "{ENCRYPTED_VALUE_PREFIX}{}",
        BASE64_URL_SAFE_NO_PAD.encode(serialized)
    );
    if envelope.len() > MAX_ENCRYPTED_VALUE_CHARS {
        return Err(anyhow!(
            "Voter secret attribute `{}` does not fit the {MAX_ENCRYPTED_VALUE_CHARS}-character Keycloak attribute column",
            scope.attribute_name
        ));
    }
    Ok(envelope)
}

fn decrypt_with_master_secret(
    master_secret: &SymmetricKey,
    scope: &VoterSecretAttributeScope<'_>,
    value: &str,
) -> Result<String> {
    let encoded = value.strip_prefix(ENCRYPTED_VALUE_PREFIX).ok_or_else(|| {
        anyhow!(
            "Encrypted voter attribute `{}` has an invalid envelope",
            scope.attribute_name
        )
    })?;
    let serialized = BASE64_URL_SAFE_NO_PAD
        .decode(encoded)
        .context("Failed to decode voter secret attribute")?;
    let encrypted = EncryptionData::strand_deserialize(&serialized)
        .context("Failed to deserialize voter secret attribute")?;
    let key = derive_key(master_secret, scope)?;
    let plaintext =
        decrypt(&key, &encrypted).context("Failed to decrypt voter secret attribute")?;
    String::from_utf8(plaintext).context("Voter secret attribute is not valid UTF-8")
}

pub async fn encrypt_attribute_values(
    tenant_id: &str,
    election_event_id: &str,
    user_id: &str,
    attribute_name: &str,
    values: &[String],
) -> Result<Vec<String>> {
    let master_secret = get_master_secret().await?;
    let scope = VoterSecretAttributeScope {
        tenant_id,
        election_event_id,
        user_id,
        attribute_name,
    };
    values
        .iter()
        .map(|value| encrypt_with_master_secret(&master_secret, &scope, value))
        .collect()
}

pub async fn decrypt_attribute_values(
    tenant_id: &str,
    election_event_id: &str,
    user_id: &str,
    attribute_name: &str,
    values: &[String],
) -> Result<Vec<String>> {
    VoterSecretAttributeDecryptor::new()
        .await?
        .decrypt_attribute_values(
            tenant_id,
            election_event_id,
            user_id,
            attribute_name,
            values,
        )
}

/// Reuses one master-secret lookup across a batch of voter decryptions.
///
/// A voter export can contain many voters and several secret fields per voter.
/// Keeping the key in this short-lived service avoids re-entering the vault
/// accessor for every value while still scoping the key to the export task.
pub struct VoterSecretAttributeDecryptor {
    master_secret: SymmetricKey,
}

impl VoterSecretAttributeDecryptor {
    pub async fn new() -> Result<Self> {
        Ok(Self {
            master_secret: get_master_secret().await?,
        })
    }

    #[cfg(test)]
    pub(crate) fn with_master_secret(master_secret: SymmetricKey) -> Self {
        Self { master_secret }
    }

    pub fn decrypt_attribute_values(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        user_id: &str,
        attribute_name: &str,
        values: &[String],
    ) -> Result<Vec<String>> {
        let scope = VoterSecretAttributeScope {
            tenant_id,
            election_event_id,
            user_id,
            attribute_name,
        };
        values
            .iter()
            .map(|value| decrypt_with_master_secret(&self.master_secret, &scope, value))
            .collect()
    }

    pub fn decrypt_user_attributes(
        &self,
        user: &mut User,
        tenant_id: &str,
        election_event_id: &str,
        attribute_names: &HashSet<String>,
    ) -> Result<()> {
        let user_id = user
            .id
            .clone()
            .ok_or_else(|| anyhow!("Cannot decrypt secret attributes for a user without an id"))?;
        for name in attribute_names {
            let values = user_attribute_values(user, name);
            if values.is_empty() {
                continue;
            }
            let decrypted = self.decrypt_attribute_values(
                tenant_id,
                election_event_id,
                &user_id,
                name,
                &values,
            )?;
            set_user_attribute_values(user, name, decrypted)?;
        }
        Ok(())
    }
}

pub fn user_attribute_values(user: &User, name: &str) -> Vec<String> {
    user.attributes
        .as_ref()
        .and_then(|attributes| attributes.get(name))
        .cloned()
        .unwrap_or_default()
}

fn set_user_attribute_values(user: &mut User, name: &str, values: Vec<String>) -> Result<()> {
    user.attributes
        .get_or_insert_with(HashMap::new)
        .insert(name.to_string(), values);
    Ok(())
}

pub fn redact_user(user: &mut User, secret_names: &HashSet<String>) {
    for name in secret_names {
        if let Some(values) = user
            .attributes
            .as_mut()
            .and_then(|attributes| attributes.get_mut(name))
        {
            if !values.is_empty() {
                *values = vec![REDACTED_SECRET_VALUE.to_string()];
            }
        }
    }
}

/// Removes every configured secret attribute that a voter-level output did
/// not declare, so neither ciphertext nor plaintext of an undeclared secret
/// can reach a rendered template.
pub fn strip_undeclared_secret_attributes(
    user: &mut User,
    configured_names: &HashSet<String>,
    declared_names: &HashSet<String>,
) {
    if let Some(attributes) = user.attributes.as_mut() {
        attributes
            .retain(|name, _| !configured_names.contains(name) || declared_names.contains(name));
    }
}

pub async fn decrypt_user_attributes(
    user: &mut User,
    tenant_id: &str,
    election_event_id: &str,
    attribute_names: &HashSet<String>,
) -> Result<()> {
    VoterSecretAttributeDecryptor::new()
        .await?
        .decrypt_user_attributes(user, tenant_id, election_event_id, attribute_names)
}

pub async fn encrypt_secret_attribute_map(
    tenant_id: &str,
    election_event_id: &str,
    user_id: &str,
    secret_names: &HashSet<String>,
    values: HashMap<String, Option<Vec<String>>>,
) -> Result<HashMap<String, Vec<String>>> {
    let master_secret = get_master_secret().await?;
    encrypt_secret_attribute_map_with_key(
        &master_secret,
        tenant_id,
        election_event_id,
        user_id,
        secret_names,
        values,
    )
}

fn encrypt_secret_attribute_map_with_key(
    master_secret: &SymmetricKey,
    tenant_id: &str,
    election_event_id: &str,
    user_id: &str,
    secret_names: &HashSet<String>,
    values: HashMap<String, Option<Vec<String>>>,
) -> Result<HashMap<String, Vec<String>>> {
    let mut encrypted = HashMap::new();
    for (name, values) in values {
        if !secret_names.contains(&name) {
            return Err(anyhow!(
                "User-profile attribute `{name}` is not configured as encrypted"
            ));
        }
        let values = values.unwrap_or_default();
        encrypted.insert(
            name.clone(),
            values
                .iter()
                // An encrypted empty string would incorrectly satisfy Keycloak's
                // required-field check. Preserve its blank-as-missing semantics.
                .filter(|value| !value.trim().is_empty())
                .map(|value| {
                    encrypt_with_master_secret(
                        master_secret,
                        &VoterSecretAttributeScope {
                            tenant_id,
                            election_event_id,
                            user_id,
                            attribute_name: &name,
                        },
                        value,
                    )
                })
                .collect::<Result<Vec<_>>>()?,
        );
    }
    Ok(encrypted)
}

#[cfg(test)]
mod tests {
    use super::*;
    use keycloak::types::{UPAttribute, UPAttributePermissions};
    use sequent_core::types::keycloak::{
        REALM_ATTR_PIN_LINK_EXPIRES_AT, REALM_ATTR_PIN_LINK_POLICY,
    };
    use strand::symm::gen_key;

    fn pin_link_realm_attributes() -> HashMap<String, String> {
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

    /// A report can declare the PIN but not the generated attributes, whose
    /// names stay out of its variables either way.
    #[test]
    fn a_report_cannot_declare_the_generated_attributes() {
        let config = SecretAttributeConfig::from_profile(&[secret_attribute("pin")])
            .with_realm_attributes(&pin_link_realm_attributes());
        let generated = config.generated_names().clone();
        assert!(!generated.is_empty());
        for declared in [HashSet::new(), HashSet::from(["pin".to_string()])] {
            let names = config.report_names(&declared).unwrap();
            assert!(names.contains("pin"));
            assert!(generated.is_subset(&names));
        }
        for name in &generated {
            let error = config
                .report_names(&HashSet::from([name.clone()]))
                .unwrap_err();
            assert!(error.to_string().contains(name.as_str()));
        }
        assert!(config
            .report_names(&HashSet::from(["unknown".to_string()]))
            .is_err());
    }

    /// The link's generated attributes are redacted on read paths like secrets,
    /// but are not among the names a write may supply, so no request can set
    /// them.
    #[test]
    fn generated_attributes_are_redacted_but_never_offered_as_secrets() {
        let config = SecretAttributeConfig::from_profile(&[secret_attribute("pin")])
            .with_realm_attributes(&pin_link_realm_attributes());
        assert!(config.pin_link().is_some());
        let names = config.validated_names().unwrap();
        for generated in ["pin-link-token-hash", "pin-link-attempts"] {
            assert!(config.generated_names().contains(generated));
            assert!(config.redacted_names().contains(generated));
            assert!(!names.contains(generated));
        }
    }

    #[test]
    fn generated_attributes_stay_redacted_with_the_link_off() {
        let config = SecretAttributeConfig::from_profile(&[secret_attribute("pin")])
            .with_realm_attributes(&HashMap::new());
        assert!(config.pin_link().is_none());
        assert!(config.validated_names().is_ok());
        assert!(config.redacted_names().contains("pin-link-token-hash"));
    }

    #[test]
    fn a_pin_that_is_not_a_declared_secret_turns_the_link_off_and_reports_it() {
        let config = SecretAttributeConfig::from_profile(&[secret_attribute("reference")])
            .with_realm_attributes(&pin_link_realm_attributes());
        assert!(config.pin_link().is_none());
        let error = config.validated_names().unwrap_err();
        assert!(error.to_string().contains("`pin`"), "{error}");
    }

    fn profile_attribute(name: &str, view: &[&str], edit: &[&str]) -> UPAttribute {
        let roles = |roles: &[&str]| Some(roles.iter().map(|role| role.to_string()).collect());
        UPAttribute {
            name: Some(name.to_string()),
            permissions: Some(UPAttributePermissions {
                view: roles(view),
                edit: roles(edit),
            }),
            ..Default::default()
        }
    }

    fn gated_by(attributes: Vec<UPAttribute>) -> SecretAttributeConfig {
        SecretAttributeConfig::from_profile(&[secret_attribute("pin")])
            .with_realm_attributes(&pin_link_realm_attributes())
            .with_pin_link_gate(&UPConfig {
                attributes: Some(attributes),
                ..Default::default()
            })
    }

    /// Keycloak lets a role view what it can edit, so either one is enough.
    #[test]
    fn a_pin_voters_can_view_or_edit_turns_the_link_off_and_reports_it() {
        for pin in [
            profile_attribute("pin", &["admin", "user"], &["admin"]),
            profile_attribute("pin", &["admin"], &["admin", "user"]),
        ] {
            let config = gated_by(vec![pin]);
            assert!(config.pin_link().is_none());
            let error = config.validated_names().unwrap_err();
            assert!(error.to_string().contains("`pin`"), "{error}");
        }
    }

    #[test]
    fn a_pin_only_administrators_can_reach_keeps_the_link_on() {
        let config = gated_by(vec![profile_attribute("pin", &["admin"], &["admin"])]);
        assert!(config.pin_link().is_some());
        assert!(config.validated_names().is_ok());
    }

    /// With no permissions this one is not among the attributes the portals
    /// get, and Keycloak still sees it.
    #[test]
    fn a_declared_generated_attribute_turns_the_link_off_and_reports_it() {
        let config = gated_by(vec![UPAttribute {
            name: Some("pin-link-attempts".to_string()),
            ..Default::default()
        }]);
        assert!(config.pin_link().is_none());
        let error = config.validated_names().unwrap_err();
        assert!(error.to_string().contains("`pin-link-attempts`"), "{error}");
    }

    #[test]
    fn voter_permissions_on_another_attribute_leave_the_link_on() {
        let config = gated_by(vec![
            profile_attribute("pin", &["admin"], &["admin"]),
            profile_attribute("reference", &["user"], &["user"]),
        ]);
        assert!(config.pin_link().is_some());
    }

    /// This failure is what stops a job in such a realm, where under `ENABLED`
    /// Keycloak keeps the link off. If an update of the crate lets these
    /// through, `pin_link_gate_problem` has to check the policy.
    #[test]
    fn a_profile_that_sets_the_unmanaged_attribute_policy_cannot_be_read() {
        for policy in ["ENABLED", "ADMIN_VIEW", "ADMIN_EDIT"] {
            let profile = format!(r#"{{"unmanagedAttributePolicy":"{policy}"}}"#);
            assert!(serde_json::from_str::<UPConfig>(&profile).is_err());
        }
        assert!(serde_json::from_str::<UPConfig>("{}").is_ok());
    }

    #[test]
    fn the_gate_leaves_a_realm_with_the_link_off_alone() {
        let config = SecretAttributeConfig::from_profile(&[secret_attribute("pin")])
            .with_realm_attributes(&HashMap::new())
            .with_pin_link_gate(&UPConfig {
                attributes: Some(vec![profile_attribute("pin", &["user"], &["user"])]),
                ..Default::default()
            });
        assert!(config.validated_names().is_ok());
    }

    #[test]
    fn creation_ciphertext_preserves_missing_values_and_is_bound_to_user_id() {
        let key = gen_key();
        let names = HashSet::from(["mobile-number".to_string()]);
        for values in [None, Some(vec![]), Some(vec!["".into(), "  ".into()])] {
            let encrypted = encrypt_secret_attribute_map_with_key(
                &key,
                "tenant",
                "event",
                "provisional",
                &names,
                HashMap::from([("mobile-number".into(), values)]),
            )
            .unwrap();
            assert!(
                encrypted["mobile-number"].is_empty(),
                "Missing plaintext must stay missing for required-field validation"
            );
        }
        let plaintext = HashMap::from([("mobile-number".into(), Some(vec!["test-secret".into()]))]);
        let provisional = encrypt_secret_attribute_map_with_key(
            &key,
            "tenant",
            "event",
            "provisional",
            &names,
            plaintext.clone(),
        )
        .unwrap();
        let final_values = encrypt_secret_attribute_map_with_key(
            &key,
            "tenant",
            "event",
            "created-voter",
            &names,
            plaintext,
        )
        .unwrap();
        assert!(provisional["mobile-number"][0].starts_with(ENCRYPTED_VALUE_PREFIX));
        assert!(decrypt_with_master_secret(
            &key,
            &scope("created-voter"),
            &provisional["mobile-number"][0]
        )
        .is_err());
        assert_eq!(
            decrypt_with_master_secret(
                &key,
                &scope("created-voter"),
                &final_values["mobile-number"][0]
            )
            .unwrap(),
            "test-secret"
        );
    }

    #[test]
    fn keycloak_v1_compatibility_fixture() {
        // Shared with Java; fixed public test key and deterministic nonce, never production data.
        let v: serde_json::Value = serde_json::from_str(include_str!(
            "../../../keycloak-extensions/message-otp-authenticator/src/test/resources/voter-secret-v1.json"
        )).unwrap();
        let key_bytes = hex::decode(v["master"].as_str().unwrap()).unwrap();
        let key = SymmetricKey::clone_from_slice(&key_bytes);
        let scope = VoterSecretAttributeScope {
            tenant_id: v["tenant"].as_str().unwrap(),
            election_event_id: v["event"].as_str().unwrap(),
            user_id: v["user"].as_str().unwrap(),
            attribute_name: v["attribute"].as_str().unwrap(),
        };
        assert_eq!(
            decrypt_with_master_secret(&key, &scope, v["envelope"].as_str().unwrap()).unwrap(),
            v["plaintext"].as_str().unwrap()
        );
    }

    #[test]
    fn exported_multi_values_can_be_reencrypted_for_an_import_destination() {
        let master = gen_key();
        let values = vec!["first-secret".to_string(), "second-secret".to_string()];
        let encrypted = values
            .iter()
            .map(|value| {
                encrypt_with_master_secret(&master, &scope("original-voter"), value).unwrap()
            })
            .collect::<Vec<_>>();
        let decryptor = VoterSecretAttributeDecryptor {
            master_secret: master,
        };
        let mut user = User {
            id: Some("original-voter".into()),
            attributes: Some(HashMap::from([("mobile-number".into(), encrypted)])),
            ..Default::default()
        };
        decryptor
            .decrypt_user_attributes(
                &mut user,
                "tenant",
                "event",
                &HashSet::from(["mobile-number".into()]),
            )
            .unwrap();
        assert_eq!(user.attributes.as_ref().unwrap()["mobile-number"], values);
        for value in &user.attributes.unwrap()["mobile-number"] {
            let destination = scope("imported-voter");
            let envelope =
                encrypt_with_master_secret(&decryptor.master_secret, &destination, value).unwrap();
            assert_eq!(
                decrypt_with_master_secret(&decryptor.master_secret, &destination, &envelope)
                    .unwrap(),
                *value
            );
            assert!(decrypt_with_master_secret(
                &decryptor.master_secret,
                &scope("original-voter"),
                &envelope
            )
            .is_err());
        }
    }

    fn scope<'a>(user_id: &'a str) -> VoterSecretAttributeScope<'a> {
        VoterSecretAttributeScope {
            tenant_id: "tenant",
            election_event_id: "event",
            user_id,
            attribute_name: "mobile-number",
        }
    }

    #[test]
    fn encrypted_values_round_trip_and_are_randomized() {
        let master = gen_key();
        let first = encrypt_with_master_secret(&master, &scope("user-1"), "+15555550100")
            .expect("first encryption succeeds");
        let second = encrypt_with_master_secret(&master, &scope("user-1"), "+15555550100")
            .expect("second encryption succeeds");
        assert_ne!(first, second);
        assert_eq!(
            decrypt_with_master_secret(&master, &scope("user-1"), &first)
                .expect("decryption succeeds"),
            "+15555550100"
        );
    }

    #[test]
    fn ciphertext_is_bound_to_its_scope() {
        let master = gen_key();
        let encrypted = encrypt_with_master_secret(&master, &scope("user-1"), "secret")
            .expect("encryption succeeds");
        assert!(decrypt_with_master_secret(&master, &scope("user-2"), &encrypted).is_err());
    }

    #[test]
    fn encrypted_annotation_accepts_boolean_or_string_true() {
        for value in [Value::Bool(true), Value::String("true".to_string())] {
            let attribute = UserProfileAttribute {
                annotations: Some(HashMap::from([(
                    SECRET_ATTRIBUTE_ANNOTATION.to_string(),
                    value,
                )])),
                display_name: None,
                group: None,
                multivalued: None,
                name: Some("private-reference".to_string()),
                required: None,
                validations: None,
                permissions: None,
                selector: None,
            };
            assert_eq!(
                storage_policy(&attribute),
                VoterAttributeStoragePolicy::Encrypted
            );
        }
    }

    fn secret_attribute(name: &str) -> UserProfileAttribute {
        UserProfileAttribute {
            annotations: Some(HashMap::from([(
                SECRET_ATTRIBUTE_ANNOTATION.to_string(),
                Value::Bool(true),
            )])),
            display_name: None,
            group: None,
            multivalued: None,
            name: Some(name.to_string()),
            required: None,
            validations: None,
            permissions: None,
            selector: None,
        }
    }

    #[test]
    fn envelope_of_the_largest_allowed_plaintext_fits_the_keycloak_value_column() {
        let master = gen_key();
        let plaintext = "x".repeat(MAX_SECRET_VALUE_BYTES);
        let envelope = encrypt_with_master_secret(&master, &scope("user-1"), &plaintext)
            .expect("encryption succeeds");
        assert!(envelope.len() <= MAX_ENCRYPTED_VALUE_CHARS);
        assert_eq!(
            decrypt_with_master_secret(&master, &scope("user-1"), &envelope).unwrap(),
            plaintext
        );
    }

    #[test]
    fn plaintext_over_the_limit_is_rejected() {
        let master = gen_key();
        let plaintext = "x".repeat(MAX_SECRET_VALUE_BYTES + 1);
        assert!(encrypt_with_master_secret(&master, &scope("user-1"), &plaintext).is_err());
    }

    #[test]
    fn built_in_name_fields_cannot_be_secret() {
        for name in [
            "first_name",
            "last_name",
            "firstName",
            "lastName",
            "email",
            "username",
        ] {
            let config = SecretAttributeConfig::from_profile(&[secret_attribute(name)]);
            assert!(config.validated_names().is_err(), "{name} must be rejected");
            assert!(config.redacted_names().contains(name));
        }
    }

    #[test]
    fn misconfigured_profile_is_still_redacted_but_refuses_validation() {
        let config = SecretAttributeConfig::from_profile(&[
            secret_attribute("private-reference"),
            secret_attribute("email"),
        ]);
        assert_eq!(
            config.redacted_names(),
            &HashSet::from(["private-reference".to_string(), "email".to_string()])
        );
        assert!(config.validated_names().is_err());

        let mut user = User {
            attributes: Some(HashMap::from([
                (
                    "private-reference".to_string(),
                    vec!["seqenc:v1:x".to_string()],
                ),
                ("public".to_string(), vec!["visible".to_string()]),
            ])),
            ..Default::default()
        };
        redact_user(&mut user, config.redacted_names());
        let attributes = user.attributes.unwrap();
        assert_eq!(attributes["private-reference"], vec![REDACTED_SECRET_VALUE]);
        assert_eq!(attributes["public"], vec!["visible"]);
    }

    #[test]
    fn undeclared_secret_attributes_are_stripped_and_declared_ones_kept() {
        let mut user = User {
            attributes: Some(HashMap::from([
                ("declared".to_string(), vec!["plain".to_string()]),
                ("undeclared".to_string(), vec!["seqenc:v1:x".to_string()]),
                ("public".to_string(), vec!["visible".to_string()]),
            ])),
            ..Default::default()
        };
        strip_undeclared_secret_attributes(
            &mut user,
            &HashSet::from(["declared".to_string(), "undeclared".to_string()]),
            &HashSet::from(["declared".to_string()]),
        );
        let attributes = user.attributes.unwrap();
        assert_eq!(attributes.get("declared"), Some(&vec!["plain".to_string()]));
        assert!(!attributes.contains_key("undeclared"));
        assert_eq!(attributes.get("public"), Some(&vec!["visible".to_string()]));
    }

    #[test]
    fn encrypted_attributes_allow_required_but_reject_value_validation_rules() {
        let encrypted_annotation = Some(HashMap::from([(
            SECRET_ATTRIBUTE_ANNOTATION.to_string(),
            Value::Bool(true),
        )]));
        let mut attribute = UserProfileAttribute {
            annotations: encrypted_annotation,
            display_name: None,
            group: None,
            multivalued: None,
            name: Some("private-reference".to_string()),
            required: Some(sequent_core::types::keycloak::UPAttributeRequired {
                roles: None,
                scopes: None,
            }),
            validations: Some(HashMap::from([(
                "person-name-prohibited-characters".to_string(),
                HashMap::new(),
            )])),
            permissions: None,
            selector: None,
        };
        let names = secret_attribute_names(&[attribute.clone()]).unwrap();
        assert!(names.contains("private-reference"));

        attribute.validations = Some(HashMap::from([(
            "length".to_string(),
            HashMap::from([("max".to_string(), Value::from(255))]),
        )]));
        assert!(secret_attribute_names(&[attribute]).is_err());
    }
}
