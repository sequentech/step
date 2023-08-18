// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use borsh::{BorshDeserialize, BorshSerialize};
use std::collections::HashMap;
use strand::serialization::StrandSerialize;
use strand::signature::{
    StrandSignature as Signature, StrandSignaturePk as PublicKey,
    StrandSignatureSk as SecretKey,
};
use strand::util::StrandError;

#[cfg(feature = "build-server")]
use std::{fs::File, io::Write, path::Path};

#[cfg(feature = "build-server")]
use crate::backend_trillian::{
    hash_leaf, Entry as TrillianEntry, TrillianError,
};

use crate::error::Error;
use crate::signature::Signable;
use crate::util::Now;
use crate::{
    board_entry, util::Timestamp, Board, BoardEntry, BoardEntryData,
    CreateBoardRequest, ModifyBoardRequest, NewDataEntry,
};

#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub enum EntryKind {
    BoardPublicConfig(Board),
    EntryData(Vec<u8>),
}

impl Default for EntryKind {
    fn default() -> Self {
        EntryKind::EntryData(vec![])
    }
}

impl From<board_entry::Kind> for EntryKind {
    fn from(entry_kind: board_entry::Kind) -> Self {
        match entry_kind {
            board_entry::Kind::Board(board) => {
                EntryKind::BoardPublicConfig(board)
            }
            board_entry::Kind::EntryData(BoardEntryData {
                data: Some(entry_data),
            }) => EntryKind::EntryData(entry_data),
            board_entry::Kind::EntryData(BoardEntryData { data: None }) => {
                EntryKind::EntryData(vec![])
            }
        }
    }
}

impl From<EntryKind> for board_entry::Kind {
    fn from(entry_kind: EntryKind) -> Self {
        match entry_kind {
            EntryKind::BoardPublicConfig(board) => {
                board_entry::Kind::Board(board)
            }
            EntryKind::EntryData(entry_data) => {
                board_entry::Kind::EntryData(BoardEntryData {
                    data: Some(entry_data),
                })
            }
        }
    }
}

/// Represents an entry in the bulletin board
#[derive(BorshSerialize, BorshDeserialize, Clone, Default)]
pub struct Entry {
    /// Indicates entry kind and contains the data
    pub kind: EntryKind,

    /// Entry time stamp
    pub timestamp: Timestamp,

    /// Entry extensible metadata
    pub metadata: HashMap<String, String>,

    // Signer of the board entry.
    pub signer_public_key: String,

    /// Entry signature
    pub signature: String,
}

impl From<BoardEntry> for Entry {
    fn from(entry: BoardEntry) -> Self {
        Entry {
            kind: EntryKind::from(entry.kind.unwrap()),
            timestamp: entry.timestamp,
            metadata: entry.metadata,
            signer_public_key: entry.signer_public_key,
            signature: entry.signature,
        }
    }
}

impl From<NewDataEntry> for Entry {
    fn from(entry: NewDataEntry) -> Self {
        Entry {
            kind: EntryKind::EntryData(entry.data),
            timestamp: Timestamp::now(),
            metadata: entry.metadata,
            signer_public_key: entry.signer_public_key,
            signature: entry.signature,
        }
    }
}

impl TryFrom<(&CreateBoardRequest, &Board)> for Entry {
    type Error = Error;
    fn try_from(
        (request, board): (&CreateBoardRequest, &Board),
    ) -> Result<Self, Self::Error> {
        Ok(Entry {
            kind: EntryKind::BoardPublicConfig(board.clone()),
            timestamp: Timestamp::now(),
            metadata: HashMap::new(),
            signer_public_key: request.signer_public_key.clone(),
            signature: request.signature.clone(),
        })
    }
}

impl TryFrom<(&ModifyBoardRequest, &Board)> for Entry {
    type Error = Error;
    fn try_from(
        (request, board): (&ModifyBoardRequest, &Board),
    ) -> Result<Self, Self::Error> {
        Ok(Entry {
            kind: EntryKind::BoardPublicConfig(board.clone()),
            timestamp: Timestamp::now(),
            metadata: HashMap::new(),
            signer_public_key: request.signer_public_key.clone(),
            signature: request.signature.clone(),
        })
    }
}

