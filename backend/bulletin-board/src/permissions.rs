// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use base64::{engine::general_purpose, Engine as _};
use std::collections::HashSet;
use strand::serialization::StrandDeserialize;
use strand::signature::StrandSignaturePk as PublicKey;
use tracing::instrument;

use crate::signature::{Signable, Verify};
use crate::util::{is_valid_identifier, Validate};
use crate::{Error, Permissions};

impl Permissions {
    /// Verifies that a signed object is correctly signed by a public key with
    /// the required permissions
    #[instrument(skip(signed), err)]
    pub fn verify_permissions<T: Signable>(
        &self,
        signed: &T,
        required_permissions: Vec<String>,
    ) -> Result<(), Error> {
        // verify the signature of the signed object
        signed.verify_signature()?;

        let signer_public_key: String =
            signed.signer_public_key()?.try_into()?;

        // find the user associated to this public key
        let user = self
            .users
            .iter()
            .find(|user| user.public_key == signer_public_key)
            .ok_or_else(|| {
                Error::SignatureVerificationError("user not found".to_string())
            })?;

        // find the role names of this user name
        let mut roles: Vec<String> = self
            .user_roles
            .iter()
            .filter(|user_role| user_role.user_name == user.name)
            .flat_map(|user_role| user_role.role_names.clone())
            .collect();
        // deduplicate roles if any
        roles.sort();
        roles.dedup();

        // obtain permission list associated to all the roles of this user
        let mut permissions: Vec<String> = self
            .roles
            .iter()
            .filter(|role| roles.contains(&role.name))
            .flat_map(|role| role.permissions.clone())
            .collect();
        permissions.sort();
        permissions.dedup();

        // Check if there are any missing permissions and return
        let permissions_set: HashSet<_> = permissions.into_iter().collect();
        let required_permissions_set: HashSet<_> =
            required_permissions.into_iter().collect();
        let missing_permissions: HashSet<_> = required_permissions_set
            .difference(&permissions_set)
            .collect();
        if missing_permissions.is_empty() {
            Ok(())
        } else {
            Err(Error::MissingPermissions(
                missing_permissions.into_iter().cloned().collect(),
            ))
        }
    }

    /// Returns a list of all permission names assigned for all roles, deduped.
    ///
    /// Useful when implementing `Validate`, to check that only valid
    /// permissions names are being assigned.
    pub fn permission_names(&self) -> Vec<String> {
        // obtain permission list associated to all the roles of this user
        let mut permission_names: Vec<String> = self
            .roles
            .iter()
            .flat_map(|role| role.permissions.clone())
            .collect();
        permission_names.sort();
        permission_names.dedup();
        permission_names
    }
}

