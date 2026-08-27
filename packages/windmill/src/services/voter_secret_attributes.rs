// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::vault::vault::get_master_secret;
use anyhow::{anyhow, Context, Result};
use base64::prelude::{Engine as _, BASE64_URL_SAFE_NO_PAD};
use ring::hkdf;
use sequent_core::types::keycloak::{
    User, UserProfileAttribute, FIRST_NAME_ATTRIBUTE, LAST_NAME_ATTRIBUTE,
};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use strand::serialization::{StrandDeserialize, StrandSerialize};
use strand::symm::{decrypt, encrypt, EncryptionData, SymmetricKey};

pub const SECRET_ATTRIBUTE_ANNOTATION: &str = "sequent.secret";
pub const ENCRYPTED_VALUE_PREFIX: &str = "seqenc:v1:";
pub const REDACTED_SECRET_VALUE: &str = "<redacted>";

const KEY_DERIVATION_DOMAIN: &[u8] = b"sequent-voter-secret-attribute-v1";
const MAX_SECRET_VALUE_BYTES: usize = 4096;
const CIPHERTEXT_COMPATIBLE_VALIDATORS: [&str; 1] = ["person-name-prohibited-characters"];
const FORBIDDEN_SECRET_ATTRIBUTES: [&str; 13] = [
    "area-id",
    "authorized-election-ids",
    "authorized-to-election-alias",
    "dateOfBirth",
    "disable-comment",
    "email",
    "permission_labels",
    "sequent.read-only.id-card-number-validated",
    "sequent.read-only.mobile-number",
    "tenant-id",
    "username",
    "vote-weight",
    "voted-channel",
];

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

pub fn secret_attribute_names(attributes: &[UserProfileAttribute]) -> Result<HashSet<String>> {
    attributes
        .iter()
        .filter(|attribute| storage_policy(attribute) == VoterAttributeStoragePolicy::Encrypted)
        .map(|attribute| {
            let name = attribute
                .name
                .clone()
                .ok_or_else(|| anyhow!("An encrypted user-profile attribute has no name"))?;
            if FORBIDDEN_SECRET_ATTRIBUTES.contains(&name.as_str()) {
                return Err(anyhow!(
                    "User-profile attribute `{name}` cannot be configured as encrypted"
                ));
            }
            if attribute.validations.as_ref().is_some_and(|validations| {
                validations
                    .keys()
                    .any(|name| !CIPHERTEXT_COMPATIBLE_VALIDATORS.contains(&name.as_str()))
            }) {
                return Err(anyhow!(
                    "Encrypted user-profile attribute `{name}` cannot use Keycloak value validators"
                ));
            }
            Ok(name)
        })
        .collect()
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

fn encrypt_with_master_secret(
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
    Ok(format!(
        "{ENCRYPTED_VALUE_PREFIX}{}",
        BASE64_URL_SAFE_NO_PAD.encode(serialized)
    ))
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
    match name {
        FIRST_NAME_ATTRIBUTE => user.first_name.clone().into_iter().collect(),
        LAST_NAME_ATTRIBUTE => user.last_name.clone().into_iter().collect(),
        _ => user
            .attributes
            .as_ref()
            .and_then(|attributes| attributes.get(name))
            .cloned()
            .unwrap_or_default(),
    }
}

fn set_user_attribute_values(user: &mut User, name: &str, values: Vec<String>) -> Result<()> {
    match name {
        FIRST_NAME_ATTRIBUTE => {
            if values.len() > 1 {
                return Err(anyhow!(
                    "Built-in voter attribute `{name}` cannot be multivalued"
                ));
            }
            user.first_name = values.into_iter().next();
        }
        LAST_NAME_ATTRIBUTE => {
            if values.len() > 1 {
                return Err(anyhow!(
                    "Built-in voter attribute `{name}` cannot be multivalued"
                ));
            }
            user.last_name = values.into_iter().next();
        }
        _ => {
            user.attributes
                .get_or_insert_with(HashMap::new)
                .insert(name.to_string(), values);
        }
    }
    Ok(())
}

pub fn redact_user(user: &mut User, secret_names: &HashSet<String>) {
    for name in secret_names {
        match name.as_str() {
            FIRST_NAME_ATTRIBUTE if user.first_name.is_some() => {
                user.first_name = Some(REDACTED_SECRET_VALUE.to_string());
            }
            LAST_NAME_ATTRIBUTE if user.last_name.is_some() => {
                user.last_name = Some(REDACTED_SECRET_VALUE.to_string());
            }
            _ => {
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
    let mut encrypted = HashMap::new();
    for (name, values) in values {
        if !secret_names.contains(&name) {
            return Err(anyhow!(
                "User-profile attribute `{name}` is not configured as encrypted"
            ));
        }
        let values = values.unwrap_or_default();
        if matches!(name.as_str(), FIRST_NAME_ATTRIBUTE | LAST_NAME_ATTRIBUTE) && values.len() > 1 {
            return Err(anyhow!(
                "Built-in voter attribute `{name}` cannot be multivalued"
            ));
        }
        encrypted.insert(
            name.clone(),
            encrypt_attribute_values(tenant_id, election_event_id, user_id, &name, &values).await?,
        );
    }
    Ok(encrypted)
}

#[cfg(test)]
mod tests {
    use super::*;
    use strand::symm::gen_key;

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

    #[test]
    fn built_in_values_are_read_and_redacted_from_top_level_fields() {
        let mut user = User {
            first_name: Some("encrypted-first".to_string()),
            last_name: Some("encrypted-last".to_string()),
            ..Default::default()
        };
        assert_eq!(
            user_attribute_values(&user, FIRST_NAME_ATTRIBUTE),
            vec!["encrypted-first"]
        );
        assert_eq!(
            user_attribute_values(&user, LAST_NAME_ATTRIBUTE),
            vec!["encrypted-last"]
        );

        redact_user(
            &mut user,
            &HashSet::from([
                FIRST_NAME_ATTRIBUTE.to_string(),
                LAST_NAME_ATTRIBUTE.to_string(),
            ]),
        );
        assert_eq!(user.first_name.as_deref(), Some(REDACTED_SECRET_VALUE));
        assert_eq!(user.last_name.as_deref(), Some(REDACTED_SECRET_VALUE));
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