impl Entry {
    pub fn into_board_entry(self, sequence_id: u64) -> BoardEntry {
        BoardEntry {
            sequence_id,
            kind: Some(board_entry::Kind::from(self.kind)),
            timestamp: self.timestamp,
            metadata: self.metadata,
            signer_public_key: self.signer_public_key,
            signature: self.signature,
        }
    }

    /// Writes the serialized entry into a given file path containing and return
    /// a TrillianEntry, which holds the hash and a string pointing to the
    /// file path.
    #[cfg(feature = "build-server")]
    pub fn into_trillian_entry(
        self,
        file_path: &Path,
    ) -> Result<TrillianEntry, TrillianError> {
        let entry_bytes = self
            .try_to_vec()
            .map_err(|_| TrillianError::EntrySerializationError)?;
        let hash = hash_leaf(&entry_bytes);

        // Create a file
        let mut temp_file = File::create(file_path)
            .map_err(|_| TrillianError::EntrySerializationError)?;
        temp_file
            .write(&entry_bytes)
            .map_err(|_| TrillianError::EntrySerializationError)?;
        return Ok(TrillianEntry {
            hash: hash
                .try_into()
                .map_err(|_| TrillianError::EntrySerializationError)?,
            temp_file_path: file_path.to_string_lossy().into(),
        });
    }
}

impl Signable for Entry {
    // Please note that it's important that Entry::signable_data() returns the
    // same bit by bit to what NewEntryData::signable_data() or
    // CreateBoardRequest::signable_data() return, depending on the entry.kind.
    //
    // Otherwise sending signed requests to the Board Service would not be
    // stored in a signature-verifiable manner.
    fn signable_data(&self) -> Result<Vec<u8>, Error> {
        Ok(match &self.kind {
            // The signable board does not include the board's public key
            // because during creation it was not known by the signer as it
            // didn't exist yet.
            EntryKind::BoardPublicConfig(board) => Board {
                public_key: "".to_string(),
                ..board.clone()
            }
            .strand_serialize()?,
            EntryKind::EntryData(data) => data.strand_serialize()?,
        })
    }

    fn signature(&self) -> Result<Signature, Error> {
        self.signature
            .clone()
            .try_into()
            .map_err(|error: StrandError| {
                Error::ErrorDecodingSignature(error.to_string())
            })
    }

    fn signer_public_key(&self) -> Result<PublicKey, Error> {
        self.signer_public_key.clone().try_into().map_err(
            |error: StrandError| {
                Error::ErrorDecodingPublicKey(error.to_string())
            },
        )
    }

    fn sign(&self, secret_key: &SecretKey) -> Result<Self, Error> {
        let signable_data = self.signable_data()?;
        Ok(Entry {
            signer_public_key: PublicKey::from(secret_key).try_into().map_err(
                |err: StrandError| {
                    Error::ErrorEncodingPublicKey(err.to_string())
                },
            )?,
            signature: secret_key.sign(&signable_data).try_into().map_err(
                |err: StrandError| {
                    Error::ErrorEncodingSignature(err.to_string())
                },
            )?,
            ..self.clone()
        })
    }
}

impl Signable for CreateBoardRequest {
    // Signable data is the encoded board without the public_key, because the
    // requester couldn't possibly know the public key at the time since it
    // would be created later by the Board service.
    //
    // Also, this allows to make the signature match to the signature of the
    // Board since it's important that CreateBoardRequest::signable_data()
    // returns the same as the Entry::signable_data() with
    // EntryKind::PublicBoardEntry() to make it consistent.
    fn signable_data(&self) -> Result<Vec<u8>, Error> {
        let board: Board = self.into();
        Ok(board.strand_serialize()?)
    }

    fn signature(&self) -> Result<Signature, Error> {
        self.signature
            .clone()
            .try_into()
            .map_err(|error: StrandError| {
                Error::ErrorDecodingSignature(error.to_string())
            })
    }

    fn signer_public_key(&self) -> Result<PublicKey, Error> {
        self.signer_public_key.clone().try_into().map_err(
            |error: StrandError| {
                Error::ErrorDecodingPublicKey(error.to_string())
            },
        )
    }

