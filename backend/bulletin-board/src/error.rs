// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use strand::util::StrandError;
use thiserror::Error as ThisError;
use tonic::{Code, Status};

/// Bulletin board library error type
#[derive(ThisError, Debug)]
pub enum Error {
    /// Permissions is None
    #[error("Permissions is None")]
    PermissionsIsNone,

    /// Contains duplicated user names
    #[error("Contains duplicated user names")]
    DuplicatedUserNames,

    /// Contains duplicated role names
    #[error("Contains duplicated role names")]
    DuplicatedRoleNames,

    /// Contains duplicated Permission names
    #[error("Contains duplicated Permission names")]
    DuplicatedPermissionNames,

    /// Invalid identifier
    #[error("Invalid `{0}` identifier: `{1}`")]
    InvalidIdentifier(String, String),

    /// Empty user list
    #[error("Empty user list")]
    EmptyUserList,

    /// Empty role list
    #[error("Empty role list")]
    EmptyRoleList,

    /// Empty user roles list
    #[error("Empty user roles list")]
    EmptyUserRoleList,

    /// User name not found
    #[error("User name `{0}` not found")]
    UserNotFound(String),

    /// Role name not found
    #[error("Role name `{0}` not found")]
    RoleNotFound(String),

    /// Error decoding public key
    #[error("Error decoding public key: `{0}`")]
    ErrorDecodingPublicKey(String),

    /// Error encoding public key
    #[error("Error encoding public key: `{0}`")]
    ErrorEncodingPublicKey(String),

    /// Error decoding signature
    #[error("Error decoding signature: `{0}`")]
    ErrorDecodingSignature(String),

    /// Error encoding signature
    #[error("Error encoding signature: `{0}`")]
    ErrorEncodingSignature(String),

    /// Signature error
    #[error("Signature error: `{0}`")]
    SignatureVerificationError(String),

    /// Strand error
    #[error(transparent)]
    StrandError(#[from] StrandError),

    /// Missing permissions
    #[error("Missing permissions: `{0:?}`")]
    MissingPermissions(Vec<String>),

    /// Invalid permissions
    #[error("Invalid permissions: `{0:?}`")]
    InvalidPermissions(Vec<String>),
}

impl From<Error> for Status {
    fn from(error: Error) -> Self {
        match error {
            Error::MissingPermissions(_) => {
                Status::new(Code::PermissionDenied, error.to_string())
            }
            _ => Status::new(Code::Unknown, error.to_string()),
        }
    }
}