// Validates the internal consistency of the permissions. It does not check if:
// - permission names are valid other than that they are valid identifiers
// - there's an admin user
impl Validate for Permissions {
    fn validate(&self) -> Result<(), Error> {
        // Check user names are not repeated
        let mut user_names = self
            .users
            .iter()
            .map(|user| user.name.clone())
            .collect::<Vec<String>>();
        user_names.sort();
        user_names.dedup();
        if user_names.len() != self.users.len() {
            return Err(Error::DuplicatedUserNames);
        }

        // Validate user names
        user_names
            .iter()
            .map(|username| {
                is_valid_identifier(username).then_some(()).ok_or_else(|| {
                    Error::InvalidIdentifier(
                        String::from("user.name"),
                        username.clone(),
                    )
                })
            })
            .collect::<Result<Vec<()>, Error>>()?;

        // Check user public keys seem valid
        self.users
            .iter()
            .map(|user| {
                let public_key_bytes: Vec<u8> =
                    general_purpose::STANDARD_NO_PAD
                        .decode(user.public_key.clone())
                        .map_err(|_| {
                            Error::ErrorDecodingPublicKey(user.name.clone())
                        })?;
                let public_key: PublicKey =
                    PublicKey::strand_deserialize(&public_key_bytes).map_err(
                        |_| Error::ErrorDecodingPublicKey(user.name.clone()),
                    )?;
                Ok(public_key)
            })
            .collect::<Result<Vec<PublicKey>, Error>>()?;

        // Other permissions sanity checks
        if self.users.is_empty() {
            return Err(Error::EmptyUserList);
        }
        if self.roles.is_empty() {
            return Err(Error::EmptyRoleList);
        }
        if self.user_roles.is_empty() {
            return Err(Error::EmptyUserRoleList);
        }

        // Check role names are not repeated
        let mut role_names = self
            .roles
            .iter()
            .map(|role| role.name.clone())
            .collect::<Vec<String>>();
        role_names.sort();
        role_names.dedup();
        if role_names.len() != self.roles.len() {
            return Err(Error::DuplicatedRoleNames);
        }

        // Validate role names are valid identifiers
        role_names
            .iter()
            .map(|rolename| {
                is_valid_identifier(rolename).then_some(()).ok_or_else(|| {
                    Error::InvalidIdentifier(
                        String::from("role.name"),
                        rolename.clone(),
                    )
                })
            })
            .collect::<Result<Vec<()>, Error>>()?;

        // Validate permissions for each role
        self.roles
            .iter()
            .map(|role| {
                // Check permission names are not repeated within the role
                let mut permission_names = role.permissions.clone();
                permission_names.sort();
                permission_names.dedup();
                if permission_names.len() != role.permissions.len() {
                    return Err(Error::DuplicatedPermissionNames);
                }

                // Check permission names are valid identifiers
                permission_names
                    .iter()
                    .map(|permission| {
                        is_valid_identifier(permission)
                            .then_some(())
                            .ok_or_else(|| {
                                Error::InvalidIdentifier(
                                    String::from("role.permissions[i]"),
                                    permission.to_string(),
                                )
                            })
                    })
                    .collect::<Result<Vec<()>, Error>>()?;
                Ok(())
            })
            .collect::<Result<Vec<()>, Error>>()?;

        // Validate user roles correspond with an existing user name and
        // existing role names
        self.user_roles
            .iter()
            .map(|user_role| {
                // check user name corresponds to a listed user
                let valid_username = user_names
                    .iter()
                    .any(|user_name| user_name == &user_role.user_name);
                if !valid_username {
                    return Err(Error::UserNotFound(
                        user_role.user_name.clone(),
                    ));
                }

                // check that each assigned role name correspond with a listed
                // role
                for assigned_role_name in &user_role.role_names {
                    let valid_rolename =
                        role_names.iter().any(|existing_role_name| {
                            existing_role_name == assigned_role_name
                        });
                    if !valid_rolename {
                        return Err(Error::RoleNotFound(
                            assigned_role_name.clone(),
                        ));
                    }
                }
                Ok(())
            })
            .collect::<Result<Vec<()>, Error>>()?;

        Ok(())
    }
}

/// Trait that returns the list of valid permission names.
pub trait ValidPermissionNames {
    fn valid_permission_names() -> HashSet<String>;
}

/// Trait to validate permission names.
///
/// Typically you shouldn't implement it yourself. Instead, use the blanket
/// implementation below, by implementing both `ValidPermissionNames` and
/// `AsRef<Permissions>`.
pub trait ValidatePermissionNames {
    fn validate_permission_names(&self) -> Result<(), Error>;
}

impl<T> ValidatePermissionNames for T
where
    T: AsRef<Permissions> + ValidPermissionNames,
{
    fn validate_permission_names(&self) -> Result<(), Error> {
        let permissions: &Permissions = self.as_ref();
        let permission_names: HashSet<_> =
            permissions.permission_names().into_iter().collect();
        let allowed_names = T::valid_permission_names();
        let invalid_permissions: HashSet<_> =
            permission_names.difference(&allowed_names).collect();
        if !invalid_permissions.is_empty() {
            return Err(Error::InvalidPermissions(
                invalid_permissions.into_iter().cloned().collect(),
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::permissions::Permissions;
    use crate::util::{validate_test_suite, Validate};

    #[test]
    fn test_permission_names() {
        let permissions: Permissions = serde_json::from_value(serde_json::json!({
                "users": [{
                    "name": "admin",
                    "public_key": "gS2o6mE/9PmDYXHqcqKfyfHSVsoKuEip+olBk3YiQCM",
                    "metadata": {}
                }],
                "roles": [
                    {
                        "name": "admins",
                        "permissions": ["AddEntries", "ChangeBoard"],
                        "metadata": {
                            "no identifier validations here": "whatever ñ+p goes"
                        }
                    },
                    {
                        "name": "writers",
                        "permissions": ["AddEntries"],
                        "metadata": {}
                    }
                ],
                "user_roles": [{
                    "user_name": "admin",
                    "role_names": ["admins"]
                }]
            })).unwrap();

        assert_eq!(
            permissions.permission_names(),
            vec!["AddEntries".to_string(), "ChangeBoard".to_string()]
        );
    }

    #[test]
    fn test_permissions_validate() {
        let test_suite_str =
            include_str!("../fixtures/test_permissions_validate.toml");

        validate_test_suite(test_suite_str, |permissions: Permissions| {
            permissions.validate()
        });
    }
}