    fn sign(&self, secret_key: &SecretKey) -> Result<Self, Error> {
        let signable_data = self.signable_data()?;
        Ok(CreateBoardRequest {
            signer_public_key: PublicKey::from(secret_key).try_into().map_err(
                |err: StrandError| {
                    Error::ErrorEncodingPublicKey(err.to_string())
                },
            )?,
            signature: secret_key.sign(&signable_data).try_into().map_err(
                |err: StrandError| {
                    Error::ErrorEncodingSignature(err.to_string())
                },
            )?,
            ..self.clone()
        })
    }
}

impl Signable for ModifyBoardRequest {
    // Signable data is the encoded board without the public_key, because the
    // requester couldn't possibly know the public key at the time since it
    // would be created later by the Board service.
    //
    // Also, this allows to make the signature match to the signature of the
    // Board since it's important that CreateBoardRequest::signable_data()
    // returns the same as the Entry::signable_data() with
    // EntryKind::PublicBoardEntry() to make it consistent.
    fn signable_data(&self) -> Result<Vec<u8>, Error> {
        let board: Board = self.into();
        Ok(board.strand_serialize()?)
    }

    fn signature(&self) -> Result<Signature, Error> {
        self.signature
            .clone()
            .try_into()
            .map_err(|error: StrandError| {
                Error::ErrorDecodingSignature(error.to_string())
            })
    }

    fn signer_public_key(&self) -> Result<PublicKey, Error> {
        self.signer_public_key.clone().try_into().map_err(
            |error: StrandError| {
                Error::ErrorDecodingPublicKey(error.to_string())
            },
        )
    }

    fn sign(&self, secret_key: &SecretKey) -> Result<Self, Error> {
        let signable_data = self.signable_data()?;
        Ok(ModifyBoardRequest {
            signer_public_key: PublicKey::from(secret_key).try_into().map_err(
                |err: StrandError| {
                    Error::ErrorEncodingPublicKey(err.to_string())
                },
            )?,
            signature: secret_key.sign(&signable_data).try_into().map_err(
                |err: StrandError| {
                    Error::ErrorEncodingSignature(err.to_string())
                },
            )?,
            ..self.clone()
        })
    }
}

impl Signable for NewDataEntry {
    // Note that it is important that NewDataEntry::signable_data() returns the
    // same as the Entry::signable_data() with EntryKind::EntryData() to make it
    // consistent. In this case, it just returns the data.
    fn signable_data(&self) -> Result<Vec<u8>, Error> {
        Ok(self.data.strand_serialize()?)
    }

    fn signature(&self) -> Result<Signature, Error> {
        self.signature
            .clone()
            .try_into()
            .map_err(|error: StrandError| {
                Error::ErrorDecodingSignature(error.to_string())
            })
    }

    fn signer_public_key(&self) -> Result<PublicKey, Error> {
        self.signer_public_key.clone().try_into().map_err(
            |error: StrandError| {
                Error::ErrorDecodingPublicKey(error.to_string())
            },
        )
    }

    fn sign(&self, secret_key: &SecretKey) -> Result<Self, Error> {
        let signable_data = self.signable_data()?;
        Ok(NewDataEntry {
            signer_public_key: PublicKey::from(secret_key).try_into().map_err(
                |err: StrandError| {
                    Error::ErrorEncodingPublicKey(err.to_string())
                },
            )?,
            signature: secret_key.sign(&signable_data).try_into().map_err(
                |err: StrandError| {
                    Error::ErrorEncodingSignature(err.to_string())
                },
            )?,
            ..self.clone()
        })
    }
}

#[cfg(test)]
mod tests {
    use strand::rnd::StrandRng;
    use strand::signature::{
        StrandSignaturePk as PublicKey, StrandSignatureSk as SecretKey,
    };

    use crate::signature::Signable;
    use crate::{NewDataEntry, Permissions, Role, User, UserRole};

    lazy_static! {
        // Service user secret (and public) key, used for creating boards and
        // also for executing board actions
        static ref SECRET_KEY: SecretKey = {
            let mut generator = StrandRng;
            SecretKey::new(&mut generator)
        };

        static ref SECRET_KEY2: SecretKey = {
            let mut generator = StrandRng;
            SecretKey::new(&mut generator)
        };
    }

    fn public_key() -> String {
        PublicKey::from(&*SECRET_KEY).clone().try_into().unwrap()
    }

    fn public_key2() -> String {
        PublicKey::from(&*SECRET_KEY2).clone().try_into().unwrap()
    }

