// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
//! Pre-write check that an election-event realm stores the user attributes an
//! inbound Datafix operation is about to write.
//!
//! Keycloak's declarative user profile silently drops, from an admin write,
//! any attribute the realm does not declare, unless the unmanaged-attribute
//! policy lets administrators write undeclared attributes. It also drops a
//! declared attribute administrators are not permitted to edit. Without this
//! check such an operation would answer success to the external system, the
//! electoral log would record the requested values, and the voter would keep
//! the previous ones.
use super::types::DatafixError;
use keycloak::types::{UPAttribute, UPConfig, UnmanagedAttributePolicy};
use sequent_core::services::keycloak::KeycloakAdminClient;
use sequent_core::types::keycloak::PERMISSION_TO_EDIT;
use tracing::{error, instrument, warn};

/// Fails, before anything is written, when the realm would drop any of the
/// `attributes` the operation writes.
#[instrument(skip(client), err)]
pub async fn ensure_realm_stores_attributes(
    client: &KeycloakAdminClient,
    realm: &str,
    attributes: &[&str],
) -> Result<(), DatafixError> {
    if attributes.is_empty() {
        return Ok(());
    }
    let profile = client.get_user_profile(realm).await.map_err(|e| {
        error!("Error reading the realm user profile: {e:?}");
        DatafixError::internal(format!("Error reading the realm user profile: {e}"))
    })?;
    let dropped = attributes_dropped_by_profile(&profile, attributes);
    if dropped.is_empty() {
        return Ok(());
    }
    warn!("Realm user profile does not store attributes {dropped:?}");
    Err(realm_drops_attributes(&dropped))
}

/// The attributes, among `attributes`, that an admin write to a realm with
/// this user profile would silently drop.
#[instrument(skip_all)]
pub fn attributes_dropped_by_profile(profile: &UPConfig, attributes: &[&str]) -> Vec<String> {
    let admin_writes_unmanaged = matches!(
        profile.unmanaged_attribute_policy,
        Some(UnmanagedAttributePolicy::Enabled | UnmanagedAttributePolicy::AdminEdit)
    );
    let declared = profile.attributes.as_deref().unwrap_or_default();
    attributes
        .iter()
        .filter(|attribute| {
            match declared
                .iter()
                .find(|declared| declared.name.as_deref() == Some(**attribute))
            {
                Some(declared) => !admin_can_edit(declared),
                None => !admin_writes_unmanaged,
            }
        })
        .map(|attribute| attribute.to_string())
        .collect()
}

/// A declared attribute without explicit permissions is editable by
/// administrators only; with explicit permissions, only if `edit` names them.
fn admin_can_edit(attribute: &UPAttribute) -> bool {
    attribute
        .permissions
        .as_ref()
        .and_then(|permissions| permissions.edit.as_ref())
        .map_or(true, |edit| edit.contains(&PERMISSION_TO_EDIT.to_string()))
}

/// The error recorded when the realm would drop `attributes`. It is internal
/// because the fix is in the election event's user profile, not in the
/// request.
fn realm_drops_attributes(attributes: &[String]) -> DatafixError {
    DatafixError::internal(format!(
        "Realm user profile does not store attributes [{}]: declare them in the election event user profile or allow administrators to edit unmanaged attributes",
        attributes.join(", ")
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::datafix::types::DatafixErrorCode;
    use keycloak::types::UPAttributePermissions;
    use sequent_core::types::keycloak::{DATE_OF_BIRTH, DISABLE_COMMENT, VOTED_CHANNEL};

    fn declared(name: &str, edit: Option<Vec<&str>>) -> UPAttribute {
        UPAttribute {
            name: Some(name.into()),
            permissions: edit.map(|edit| UPAttributePermissions {
                edit: Some(
                    edit.iter()
                        .map(|role| role.to_string())
                        .collect::<Vec<_>>()
                        .into(),
                ),
                view: None,
            }),
            ..UPAttribute::default()
        }
    }

    fn profile(attributes: Vec<UPAttribute>, policy: Option<UnmanagedAttributePolicy>) -> UPConfig {
        UPConfig {
            attributes: Some(attributes.into()),
            groups: None,
            unmanaged_attribute_policy: policy,
        }
    }

    #[test]
    fn declared_attributes_administrators_can_edit_are_stored() {
        let profile = profile(
            vec![
                declared(DATE_OF_BIRTH, None),
                declared(VOTED_CHANNEL, Some(vec!["admin"])),
                declared(DISABLE_COMMENT, Some(vec!["admin", "user"])),
            ],
            None,
        );
        assert!(attributes_dropped_by_profile(
            &profile,
            &[DATE_OF_BIRTH, VOTED_CHANNEL, DISABLE_COMMENT]
        )
        .is_empty());
    }

    #[test]
    fn undeclared_attributes_are_dropped_unless_administrators_may_write_unmanaged_ones() {
        for policy in [None, Some(UnmanagedAttributePolicy::AdminView)] {
            let profile = profile(vec![declared(VOTED_CHANNEL, None)], policy);
            assert_eq!(
                attributes_dropped_by_profile(&profile, &[VOTED_CHANNEL, DATE_OF_BIRTH]),
                vec![DATE_OF_BIRTH.to_string()]
            );
        }
        for policy in [
            UnmanagedAttributePolicy::Enabled,
            UnmanagedAttributePolicy::AdminEdit,
        ] {
            let profile = profile(vec![], Some(policy));
            assert!(
                attributes_dropped_by_profile(&profile, &[VOTED_CHANNEL, DATE_OF_BIRTH]).is_empty()
            );
        }
    }

    #[test]
    fn declared_attributes_administrators_cannot_edit_are_dropped() {
        let profile = profile(
            vec![declared(DATE_OF_BIRTH, Some(vec!["user"]))],
            Some(UnmanagedAttributePolicy::Enabled),
        );
        assert_eq!(
            attributes_dropped_by_profile(&profile, &[DATE_OF_BIRTH]),
            vec![DATE_OF_BIRTH.to_string()]
        );
    }

    #[test]
    fn a_profile_without_attributes_drops_everything() {
        let profile = UPConfig {
            attributes: None,
            groups: None,
            unmanaged_attribute_policy: None,
        };
        assert_eq!(
            attributes_dropped_by_profile(&profile, &[DATE_OF_BIRTH, DISABLE_COMMENT]),
            vec![DATE_OF_BIRTH.to_string(), DISABLE_COMMENT.to_string()]
        );
        assert!(attributes_dropped_by_profile(&profile, &[]).is_empty());
    }

    #[test]
    fn the_error_is_internal_and_names_the_attributes() {
        let err = realm_drops_attributes(&[DATE_OF_BIRTH.to_string(), VOTED_CHANNEL.to_string()]);
        assert_eq!(err.code, DatafixErrorCode::InternalError);
        assert_eq!(
            err.detail,
            "Realm user profile does not store attributes [dateOfBirth, voted-channel]: declare them in the election event user profile or allow administrators to edit unmanaged attributes"
        );
    }
}
