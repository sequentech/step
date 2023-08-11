// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use serde_with::serde_as;
use std::collections::HashSet;

use crate::error::Error;
use crate::permissions::{ValidPermissionNames, ValidatePermissionNames};
use crate::util::Validate;

tonic::include_proto!("bulletin_board");

impl AsRef<Permissions> for Board {
    fn as_ref(&self) -> &Permissions {
        // Note that we do an unwrap here! no way around that.
        self.permissions.as_ref().unwrap()
    }
}

impl ValidPermissionNames for Board {
    fn valid_permission_names() -> HashSet<String> {
        HashSet::from(["AddEntries".to_string(), "ModifyBoard".into()])
    }
}

impl Validate for Board {
    fn validate(&self) -> Result<(), Error> {
        // validate permissions. Note that we don't take advantage of the
        // `AsRef<Permissions>` because of the potential unwrap call there
        let permissions: &Permissions =
            self.permissions.as_ref().ok_or(Error::PermissionsIsNone)?;
        permissions.validate()?;

        // check permission list doesn't include any strange permission
        self.validate_permission_names()?;

        Ok(())
    }
}

impl TryFrom<(&CreateBoardRequest, String)> for Board {
    type Error = Error;

    fn try_from(
        (request, public_key): (&CreateBoardRequest, String),
    ) -> Result<Self, Self::Error> {
        let board = Board {
            uuid: request.board_uuid.clone(),
            name: request.board_name.clone(),
            description: request.board_description.clone(),
            metadata: request.board_metadata.clone(),
            is_archived: false,
            permissions: Some(
                request
                    .permissions
                    .as_ref()
                    .ok_or(Error::PermissionsIsNone)?
                    .clone(),
            ),
            public_key,
        };
        board.validate()?;
        Ok(board)
    }
}

impl From<Board> for ModifyBoardRequest {
    fn from(board: Board) -> Self {
        ModifyBoardRequest {
            board_uuid: board.uuid,
            board_name: board.name,
            board_description: board.description,
            board_metadata: board.metadata,
            is_archived: board.is_archived,
            permissions: board.permissions,
            signer_public_key: board.public_key,
            signature: String::new(),
        }
    }
}

impl TryFrom<(&ModifyBoardRequest, String)> for Board {
    type Error = Error;

    fn try_from(
        (request, public_key): (&ModifyBoardRequest, String),
    ) -> Result<Self, Self::Error> {
        let board = Board {
            uuid: request.board_uuid.clone(),
            name: request.board_name.clone(),
            description: request.board_description.clone(),
            metadata: request.board_metadata.clone(),
            is_archived: request.is_archived,
            permissions: Some(
                request
                    .permissions
                    .as_ref()
                    .ok_or(Error::PermissionsIsNone)?
                    .clone(),
            ),
            public_key,
        };
        board.validate()?;
        Ok(board)
    }
}

#[cfg(test)]
mod tests {
    use crate::NewDataEntry;

    // Test that serde json serialization/deserialization of NewDataEntry.data
    // works using strings
    #[test]
    fn test_serde_json_base64() {
        let entry1 = NewDataEntry {
            data: b"Hello Board!".to_vec(),
            ..Default::default()
        };

        // data here is the result of executing:
        // `echo -n 'Hello Board!' | base64`
        let entry2: NewDataEntry = serde_json::from_str(
            r#"{
            "data": "SGVsbG8gQm9hcmQh",
            "metadata": {},
            "signer_public_key": "",
            "signature": ""
        }"#,
        )
        .unwrap();
        assert_eq!(entry1.data, entry2.data);
    }
}