    fn basic_permissions() -> Permissions {
        Permissions {
            users: vec![User {
                name: "admin".into(),
                public_key: public_key(),
                ..Default::default()
            }],
            roles: vec![Role {
                name: "admins".into(),
                permissions: vec!["CreateBoard".into()],
                ..Default::default()
            }],
            user_roles: vec![UserRole {
                user_name: "admin".into(),
                role_names: vec!["admins".into()],
            }],
            ..Default::default()
        }
    }

    #[test]
    fn test_newdataentry_verify_permissions() {
        let permissions = basic_permissions();
        let signed_data = NewDataEntry {
            data: b"Hello world".to_vec(),
            ..Default::default()
        }
        .sign(&*SECRET_KEY)
        .unwrap();

        // This should verify correctly, the permission is granted
        let verification_result = permissions
            .verify_permissions(&signed_data, vec!["CreateBoard".to_string()]);
        assert!(verification_result.is_ok(), "{verification_result:?}");

        // This should not verify, that permission is not granted
        let verification_result = permissions.verify_permissions(
            &signed_data,
            vec!["SomeOtherPermission".to_string()],
        );
        assert!(verification_result.is_err(), "{verification_result:?}");
    }

    #[test]
    fn test_newdataentry_err_verify_permissions() {
        let permissions = basic_permissions();
        let data = NewDataEntry {
            data: b"Hello world".to_vec(),
            ..Default::default()
        };

        // This should not verify, the data is not signed
        let verification_result = permissions
            .verify_permissions(&data, vec!["CreateBoard".to_string()]);
        assert!(verification_result.is_err(), "{verification_result:?}");

        // This should not verify either - it's signed by someone who has no
        // permissions nor user
        let signed_data = data.sign(&*SECRET_KEY2).unwrap();
        let verification_result = permissions
            .verify_permissions(&signed_data, vec!["CreateBoard".to_string()]);
        assert!(verification_result.is_err(), "{verification_result:?}");
    }

    #[test]
    fn test_newdataentry_complex_verify_permissions() {
        let permissions = Permissions {
            users: vec![
                User {
                    name: "user1".into(),
                    public_key: public_key(),
                    ..Default::default()
                },
                User {
                    name: "user2".into(),
                    public_key: public_key2(),
                    ..Default::default()
                },
            ],
            roles: vec![
                Role {
                    name: "admins".into(),
                    permissions: vec![
                        "AddEntries".into(),
                        "CreateBoard".into(),
                        "ChangeBoard".into(),
                        "ListEntries".into(),
                    ],
                    ..Default::default()
                },
                Role {
                    name: "writers".into(),
                    permissions: vec![
                        "AddEntries".into(),
                        "ListEntries".into(),
                    ],
                    ..Default::default()
                },
                Role {
                    name: "readers".into(),
                    permissions: vec!["ListEntries".into()],
                    ..Default::default()
                },
            ],
            user_roles: vec![
                UserRole {
                    user_name: "user1".into(),
                    role_names: vec!["admins".into()],
                },
                UserRole {
                    user_name: "user2".into(),
                    role_names: vec!["writers".into(), "readers".into()],
                },
            ],
            ..Default::default()
        };
        let signed_data = NewDataEntry {
            data: b"Hello world".to_vec(),
            ..Default::default()
        }
        .sign(&*SECRET_KEY2)
        .unwrap();

        // This should not verify, user2 is a writer but cannot create boards
        let verification_result = permissions.verify_permissions(
            &signed_data,
            vec!["CreateBoard".into(), "ListEntries".into()],
        );
        assert!(verification_result.is_err(), "{verification_result:?}");

        // This should verify - user2 can perform both actions
        let verification_result = permissions.verify_permissions(
            &signed_data,
            vec!["AddEntries".into(), "ListEntries".into()],
        );
        assert!(verification_result.is_ok(), "{verification_result:?}");

        // This should verify - user1 can create boards
        let signed_data = NewDataEntry {
            data: b"Hello world".to_vec(),
            ..Default::default()
        }
        .sign(&*SECRET_KEY)
        .unwrap();
        let verification_result = permissions.verify_permissions(
            &signed_data,
            vec!["CreateBoard".into(), "ListEntries".into()],
        );
        assert!(verification_result.is_ok(), "{verification_result:?}");
    }
}
