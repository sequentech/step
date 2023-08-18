// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use borsh::{maybestd::io::Error as BorshError, BorshDeserialize};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;
use thiserror::Error as ThisError;
use tonic::{Code, Request, Response, Status};
use tracing::{debug, instrument};
use uuid::{Error as UuidError, Uuid};

use crate::backend_trillian;
use crate::backend_trillian::{Entry as TrillianEntry, TrillianError};
pub use crate::bulletin_board_server::{BulletinBoard, BulletinBoardServer};
use crate::entry::{Entry, EntryKind};
use crate::error::Error as BBError;
use crate::permissions::{ValidPermissionNames, ValidatePermissionNames};
use crate::signature::Verify;
use crate::util::{Now, Timestamp, Validate};
use crate::{
    board_entry, to_seq_path, AddEntriesRequest, AddEntriesResponse, Board,
    BoardConfig, BoardEntry, BoardEntryData, BoardUuid, Checkpoint,
    CreateBoardRequest, CreateBoardResponse, ListBoardItem, ListBoardsRequest,
    ListBoardsResponse, ListEntriesRequest, ListEntriesResponse,
    ModifyBoardRequest, ModifyBoardResponse, Permissions,
};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct BulletinBoardServiceConfig {
    pub storage_path: String,
    pub server_url: String,
    pub permissions: Permissions,
}

impl AsRef<Permissions> for BulletinBoardServiceConfig {
    fn as_ref(&self) -> &Permissions {
        &self.permissions
    }
}

impl ValidPermissionNames for BulletinBoardServiceConfig {
    fn valid_permission_names() -> HashSet<String> {
        HashSet::from(["CreateBoard".to_string()])
    }
}

impl Validate for BulletinBoardServiceConfig {
    fn validate(&self) -> Result<(), BBError> {
        self.permissions.validate()?;

        // check permission list doesn't include any invalid permission
        self.validate_permission_names()?;

        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct BulletinBoardService {
    config: BulletinBoardServiceConfig,
    boards: Arc<DashMap<BoardUuid, BoardConfig>>,
}

trait MapErrStatus<T> {
    fn map_err_status(self, code: Code) -> Result<T, Status>;
}

impl<T> MapErrStatus<T> for Result<T, String> {
    fn map_err_status(self, code: Code) -> Result<T, Status> {
        return self.map_err(|error| Status::new(code, error));
    }
}

impl<T> MapErrStatus<T> for Result<T, Error> {
    fn map_err_status(self, code: Code) -> Result<T, Status> {
        return self.map_err(|error| Status::new(code, error.to_string()));
    }
}

impl<T> MapErrStatus<T> for Result<T, TrillianError> {
    fn map_err_status(self, code: Code) -> Result<T, Status> {
        return self.map_err(|error| Status::new(code, error.to_string()));
    }
}

impl<T> MapErrStatus<T> for Result<T, UuidError> {
    fn map_err_status(self, code: Code) -> Result<T, Status> {
        return self.map_err(|error| Status::new(code, error.to_string()));
    }
}

impl TryFrom<String> for BulletinBoardService {
    type Error = Error;
    fn try_from(data: String) -> Result<Self, Error> {
        let service_config: BulletinBoardServiceConfig = toml::from_str(&data)?;
        service_config.validate()?;

        return BulletinBoardService::new(service_config);
    }
}

/// Enumerates all possible errors in the Board Service
#[derive(ThisError, Debug)]
pub enum Error {
    /// Error when storage path is not a directory
    #[error("Storage path `{0}` is not a directory")]
    StoragePathNotADirectory(String),

    /// Error when board config path is not a file
    #[error(
        "Board config at path=`{board_config_path:?}` for \
        board-uuid=`{board_uuid:?}` can't be found"
    )]
    BoardConfigNotAFile {
        board_uuid: String,
        board_config_path: PathBuf,
    },

    /// Error parsing a board entry
    #[error("Board entry at path=`{board_entry_path:?}` can't be read")]
    EntryParseError {
        board_entry_path: PathBuf,
        source: BorshError,
    },

    /// Board entry does not have expected entry kind
    #[error(
        "Board entry with sequence_id=`{sequence_id}` has not the \
        expected_entry_type=`{expected_entry_type}`"
    )]
    InvalidEntryType {
        sequence_id: u64,
        expected_entry_type: String,
    },

    /// Error when reading an board config entry and it is for a board uuid
    /// that is not the one expected
    #[error(
        "Board config entry for board-uuid=`{board_uuid}` contains the \
        configuration for a board with a different board_uuid=`{config_uuid}`"
    )]
    MismatchedBoardUuid {
        config_uuid: String,
        board_uuid: String,
    },

    /// All other cases of `std::io::Error`.
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// JSON Serde serialization error
    #[error(transparent)]
    SerdeJsonError(#[from] serde_json::Error),

    /// Toml Serde serialization error
    #[error(transparent)]
    SerdeTomlError(#[from] toml::de::Error),

    /// Trillian Error wrapper
    #[error(transparent)]
    TrillianError(#[from] TrillianError),

    /// BBError wrapper
    #[error(transparent)]
    BBError(#[from] BBError),

    /// Other error
    #[error("Other Error")]
    Other,
}

impl BulletinBoardService {
    #[instrument(err)]
    pub fn new(
        service_config: BulletinBoardServiceConfig,
    ) -> Result<BulletinBoardService, Error> {
        // check the storage dir structure exists
        let storage_path: PathBuf = service_config.storage_path.clone().into();
        if !storage_path.is_dir() {
            return Err(Error::StoragePathNotADirectory(
                service_config.storage_path.clone(),
            ));
        }
        if !storage_path.join("public").exists() {
            fs::create_dir_all(storage_path.join("public"))?;
        }
        if !storage_path.join("private").exists() {
            fs::create_dir_all(storage_path.join("private"))?;
        }

        let service = BulletinBoardService {
            config: service_config,
            boards: Arc::new(
                fs::read_dir(storage_path.join("public"))?
                    .filter_map(|dir_entry_result| {
                        // Return false if it's not a directory
                        let dir_entry = dir_entry_result.ok()?;
                        return dir_entry.path().is_dir().then_some(dir_entry);
                    })
                    .map(|dir_entry| {
                        let board_uuid: String =
                            dir_entry.file_name().to_string_lossy().into();
                        let board_config =
                            BulletinBoardService::read_board_config(
                                &storage_path,
                                &board_uuid,
                            )?;
                        return Ok((board_uuid, board_config));
                    })
                    .collect::<Result<DashMap<String, BoardConfig>, Error>>()?,
            ),
        };

        return Ok(service);
    }

    #[instrument(err)]
    pub fn read_config() -> Result<BulletinBoardService, Error> {
        let config_path_str =
            env::var("CONFIG_PATH").unwrap_or(String::from("config.json"));

        debug!(config_path_str, "reading config_path");

        let data = fs::read_to_string(config_path_str)?;

        return Self::try_from(data);
    }

    pub fn get_config(&self) -> &BulletinBoardServiceConfig {
        return &self.config;
    }

    fn get_board_dir_path(&self, board_uuid: &String) -> PathBuf {
        let storage_path: PathBuf = self.config.storage_path.clone().into();
        return BulletinBoardService::board_dir_path(&storage_path, board_uuid);
    }

    fn board_dir_path(storage_path: &PathBuf, board_uuid: &String) -> PathBuf {
        let board_dir_path = storage_path.join("public").join(board_uuid);
        return board_dir_path;
    }

    fn get_board_config_path(&self, board_uuid: &String) -> PathBuf {
        let storage_path: PathBuf = self.config.storage_path.clone().into();
        return BulletinBoardService::board_config_path(
            &storage_path,
            board_uuid,
        );
    }

    fn board_config_path(
        storage_path: &PathBuf,
        board_uuid: &String,
    ) -> PathBuf {
        let board_config_path = storage_path
            .join("private")
            .join(&format!("{}.json", board_uuid));
        return board_config_path;
    }

    fn read_board_config(
        storage_path: &PathBuf,
        board_uuid: &String,
    ) -> Result<BoardConfig, Error> {
        let board_config_path =
            BulletinBoardService::board_config_path(storage_path, board_uuid);
        if !board_config_path.is_file() {
            return Err(Error::BoardConfigNotAFile {
                board_config_path,
                board_uuid: board_uuid.clone(),
            });
        }

        let board_config_str = fs::read_to_string(board_config_path)?;
        let mut board_config: BoardConfig =
            serde_json::from_str(&board_config_str)?;

        // Read the checkpoint, to obtain the last sequence id
        let board_dir_path =
            BulletinBoardService::board_dir_path(storage_path, board_uuid);

        // Read board_config.board from the log itself
        let board_entry_path =
            to_seq_path(&board_dir_path, board_config.board_config_sequence_id);
        let board_entry_bytes: Vec<u8> = fs::read(&board_entry_path)?;
        let board_entry = Entry::try_from_slice(&board_entry_bytes)?;
        if let EntryKind::BoardPublicConfig(public_config) = board_entry.kind {
            board_config.board = public_config;
        } else {
            return Err(Error::InvalidEntryType {
                sequence_id: board_config.board_config_sequence_id,
                expected_entry_type: String::from("BoardPublicConfig"),
            });
        }
        debug!(
            "Loaded board config from log entry BoardConfig={:?}",
            board_config.board,
        );

        // Read the board checkpoint to obtain the last_sequence_id
        let read_checkpoint_result = backend_trillian::read_checkpoint(
            /* storage_dir = */ &board_dir_path.to_string_lossy(),
            /* pub_key = */ &board_config.board.public_key,
            /* origin = */ &board_config.board.name,
        )?;

        board_config.last_sequence_id =
            read_checkpoint_result.checkpoint_size - 1;
        debug!(
            "Loading board {}, with last_sequence_id = {}",
            board_config.board.name, board_config.last_sequence_id
        );

        let config_uuid = &board_config.board.uuid;
        if &board_uuid != &config_uuid {
            return Err(Error::MismatchedBoardUuid {
                config_uuid: config_uuid.clone(),
                board_uuid: board_uuid.clone(),
            });
        }
        return Ok(board_config);
    }

    fn write_board_config(
        &self,
        board_uuid: &String,
        board_config: &BoardConfig,
    ) -> Result<(), Error> {
        let board_config_path = self.get_board_config_path(board_uuid);
        let board_config_str = serde_json::to_string_pretty(board_config)?;
        fs::write(board_config_path, board_config_str)?;
        return Ok(());
    }

    /// Returns if the board name is in use, locking only briefly the board list
    #[instrument(err)]
    fn has_board_name(&self, board_name: &String) -> Result<bool, Error> {
        return Ok((*self.boards)
            .iter()
            .find(|pair| &pair.value().board.name == board_name)
            .is_some());
    }
}

#[tonic::async_trait]
impl BulletinBoard for BulletinBoardService {
    #[instrument(err)]
    async fn create_board(
        &self,
        request: Request<CreateBoardRequest>,
    ) -> Result<Response<CreateBoardResponse>, Status> {
        // Check that the signer has create board permissions and the signature
        // is valid
        self.config.permissions.verify_permissions(
            request.get_ref(),
            vec!["CreateBoard".to_string()],
        )?;

        debug!("check board doesn't exist");
        // ensure the board uuid has a valid format
        let board_uuid_original = &request.get_ref().board_uuid;
        let board_uuid_parsed = Uuid::parse_str(board_uuid_original)
            .map_err_status(Code::InvalidArgument)?;
        // Obtain the board uuid in its canonical, hyphenated form
        let board_uuid = board_uuid_parsed.hyphenated().to_string();
        if (*self.boards).contains_key(&board_uuid) {
            return Err(Status::new(
                Code::AlreadyExists,
                format!("Board with id exists: {board_uuid}"),
            ));
        }

        // check board name doesn't exist
        let board_name = &request.get_ref().board_name;
        let board_name_exists = self
            .has_board_name(board_name)
            .map_err_status(Code::Internal)?;
        if board_name_exists {
            return Err(Status::new(
                Code::AlreadyExists,
                format!("Board with name exists: {board_name}"),
            ));
        }
        debug!("DONE checking board doesn't exist: it doesn't");

        // generate board public and private keys
        debug!("Generating keys");
        let keys = backend_trillian::generate_keys(board_name)
            .map_err_status(Code::Internal)?;
        debug!(public_key = keys.public_key, "DONE Generated keys");

        // Generate the board and the board entry
        let board =
            Board::try_from((request.get_ref(), keys.public_key.clone()))
                .map_err(|board_error| {
                    Status::new(Code::InvalidArgument, board_error.to_string())
                })?;
        let entry = Entry::try_from((request.get_ref(), &board))?;

        // sanity check: the entry signature should verify. Note that the
        // signature comes from the CreateBoardRequest signature - it just
        // generates a Board (without the public key) calls
        // board.verify_signature() to that.
        entry.verify_signature()?;

        let mut board_config = BoardConfig {
            private_key: keys.private_key.clone(),
            last_sequence_id: 0,
            board_config_sequence_id: 0,
            board: board.clone(),
        };

        // Create the board in storage directory
        let board_dir_path = self.get_board_dir_path(&board.uuid);
        if board_dir_path.exists() {
            return Err(Status::new(
                Code::Internal,
                format!(
                    "Creating Board directory {board_dir_path:?}: it already \
                    exists"
                ),
            ));
        }

        // Create the trillian board directory structure
        let _create_integrate_result = backend_trillian::integrate(
            /* storage_dir = */ &board_dir_path.to_string_lossy(),
            /* initialise = */ true,
            /* pub_key = */ &keys.public_key,
            /* priv_key = */ &keys.private_key,
            /* origin = */ &board.name,
        )
        .map_err_status(Code::Internal)?;

        // Write board config files
        let board_config_path = self.get_board_config_path(&board.uuid);
        if board_config_path.exists() {
            return Err(Status::new(
                Code::Internal,
                format!(
                    "Creating Board config {board_config_path:?}: it already \
                     exists"
                ),
            ));
        }
        self.write_board_config(&board.uuid, &board_config)
            .map_err_status(Code::Internal)?;

        // Add the Board Config as the first entry to the board
        let indicated_sequence_id = board_config.last_sequence_id;

        // Create a directory inside of `std::env::temp_dir()`. It will be
        // removed when it goes out of scope
        let tmp_dir = TempDir::new()?;

        // Here we hash and write entries into files. We generally do this
        // before obtaining the lock on the board and improve parallelization,
        // although in this case we don't acquire the lock since the board is
        // not yet added to the board list
        let temp_file_path = tmp_dir.path().join("entry");
        let trillian_entry =
            entry.into_trillian_entry(&temp_file_path).map_err(|_| {
                Status::new(Code::Internal, "Error serializing the board entry")
            })?;
        let sequence_result = backend_trillian::sequence(
            /* storage_dir = */ &board_dir_path.to_string_lossy(),
            /* entries = */ vec![trillian_entry],
            /* pub_key = */ &keys.public_key,
            /* origin = */ &board.name,
        )
        .map_err_status(Code::Internal)?;
        let obtained_entry_sequence_id =
            sequence_result.entries_sequence_ids[0];

        if indicated_sequence_id != obtained_entry_sequence_id {
            return Err(Status::new(
                Code::Internal,
                format!(
                    "Error sequencing entries - unexpected 
                    sequence id {obtained_entry_sequence_id}, expected 
                    {indicated_sequence_id}"
                ),
            ));
        }

        // Integrate the sequenced items
        let integrate_result = backend_trillian::integrate(
            /* storage_dir = */ &board_dir_path.to_string_lossy(),
            /* initialise = */ false,
            /* pub_key = */ &keys.public_key,
            /* priv_key = */ &keys.private_key,
            /* origin = */ &board.name,
        )
        .map_err_status(Code::Internal)?;

        let integrated_sequence_id = integrate_result.checkpoint_size - 1;
        if obtained_entry_sequence_id != integrated_sequence_id {
            return Err(Status::new(
                Code::Internal,
                format!(
                    "Error integrating entries - unexpected 
                    sequence id {integrated_sequence_id}, expected 
                    {obtained_entry_sequence_id}"
                ),
            ));
        }

        // update last_sequence_id, note that this doesn't require saving to
        // disk
        board_config.last_sequence_id = obtained_entry_sequence_id;
        board_config.board_config_sequence_id = obtained_entry_sequence_id;

        // Add the board to the in-memory store
        debug!("inserting the board in the in-memory store");
        self.boards.insert(board.uuid.clone(), board_config);
        debug!("DONE inserting the board in the in-memory store");

        let response = Response::new(CreateBoardResponse {
            bulletin_board: Some(board),
            checkpoint: Some(Checkpoint {
                origin: integrate_result.checkpoint_origin,
                size: integrate_result.checkpoint_size,
                hash: integrate_result.checkpoint_hash,
            }),
        });
        debug!("response = {response:?}");
        return Ok(response);
    }

    #[instrument(err, ret)]
    async fn list_boards(
        &self,
        request: Request<ListBoardsRequest>,
    ) -> Result<Response<ListBoardsResponse>, Status> {
        let is_archived = &request.get_ref().is_archived;
        let board_name = &request.get_ref().board_name;
        let board_uuid = &request.get_ref().board_uuid;

        return Ok(Response::new(ListBoardsResponse {
            boards: (*self.boards)
                .iter()
                .filter(
                    #[rustfmt::skip]
                    |pair| (
                        (
                            is_archived.is_none() ||
                            is_archived.contains(&pair.value().board.is_archived)
                        ) && (
                            board_name.is_none() ||
                            board_name.contains(&pair.value().board.name)
                        ) && (
                            board_uuid.is_none() ||
                            board_uuid.contains(&pair.value().board.uuid)
                        )
                    )
                )
                .map(|board_config| ListBoardItem {
                    board_last_sequence_id: board_config.last_sequence_id,
                    board: Some(board_config.board.clone()),
                })
                .collect(),
        }));
    }

    #[instrument(err)]
    async fn list_entries(
        &self,
        request: Request<ListEntriesRequest>,
    ) -> Result<Response<ListEntriesResponse>, Status> {
        let message = request.into_inner();
        let board_uuid = &message.board_uuid;
        let start_sequence_id = message.start_sequence_id;

        // get required info from the board config while at the same time
        // limiting the lock in the board to improve performance through
        // parallelization
        let last_sequence_id = {
            let mut board_pair =
                (*self.boards).get_mut(board_uuid).ok_or_else(|| {
                    Status::new(
                        Code::NotFound,
                        format!("Board with uuid={board_uuid} doesn't exist"),
                    )
                })?;
            let board_config = board_pair.value_mut();

            // if already archived, finish here
            if board_config.board.is_archived == true {
                return Err(Status::new(
                    Code::InvalidArgument,
                    format!("Board with uuid={board_uuid} is archived"),
                ));
            }
            board_config.last_sequence_id
        };

        let storage_path: PathBuf = self.config.storage_path.clone().into();
        let board_dir_path =
            BulletinBoardService::board_dir_path(&storage_path, board_uuid);
        debug!(
            "listing entries from {} to {}",
            start_sequence_id, last_sequence_id
        );
        return Ok(Response::new(ListEntriesResponse {
            board_last_sequence_id: last_sequence_id,
            board_entries: (start_sequence_id..last_sequence_id + 1)
                .map(|sequence_id| {
                    debug!("listing entry with sequence_id={sequence_id}");
                    // Read board_config.board from the log itself
                    let board_entry_path =
                        to_seq_path(&board_dir_path, sequence_id);

                    let board_entry_bytes: Vec<u8> =
                        fs::read(&board_entry_path).map_err(|_| {
                            format!(
                                "Board config entry at \
                                path={board_entry_path:?} can't be read"
                            )
                        })?;
                    let board_entry = Entry::try_from_slice(&board_entry_bytes)
                        .map_err(|_| {
                            format!(
                                "Board entry at path={board_entry_path:?} \
                                can't be read"
                            )
                        })?;
                    return Ok(board_entry.into_board_entry(sequence_id));
                })
                .collect::<Result<Vec<BoardEntry>, String>>()
                .map_err_status(Code::Internal)?,
        }));
    }

    /// Adds an entry, sequencing it and then integrating it into the log.
    /// Regarding the logging with the `#instrument` decorator, note we skip the
    /// request since it could be huge.
    #[instrument(skip(request), err)]
    async fn add_entries(
        &self,
        request: Request<AddEntriesRequest>,
    ) -> Result<Response<AddEntriesResponse>, Status> {
        let message = request.into_inner();
        let board_uuid = &message.board_uuid;
        let entries = message.entries;

        if entries.len() == 0 {
            return Err(Status::new(
                Code::InvalidArgument,
                format!(
                    "Received 0 entries to add to board with uuid={board_uuid}"
                ),
            ));
        }

        // get required info from the board config to verify permissions while
        // at the same time limiting the lock in the board to improve
        // performance through parallelization
        let permissions = {
            let mut board_pair =
                (*self.boards).get_mut(board_uuid).ok_or_else(|| {
                    Status::new(
                        Code::NotFound,
                        format!("Board with uuid={board_uuid} doesn't exist"),
                    )
                })?;
            let board_config = board_pair.value_mut();

            // if already archived, finish here
            if board_config.board.is_archived == true {
                return Err(Status::new(
                    Code::InvalidArgument,
                    format!("Board with uuid={board_uuid} is archived"),
                ));
            }
            board_config.board.permissions.clone().ok_or_else(|| {
                Status::new(
                    Code::Internal,
                    format!(
                        "Board with uuid={board_uuid} have permissions=None"
                    ),
                )
            })?
        };

        // Check that the request signer of each requested new data entry has
        // AddEntries permissions and the signature is valid
        for new_entry_request in entries.iter() {
            permissions.verify_permissions(
                new_entry_request,
                vec!["AddEntries".to_string()],
            )?;
        }

        // Create a directory inside of `std::env::temp_dir()`. It will be
        // removed when it goes out of scope
        let tmp_dir = TempDir::new()?;

        // Here we hash and write entries into files. We do this before
        // obtaining the lock on the board, to improve parallelization and
        // reduce the time
        let data_entries: Vec<(TrillianEntry, BoardEntry)> = entries
            .into_iter()
            .enumerate()
            .map(|(index, req_entry)| {
                let temp_file_path = tmp_dir.path().join(index.to_string());
                let entry = Entry::from(req_entry);
                let board_entry = BoardEntry {
                    // this is not really the resulting sequence_id, but
                    // instead the index. Just a convenient way to store it
                    sequence_id: (index as u64),
                    kind: Some(board_entry::Kind::EntryData(BoardEntryData {
                        data: None,
                    })),
                    timestamp: entry.timestamp,
                    metadata: entry.metadata.clone(),
                    signer_public_key: entry.signer_public_key.clone(),
                    signature: entry.signature.clone(),
                };
                let trillian_entry = entry
                    .into_trillian_entry(&temp_file_path)
                    .map_err(|_| {
                        Status::new(
                            Code::Internal,
                            "Error serializing the board entry",
                        )
                    })?;
                Ok((trillian_entry, board_entry))
            })
            .collect::<Result<Vec<(TrillianEntry, BoardEntry)>, Status>>()?;

        let (hashed_entries, expected_entries): (_, Vec<BoardEntry>) =
            data_entries.into_iter().unzip();

        // get the board config
        let mut board_pair =
            (*self.boards).get_mut(board_uuid).ok_or_else(|| {
                Status::new(
                    Code::NotFound,
                    format!("Board with uuid={board_uuid} doesn't exist"),
                )
            })?;
        let board_config = board_pair.value_mut();

        // if already archived, finish here
        if board_config.board.is_archived == true {
            return Err(Status::new(
                Code::InvalidArgument,
                format!("Board with uuid={board_uuid} is archived"),
            ));
        }
        let board_dir_path = self.get_board_dir_path(&board_config.board.uuid);

        let sequence_result = backend_trillian::sequence(
            /* storage_dir = */
            &board_dir_path.to_string_lossy(),
            /* entries = */ hashed_entries,
            /* pub_key = */ &board_config.board.public_key,
            /* origin = */ &board_config.board.name,
        )
        .map_err_status(Code::Internal)?;

        // check the sequenced entries match expectations and convert them to
        // the resulting data
        let sequenced_entries: Vec<BoardEntry> = expected_entries
            .into_iter()
            .zip(sequence_result.entries_sequence_ids)
            .map(|pair| {
                let (expected_entry, obtained_entry_sequence_id) = pair;

                let expected_sequence_id = board_config.last_sequence_id
                    + 1
                    + expected_entry.sequence_id;

                if obtained_entry_sequence_id != expected_sequence_id {
                    return Err(Status::new(
                        Code::Internal,
                        format!(
                            "Error integrating entries - unexpected 
                            sequence id {obtained_entry_sequence_id}, expected 
                            {expected_sequence_id}"
                        ),
                    ));
                }
                return Ok(BoardEntry {
                    sequence_id: obtained_entry_sequence_id,
                    ..expected_entry
                });
            })
            .collect::<Result<Vec<BoardEntry>, Status>>()?;

        // Integrate the sequenced items
        let integrate_result = backend_trillian::integrate(
            /* storage_dir = */ &board_dir_path.to_string_lossy(),
            /* initialise = */ false,
            /* pub_key = */ &board_config.board.public_key,
            /* priv_key = */ &board_config.private_key,
            /* origin = */ &board_config.board.name,
        )
        .map_err_status(Code::Internal)?;

        // verify consistency of the checkpoint size
        let last_entry = sequenced_entries.last().unwrap();
        let last_entry_sequence_id = last_entry.sequence_id;
        let checkpoint_last_sequence_id = integrate_result.checkpoint_size - 1;
        if last_entry_sequence_id != checkpoint_last_sequence_id {
            return Err(Status::new(
                Code::Internal,
                format!(
                    "Error integrating entries - unexpected checkpoint 
                    sequence id {checkpoint_last_sequence_id}, expected 
                    {last_entry_sequence_id}"
                ),
            ));
        }

        // update last_sequence_id, note that this doesn't require saving to
        // disk
        board_config.last_sequence_id = last_entry_sequence_id;

        return Ok(Response::new(AddEntriesResponse {
            checkpoint: Some(Checkpoint {
                origin: integrate_result.checkpoint_origin,
                size: integrate_result.checkpoint_size,
                hash: integrate_result.checkpoint_hash,
            }),
            entries: sequenced_entries,
        }));
    }

    #[instrument(err)]
    async fn modify_board(
        &self,
        request: Request<ModifyBoardRequest>,
    ) -> Result<Response<ModifyBoardResponse>, Status> {
        let board_uuid = &request.get_ref().board_uuid;

        // get the board list, acquiring the lock on them too for the scope of
        // this function
        // get the board config
        let mut board_pair =
            (self.boards).get_mut(board_uuid).ok_or_else(|| {
                Status::new(
                    Code::NotFound,
                    format!("Board with uuid={board_uuid} doesn't exist"),
                )
            })?;
        let mut board_config = board_pair.value_mut();

        let permissions =
            board_config.board.permissions.clone().ok_or_else(|| {
                Status::new(
                    Code::Internal,
                    format!(
                        "Board with uuid={board_uuid} have permissions=None"
                    ),
                )
            })?;

        permissions.verify_permissions(
            request.get_ref(),
            vec!["ModifyBoard".to_string()],
        )?;

        // if already archived, finish here
        if board_config.board.is_archived && request.get_ref().is_archived {
            return Err(Status::new(
                Code::InvalidArgument,
                format!("Board with uuid={board_uuid} is archived"),
            ));
        }

        // Generate the board and the board entry
        let updated_board = Board::try_from((
            request.get_ref(),
            board_config.board.public_key.clone(),
        ))
        .map_err(|board_error| {
            Status::new(Code::InvalidArgument, board_error.to_string())
        })?;
        let entry = Entry::try_from((request.get_ref(), &updated_board))?;

        // sanity check: the entry signature should verify. Note that the
        // signature comes from the ModifyBoardRequest signature - it just
        // generates a Board (without the public key) calls
        // board.verify_signature() to that.
        entry.verify_signature()?;

        // Add the Board Config as the last entry to the board
        let board_dir_path = self.get_board_dir_path(&board_config.board.uuid);
        let indicated_sequence_id = board_config.last_sequence_id + 1;
        debug!(entry.timestamp, "entry.timestamp");

        // Create a directory inside of `std::env::temp_dir()`. It will be
        // removed when it goes out of scope
        let tmp_dir = TempDir::new()?;

        // Here we hash and write entries into files. We do this before
        // obtaining the lock on the board, to improve parallelization and
        // reduce the time
        let temp_file_path = tmp_dir.path().join("new_entry");
        let trillian_entry =
            entry.into_trillian_entry(&temp_file_path).map_err(|_| {
                Status::new(Code::Internal, "Error serializing the board entry")
            })?;
        let sequence_result = backend_trillian::sequence(
            /* storage_dir = */ &board_dir_path.to_string_lossy(),
            /* entries = */ vec![trillian_entry],
            /* pub_key = */ &board_config.board.public_key,
            /* origin = */ &board_config.board.name,
        )
        .map_err_status(Code::Internal)?;
        let obtained_entry_sequence_id =
            sequence_result.entries_sequence_ids[0];

        if obtained_entry_sequence_id != indicated_sequence_id {
            return Err(Status::new(
                Code::Internal,
                format!(
                    "Error integrating entries - unexpected checkpoint 
                    sequence id {obtained_entry_sequence_id}, expected 
                    {indicated_sequence_id}"
                ),
            ));
        }

        // Integrate the sequenced items
        let integrate_result = backend_trillian::integrate(
            /* storage_dir = */ &board_dir_path.to_string_lossy(),
            /* initialise = */ false,
            /* pub_key = */ &board_config.board.public_key,
            /* priv_key = */ &board_config.private_key,
            /* origin = */ &board_config.board.name,
        )
        .map_err_status(Code::Internal)?;

        let checkpoint_last_sequence_id = integrate_result.checkpoint_size - 1;
        if obtained_entry_sequence_id != checkpoint_last_sequence_id {
            return Err(Status::new(
                Code::Internal,
                format!(
                    "Error integrating entries - unexpected checkpoint 
                    sequence id {checkpoint_last_sequence_id}, expected 
                    {obtained_entry_sequence_id}"
                ),
            ));
        }

        // update board config, note that this doesn't require saving to
        // disk
        board_config.board = updated_board;
        board_config.last_sequence_id = obtained_entry_sequence_id;
        board_config.board_config_sequence_id = obtained_entry_sequence_id;

        // save the updated board config to disk
        self.write_board_config(board_uuid, &board_config)
            .map_err_status(Code::Internal)?;

        return Ok(Response::new(ModifyBoardResponse {
            board: Some(board_config.board.clone()),
        }));
    }
}

#[cfg(test)]
mod tests {
    use rand::{distributions::Standard, thread_rng, Rng};
    use std::fs;
    use std::sync::Once;
    use strand::rnd::StrandRng;
    use strand::signature::{
        StrandSignaturePk as PublicKey, StrandSignatureSk as SecretKey,
    };
    use tempfile::Builder;
    use tonic::Request;
    use tracing::debug;
    use uuid::Uuid;

    use crate::service::{
        BulletinBoard, BulletinBoardService, BulletinBoardServiceConfig,
    };
    use crate::signature::Signable;
    use crate::{
        board_entry, util::init_log as init_log_once, AddEntriesRequest,
        CreateBoardRequest, ListBoardsRequest, ListEntriesRequest,
        ModifyBoardRequest, NewDataEntry, Permissions, Role, User, UserRole,
    };

    static INIT: Once = Once::new();
    lazy_static! {
        // Service user secret (and public) key, used for creating boards and
        // also for executing board actions
        static ref SECRET_KEY: SecretKey = {
            let mut generator = StrandRng;
            SecretKey::new(&mut generator)
        };
    }

    /// Initialization function
    fn init_log() {
        INIT.call_once(|| {
            // This can only be called once or it would fail
            init_log_once().unwrap();
        });
    }

    fn setup() -> (tempfile::TempDir, BulletinBoardService) {
        init_log();

        let storage_dir = Builder::new().prefix("bb-test").tempdir().unwrap();
        let service_config = BulletinBoardServiceConfig {
            storage_path: storage_dir.path().display().to_string(),
            server_url: "127.0.0.1:1322".to_string(),
            permissions: Permissions {
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
            },
        };

        let bulletin_board_service =
            BulletinBoardService::new(service_config).unwrap();

        return (storage_dir, bulletin_board_service);
    }

    fn public_key() -> String {
        PublicKey::from(&*SECRET_KEY).clone().try_into().unwrap()
    }

    // Basic board permissions
    fn basic_permisssions() -> Permissions {
        Permissions {
            users: vec![User {
                name: "admin".into(),
                public_key: public_key(),
                ..Default::default()
            }],
            roles: vec![Role {
                name: "admins".into(),
                permissions: vec!["AddEntries".into(), "ModifyBoard".into()],
                ..Default::default()
            }],
            user_roles: vec![UserRole {
                user_name: "admin".into(),
                role_names: vec!["admins".into()],
            }],
            ..Default::default()
        }
    }

    /// Simple test to create a board successfully
    #[tokio::test]
    async fn test_create_board() {
        let (storage_dir, bulletin_board_service) = setup();

        let board_uuid = Uuid::new_v4().to_string();
        let request = Request::new(
            CreateBoardRequest {
                board_uuid: board_uuid.clone(),
                board_name: "test_board".into(),
                permissions: Some(basic_permisssions()),
                ..Default::default()
            }
            .sign(&*SECRET_KEY)
            .unwrap(),
        );
        let create_result = bulletin_board_service.create_board(request).await;
        assert!(create_result.is_ok());

        // verify the returning output of the call is what we expect
        let board = create_result
            .unwrap()
            .get_ref()
            .bulletin_board
            .as_ref()
            .unwrap()
            .clone();
        assert_eq!(board.uuid, board_uuid);
        assert_eq!(board.name, String::from("test_board"));
        assert!(board.description.is_none());
        assert_eq!(board.is_archived, false);

        // clean up
        fs::remove_dir_all(storage_dir).unwrap();
    }

    /// Simple test creating 10 different boards
    #[tokio::test]
    async fn test_multiple_create_board() {
        let (storage_dir, bulletin_board_service) = setup();

        for i in 0..10 {
            let board_uuid = Uuid::new_v4().to_string();
            let request = Request::new(
                CreateBoardRequest {
                    board_uuid: board_uuid.clone(),
                    board_name: format!("test_board_{i}"),
                    permissions: Some(basic_permisssions()),
                    ..Default::default()
                }
                .sign(&*SECRET_KEY)
                .unwrap(),
            );
            debug!("requesting create_board {board_uuid}");
            let create_result =
                bulletin_board_service.create_board(request).await;
            assert!(create_result.is_ok());
        }

        // clean up
        fs::remove_dir_all(storage_dir).unwrap();
    }

    /// Test that explores different ways creating a board can fail
    #[tokio::test]
    async fn test_failures_creating_board() {
        let (storage_dir, bulletin_board_service) = setup();

        // First lets create one board
        let board_uuid = Uuid::new_v4().to_string();
        let request = Request::new(
            CreateBoardRequest {
                board_uuid: board_uuid.clone(),
                board_name: "test_board".into(),
                permissions: Some(basic_permisssions()),
                ..Default::default()
            }
            .sign(&*SECRET_KEY)
            .unwrap(),
        );
        let create_result = bulletin_board_service.create_board(request).await;
        assert!(create_result.is_ok());

        // If we try to create a board with the same name, it fails
        let request_duplicate = Request::new(
            CreateBoardRequest {
                board_uuid: Uuid::new_v4().to_string(),
                board_name: "test_board".into(),
                permissions: Some(basic_permisssions()),
                ..Default::default()
            }
            .sign(&*SECRET_KEY)
            .unwrap(),
        );
        let create_result_duplicate =
            bulletin_board_service.create_board(request_duplicate).await;
        assert!(create_result_duplicate.is_err());

        // If we try to create a board with the same uuid, it also fails
        let request_duplicate_uuid = Request::new(
            CreateBoardRequest {
                board_uuid: board_uuid.clone(),
                board_name: "test_board_2".into(),
                permissions: Some(basic_permisssions()),
                ..Default::default()
            }
            .sign(&*SECRET_KEY)
            .unwrap(),
        );
        let create_result_duplicate_uuid = bulletin_board_service
            .create_board(request_duplicate_uuid)
            .await;
        assert!(create_result_duplicate_uuid.is_err());

        // These are a set of different invalid board names
        let invalid_board_names = vec![
            "",           // invalid because it's empty
            "some name",  // includes whitespace
            "some\tname", // includes whitespace
            "some+name",  // includes plus sign
        ];
        for invalid_board_name in invalid_board_names {
            let request_invalid_name = Request::new(
                CreateBoardRequest {
                    board_uuid: Uuid::new_v4().to_string(),
                    board_name: String::from(invalid_board_name),
                    permissions: Some(basic_permisssions()),
                    ..Default::default()
                }
                .sign(&*SECRET_KEY)
                .unwrap(),
            );
            let create_result_invalid_name = bulletin_board_service
                .create_board(request_invalid_name)
                .await;
            assert!(create_result_invalid_name.is_err());
        }

        // clean up
        fs::remove_dir_all(storage_dir).unwrap();
    }

    /// Test that adds a small entry
    #[tokio::test]
    async fn test_add_small_entry() {
        let (storage_dir, bulletin_board_service) = setup();

        // First lets create a board
        let board_uuid = Uuid::new_v4().to_string();
        let request = Request::new(
            CreateBoardRequest {
                board_uuid: board_uuid.clone(),
                board_name: "test_board".into(),
                permissions: Some(basic_permisssions()),
                ..Default::default()
            }
            .sign(&*SECRET_KEY)
            .unwrap(),
        );
        let create_result = bulletin_board_service.create_board(request).await;
        assert!(create_result.is_ok());

        // Let's add an entry
        let entry_size = 1;
        let entry_data = (0..entry_size).map(|i| i as u8).collect::<Vec<u8>>();
        let request = Request::new(AddEntriesRequest {
            board_uuid: board_uuid.clone(),
            entries: vec![NewDataEntry {
                data: entry_data.clone(),
                ..Default::default()
            }
            .sign(&*SECRET_KEY)
            .unwrap()],
        });
        let add_result = bulletin_board_service.add_entries(request).await;
        assert!(add_result.is_ok());

        // Check result is what we expect
        let entry = add_result.unwrap().get_ref().entries[0].clone();
        // Entry sequence is not zero because when creating the board, a first
        // entry with the board config is automatically added
        assert_eq!(entry.sequence_id, 1);
        assert!(matches!(
            entry.kind.clone().unwrap(),
            board_entry::Kind::EntryData { .. }
        ));
        if let board_entry::Kind::EntryData(data) = entry.kind.unwrap() {
            // the data is not returned to save network resources
            assert!(data.data.is_none());
        } else {
            panic!("entry data is invalid kind - this cannot really happen");
        }

        // clean up
        fs::remove_dir_all(storage_dir).unwrap();
    }

    /// Test that adds multiple entries
    #[tokio::test]
    async fn test_add_multiple_entries() {
        let (storage_dir, bulletin_board_service) = setup();

        // First lets create a board
        let board_uuid = Uuid::new_v4().to_string();
        let request = Request::new(
            CreateBoardRequest {
                board_uuid: board_uuid.clone(),
                board_name: "test_board".into(),
                permissions: Some(basic_permisssions()),
                ..Default::default()
            }
            .sign(&*SECRET_KEY)
            .unwrap(),
        );
        let create_result = bulletin_board_service.create_board(request).await;
        assert!(create_result.is_ok());

        // Let's add entries
        let entry_size = 100;
        let num_batches = 10;
        let batch_size = 10;
        let mut rng = thread_rng();
        for i in 0..num_batches {
            let request = Request::new(AddEntriesRequest {
                board_uuid: board_uuid.clone(),
                entries: (0..batch_size)
                    .map(|_| {
                        NewDataEntry {
                            // note that since duplicated entries are not
                            // allowed we
                            // ensure entry data is unique
                            data: (&mut rng)
                                .sample_iter(Standard)
                                .take(entry_size)
                                .collect(),
                            ..Default::default()
                        }
                        .sign(&*SECRET_KEY)
                        .unwrap()
                    })
                    .collect::<Vec<NewDataEntry>>(),
            });
            // adding another batch of entries with a different timestamp, as we
            // waited 10 milliseconds
            let ten_millis = std::time::Duration::from_millis(10);
            std::thread::sleep(ten_millis);

            let add_result = bulletin_board_service.add_entries(request).await;
            assert!(add_result.is_ok());

            // Check result is what we expect
            let entries = add_result.unwrap().get_ref().entries.clone();
            assert!(entries.len() == batch_size);
            let entry = &entries[0];
            // Entry sequence is not zero because when creating the board, a
            // first entry with the board config is automatically
            // added
            assert_eq!(entry.sequence_id, 1 + ((i * num_batches) as u64));
        }

        // clean up
        fs::remove_dir_all(storage_dir).unwrap();
    }

    /// Test that adds a megabyte entry
    #[tokio::test]
    async fn test_add_megabyte_entry() {
        let (storage_dir, bulletin_board_service) = setup();

        // First lets create a board
        let board_uuid = Uuid::new_v4().to_string();
        let request = Request::new(
            CreateBoardRequest {
                board_uuid: board_uuid.clone(),
                board_name: "test_board".into(),
                permissions: Some(basic_permisssions()),
                ..Default::default()
            }
            .sign(&*SECRET_KEY)
            .unwrap(),
        );
        let create_result = bulletin_board_service.create_board(request).await;
        assert!(create_result.is_ok());

        // Let's add an entry
        let entry_size = 1024 * 1024;
        let request = Request::new(AddEntriesRequest {
            board_uuid: board_uuid.clone(),
            entries: vec![NewDataEntry {
                data: (0..entry_size).map(|i| i as u8).collect::<Vec<u8>>(),
                ..Default::default()
            }
            .sign(&*SECRET_KEY)
            .unwrap()],
        });
        let add_result = bulletin_board_service.add_entries(request).await;
        assert!(add_result.is_ok());

        // Check result is what we expect
        let entry = add_result.unwrap().get_ref().entries[0].clone();
        // Entry sequence is not zero because when creating the board, a first
        // entry with the board config is automatically added
        assert_eq!(entry.sequence_id, 1);
        assert!(matches!(
            entry.kind.unwrap(),
            board_entry::Kind::EntryData { .. }
        ));

        // clean up
        fs::remove_dir_all(storage_dir).unwrap();
    }

    /// Test that tests different error scenarios while creating an entry
    #[tokio::test]
    async fn test_failures_add_entries() {
        let (storage_dir, bulletin_board_service) = setup();

        // First lets create a board
        let board_uuid = Uuid::new_v4().to_string();
        let request = Request::new(
            CreateBoardRequest {
                board_uuid: board_uuid.clone(),
                board_name: "test_board".into(),
                permissions: Some(basic_permisssions()),
                ..Default::default()
            }
            .sign(&*SECRET_KEY)
            .unwrap(),
        );
        let create_result = bulletin_board_service.create_board(request).await;
        assert!(create_result.is_ok());

        // Let's add an entry
        let entry_size = 1024 * 1024;
        let entry_data = (0..entry_size).map(|i| i as u8).collect::<Vec<u8>>();
        let request = Request::new(AddEntriesRequest {
            board_uuid: board_uuid.clone(),
            entries: vec![NewDataEntry {
                data: entry_data.clone(),
                ..Default::default()
            }
            .sign(&*SECRET_KEY)
            .unwrap()],
        });
        let add_result = bulletin_board_service.add_entries(request).await;
        assert!(add_result.is_ok());

        // adding another entry with the same data should not fail because each
        // time it has a different timestamp, as we waited 10 milliseconds
        let ten_millis = std::time::Duration::from_millis(10);
        std::thread::sleep(ten_millis);
        let request_duplicate = Request::new(AddEntriesRequest {
            board_uuid: board_uuid.clone(),
            entries: vec![NewDataEntry {
                data: entry_data.clone(),
                ..Default::default()
            }
            .sign(&*SECRET_KEY)
            .unwrap()],
        });
        let add_result_duplicate =
            bulletin_board_service.add_entries(request_duplicate).await;
        assert!(add_result_duplicate.is_ok());

        // Now let's try to add a new entry in a non-existant board - should
        // fail too
        let request_invalid_board = Request::new(AddEntriesRequest {
            board_uuid: Uuid::new_v4().to_string(),
            entries: vec![NewDataEntry {
                data: (0..10).map(|i| i as u8).collect::<Vec<u8>>(),
                ..Default::default()
            }
            .sign(&*SECRET_KEY)
            .unwrap()],
        });
        let add_result_invalid_board = bulletin_board_service
            .add_entries(request_invalid_board)
            .await;
        assert!(add_result_invalid_board.is_err());

        // clean up
        fs::remove_dir_all(storage_dir).unwrap();
    }

    /// Test list boards works as expected - simple case
    #[tokio::test]
    async fn test_simple_list_boards() {
        let (storage_dir, bulletin_board_service) = setup();

        // Listing boards should return an empty list since we didn't create
        // any board yet
        let list_boards_result = bulletin_board_service
            .list_boards(Request::new(Default::default()))
            .await;
        assert!(list_boards_result.is_ok());
        assert!(list_boards_result.unwrap().get_ref().boards.is_empty());

        // Let's create a board
        let board_uuid = Uuid::new_v4().to_string();
        let request = Request::new(
            CreateBoardRequest {
                board_uuid: board_uuid.clone(),
                board_name: "test_board".into(),
                permissions: Some(basic_permisssions()),
                ..Default::default()
            }
            .sign(&*SECRET_KEY)
            .unwrap(),
        );
        let create_result = bulletin_board_service.create_board(request).await;
        assert!(create_result.is_ok());

        // Listing boards should return an empty list since we didn't create
        // any board yet
        let list_boards_result = bulletin_board_service
            .list_boards(Request::new(Default::default()))
            .await;
        assert!(list_boards_result.is_ok());
        let result_unwrap = list_boards_result.unwrap();
        let boards = &result_unwrap.get_ref().boards;
        assert_eq!(boards.len(), 1);
        assert_eq!(boards[0].board_last_sequence_id, 0 as u64);
        let board = boards[0].board.as_ref().unwrap();
        assert_eq!(board.uuid, board_uuid);
        assert_eq!(board.name, String::from("test_board"));
        assert!(board.description.is_none());
        assert_eq!(board.is_archived, false);

        // Adding an entry updates the board_last_sequence_id
        let entry_size = 1;
        let entry_data = (0..entry_size).map(|i| i as u8).collect::<Vec<u8>>();
        let request = Request::new(AddEntriesRequest {
            board_uuid: board_uuid.clone(),
            entries: vec![NewDataEntry {
                data: entry_data.clone(),
                ..Default::default()
            }
            .sign(&*SECRET_KEY)
            .unwrap()],
        });
        let add_result = bulletin_board_service.add_entries(request).await;
        assert!(add_result.is_ok());

        let list_boards_result = bulletin_board_service
            .list_boards(Request::new(Default::default()))
            .await;
        assert!(list_boards_result.is_ok());
        let result_unwrap = list_boards_result.unwrap();
        let boards = &result_unwrap.get_ref().boards;
        assert_eq!(boards.len(), 1);
        assert_eq!(boards[0].board_last_sequence_id, 1 as u64);
        let board = boards[0].board.as_ref().unwrap();
        assert_eq!(board.uuid, board_uuid);
        assert_eq!(board.name, String::from("test_board"));
        assert!(board.description.is_none());
        assert_eq!(board.is_archived, false);

        // clean up
        fs::remove_dir_all(storage_dir).unwrap();
    }

    /// Test list boards works as expected - when filtering
    #[tokio::test]
    async fn test_filter_list_boards() {
        let (storage_dir, bulletin_board_service) = setup();

        // Let's create a two boards, one archived
        let board1_uuid = Uuid::new_v4().to_string();
        let create_request = Request::new(
            CreateBoardRequest {
                board_uuid: board1_uuid.clone(),
                board_name: "test_board".into(),
                permissions: Some(basic_permisssions()),
                ..Default::default()
            }
            .sign(&*SECRET_KEY)
            .unwrap(),
        );
        let create_result =
            bulletin_board_service.create_board(create_request).await;
        assert!(create_result.is_ok());

        let board2_uuid = Uuid::new_v4().to_string();
        let create_request2 = Request::new(
            CreateBoardRequest {
                board_uuid: board2_uuid.clone(),
                board_name: "test_board2".into(),
                permissions: Some(basic_permisssions()),
                ..Default::default()
            }
            .sign(&*SECRET_KEY)
            .unwrap(),
        );
        let create2_result =
            bulletin_board_service.create_board(create_request2).await;
        assert!(create2_result.is_ok());
        let board = create2_result
            .unwrap()
            .get_ref()
            .bulletin_board
            .clone()
            .unwrap();

        let archive_result = bulletin_board_service
            .modify_board(Request::new(
                ModifyBoardRequest {
                    is_archived: true,
                    ..ModifyBoardRequest::from(board)
                }
                .sign(&*SECRET_KEY)
                .unwrap(),
            ))
            .await;
        assert!(archive_result.is_ok());

        // Listing boards should return two boards by default
        let list_boards_result = bulletin_board_service
            .list_boards(Request::new(Default::default()))
            .await;
        assert!(list_boards_result.is_ok());
        let result_unwrap = list_boards_result.unwrap();
        let boards = &result_unwrap.get_ref().boards;
        assert_eq!(boards.len(), 2);

        // Listing filtering for board1 uuid should work
        let list_boards_result = bulletin_board_service
            .list_boards(Request::new(ListBoardsRequest {
                board_uuid: Some(board1_uuid.clone()),
                ..Default::default()
            }))
            .await;
        assert!(list_boards_result.is_ok());
        let result_unwrap = list_boards_result.unwrap();
        let boards = &result_unwrap.get_ref().boards;
        assert_eq!(boards.len(), 1);
        let board = boards[0].board.as_ref().unwrap();
        assert_eq!(board.uuid, board1_uuid);
        assert_eq!(board.name, String::from("test_board"));
        assert!(board.description.is_none());
        assert_eq!(board.is_archived, false);

        // Same for board2
        let list_boards_result = bulletin_board_service
            .list_boards(Request::new(ListBoardsRequest {
                board_uuid: Some(board2_uuid.clone()),
                ..Default::default()
            }))
            .await;
        assert!(list_boards_result.is_ok());
        let result_unwrap = list_boards_result.unwrap();
        let boards = &result_unwrap.get_ref().boards;
        assert_eq!(boards.len(), 1);
        let board = boards[0].board.as_ref().unwrap();
        assert_eq!(board.uuid, board2_uuid);
        assert_eq!(board.name, String::from("test_board2"));
        assert!(board.description.is_none());
        assert_eq!(board.is_archived, true);

        // If we filter for an inexistant board uuid, list is empty
        let list_boards_result = bulletin_board_service
            .list_boards(Request::new(ListBoardsRequest {
                board_uuid: Some(Uuid::new_v4().to_string()),
                ..Default::default()
            }))
            .await;
        assert!(list_boards_result.is_ok());
        let result_unwrap = list_boards_result.unwrap();
        let boards = &result_unwrap.get_ref().boards;
        assert_eq!(boards.len(), 0);

        // filtering by name also works - test board2 name
        let list_boards_result = bulletin_board_service
            .list_boards(Request::new(ListBoardsRequest {
                board_name: Some(String::from("test_board2")),
                ..Default::default()
            }))
            .await;
        assert!(list_boards_result.is_ok());
        let result_unwrap = list_boards_result.unwrap();
        let boards = &result_unwrap.get_ref().boards;
        assert_eq!(boards.len(), 1);
        let board = boards[0].board.as_ref().unwrap();
        assert_eq!(board.uuid, board2_uuid);
        assert_eq!(board.name, String::from("test_board2"));
        assert!(board.description.is_none());
        assert_eq!(board.is_archived, true);

        // If we filter for an inexistant board name, list is empty
        let list_boards_result = bulletin_board_service
            .list_boards(Request::new(ListBoardsRequest {
                board_uuid: Some(String::from("non-existent-board-name")),
                ..Default::default()
            }))
            .await;
        assert!(list_boards_result.is_ok());
        let result_unwrap = list_boards_result.unwrap();
        let boards = &result_unwrap.get_ref().boards;
        assert_eq!(boards.len(), 0);

        // Filtering by boards archived should only return board2
        let list_boards_result = bulletin_board_service
            .list_boards(Request::new(ListBoardsRequest {
                is_archived: Some(true),
                ..Default::default()
            }))
            .await;
        assert!(list_boards_result.is_ok());
        let result_unwrap = list_boards_result.unwrap();
        let boards = &result_unwrap.get_ref().boards;
        assert_eq!(boards.len(), 1);
        let board = boards[0].board.as_ref().unwrap();
        assert_eq!(board.uuid, board2_uuid);
        assert_eq!(board.name, String::from("test_board2"));
        assert!(board.description.is_none());
        assert_eq!(board.is_archived, true);

        // Filtering by non archived board should return board1
        let list_boards_result = bulletin_board_service
            .list_boards(Request::new(ListBoardsRequest {
                is_archived: Some(false),
                ..Default::default()
            }))
            .await;
        assert!(list_boards_result.is_ok());
        let result_unwrap = list_boards_result.unwrap();
        let boards = &result_unwrap.get_ref().boards;
        assert_eq!(boards.len(), 1);
        let board = boards[0].board.as_ref().unwrap();
        assert_eq!(board.uuid, board1_uuid);
        assert_eq!(board.name, String::from("test_board"));
        assert!(board.description.is_none());
        assert_eq!(board.is_archived, false);

        // We can also compound multiple matches: filtering by board1 uuid &
        // board1 name should return.. only board1
        let list_boards_result = bulletin_board_service
            .list_boards(Request::new(ListBoardsRequest {
                board_uuid: Some(board1_uuid.clone()),
                board_name: Some(String::from("test_board")),
                ..Default::default()
            }))
            .await;
        assert!(list_boards_result.is_ok());
        let result_unwrap = list_boards_result.unwrap();
        let boards = &result_unwrap.get_ref().boards;
        assert_eq!(boards.len(), 1);
        let board = boards[0].board.as_ref().unwrap();
        assert_eq!(board.uuid, board1_uuid);
        assert_eq!(board.name, String::from("test_board"));
        assert!(board.description.is_none());
        assert_eq!(board.is_archived, false);

        // For compound filtering, if we add to the previous one that the
        // board should be archived, then there's no match because board1 is
        // not archived
        let list_boards_result = bulletin_board_service
            .list_boards(Request::new(ListBoardsRequest {
                board_uuid: Some(board1_uuid.clone()),
                board_name: Some(String::from("test_board")),
                is_archived: Some(true),
                ..Default::default()
            }))
            .await;
        assert!(list_boards_result.is_ok());
        let result_unwrap = list_boards_result.unwrap();
        let boards = &result_unwrap.get_ref().boards;
        assert_eq!(boards.len(), 0);

        // clean up
        fs::remove_dir_all(storage_dir).unwrap();
    }

    /// Test archive board works as expected
    #[tokio::test]
    async fn test_archive_board() {
        let (storage_dir, bulletin_board_service) = setup();

        // Let's create a board
        let board_uuid = Uuid::new_v4().to_string();
        let create_request = Request::new(
            CreateBoardRequest {
                board_uuid: board_uuid.clone(),
                board_name: "test_board".into(),
                permissions: Some(basic_permisssions()),
                ..Default::default()
            }
            .sign(&*SECRET_KEY)
            .unwrap(),
        );
        let create_result =
            bulletin_board_service.create_board(create_request).await;
        assert!(create_result.is_ok());
        let board = create_result
            .unwrap()
            .get_ref()
            .bulletin_board
            .clone()
            .unwrap();

        let archive_result = bulletin_board_service
            .modify_board(Request::new(
                ModifyBoardRequest {
                    is_archived: true,
                    ..ModifyBoardRequest::from(board)
                }
                .sign(&*SECRET_KEY)
                .unwrap(),
            ))
            .await;
        assert!(archive_result.is_ok());

        // Listing filtering for board uuid should return an archived board
        let list_boards_result = bulletin_board_service
            .list_boards(Request::new(ListBoardsRequest {
                board_uuid: Some(board_uuid.clone()),
                ..Default::default()
            }))
            .await;
        assert!(list_boards_result.is_ok());
        let result_unwrap = list_boards_result.unwrap();
        let boards = &result_unwrap.get_ref().boards;
        assert_eq!(boards.len(), 1);
        let board = boards[0].board.as_ref().unwrap();
        assert_eq!(board.uuid, board_uuid);
        assert_eq!(board.name, String::from("test_board"));
        assert!(board.description.is_none());
        assert_eq!(board.is_archived, true);

        // trying to archive again the board should not work
        let archive_result = bulletin_board_service
            .modify_board(Request::new(
                ModifyBoardRequest {
                    is_archived: true,
                    ..ModifyBoardRequest::from(board.clone())
                }
                .sign(&*SECRET_KEY)
                .unwrap(),
            ))
            .await;
        assert!(archive_result.is_err());

        // Let's unarchive the board - wait a bit so that entry is not
        // duplicated
        let ten_millis = std::time::Duration::from_millis(1000);
        std::thread::sleep(ten_millis);
        let archive_result = bulletin_board_service
            .modify_board(Request::new(
                ModifyBoardRequest {
                    is_archived: false,
                    ..ModifyBoardRequest::from(board.clone())
                }
                .sign(&*SECRET_KEY)
                .unwrap(),
            ))
            .await;
        assert!(archive_result.is_ok());

        // Listing filtering for board uuid should return a non archived board
        let list_boards_result = bulletin_board_service
            .list_boards(Request::new(ListBoardsRequest {
                board_uuid: Some(board_uuid.clone()),
                ..Default::default()
            }))
            .await;
        assert!(list_boards_result.is_ok());
        let result_unwrap = list_boards_result.unwrap();
        let boards = &result_unwrap.get_ref().boards;
        assert_eq!(boards.len(), 1);
        let board = boards[0].board.as_ref().unwrap();
        assert_eq!(board.uuid, board_uuid);
        assert_eq!(board.name, String::from("test_board"));
        assert!(board.description.is_none());
        assert_eq!(board.is_archived, false);

        // Trying to archive a non-existent board should fail
        let archive_result = bulletin_board_service
            .modify_board(Request::new(
                ModifyBoardRequest {
                    board_uuid: Uuid::new_v4().to_string(),
                    is_archived: true,
                    ..ModifyBoardRequest::from(board.clone())
                }
                .sign(&*SECRET_KEY)
                .unwrap(),
            ))
            .await;
        assert!(archive_result.is_err());

        // clean up
        fs::remove_dir_all(storage_dir).unwrap();
    }

    /// Test that listing entries of a board works as expected
    #[tokio::test]
    async fn test_simple_list_entries() {
        let (storage_dir, bulletin_board_service) = setup();

        // First lets create a board
        let board_uuid = Uuid::new_v4().to_string();
        let request = Request::new(
            CreateBoardRequest {
                board_uuid: board_uuid.clone(),
                board_name: "test_board".into(),
                permissions: Some(basic_permisssions()),
                ..Default::default()
            }
            .sign(&*SECRET_KEY)
            .unwrap(),
        );
        let create_result = bulletin_board_service.create_board(request).await;
        assert!(create_result.is_ok());

        // Listing the entries gives us what we expect - one entry, just the
        // board config
        let list_entries_request = Request::new(ListEntriesRequest {
            board_uuid: board_uuid.clone(),
            start_sequence_id: 0,
        });
        let list_entries_result = bulletin_board_service
            .list_entries(list_entries_request)
            .await;
        assert!(list_entries_result.is_ok());
        let response = list_entries_result.unwrap().get_ref().clone();
        assert_eq!(response.board_last_sequence_id, 0);
        assert_eq!(response.board_entries.len(), 1);
        let board_entry = &response.board_entries[0];
        assert_eq!(board_entry.sequence_id, 0);
        assert!(matches!(
            board_entry.kind.clone().unwrap(),
            board_entry::Kind::Board { .. }
        ));
        if let board_entry::Kind::Board(board) =
            board_entry.kind.clone().unwrap()
        {
            // the data is not returned to save network resources
            assert_eq!(board.uuid, board_uuid);
            assert_eq!(board.name, String::from("test_board"));
            assert_eq!(board.is_archived, false);
        } else {
            panic!("entry data is invalid kind - this cannot really happen");
        }

        // Let's add an entry
        let entry_size = 1;
        let entry_data = (0..entry_size).map(|i| i as u8).collect::<Vec<u8>>();
        let request = Request::new(AddEntriesRequest {
            board_uuid: board_uuid.clone(),
            entries: vec![NewDataEntry {
                data: entry_data.clone(),
                ..Default::default()
            }
            .sign(&*SECRET_KEY)
            .unwrap()],
        });
        let add_result = bulletin_board_service.add_entries(request).await;
        assert!(add_result.is_ok());

        // Listing the entries gives us what we expect - two entries
        let list_entries_request = Request::new(ListEntriesRequest {
            board_uuid: board_uuid.clone(),
            start_sequence_id: 0,
        });
        let list_entries_result = bulletin_board_service
            .list_entries(list_entries_request)
            .await;
        assert!(list_entries_result.is_ok());
        let response = list_entries_result.unwrap().get_ref().clone();
        assert_eq!(response.board_last_sequence_id, 1);
        assert_eq!(response.board_entries.len(), 2);
        let board_entry = &response.board_entries[0];
        assert_eq!(board_entry.sequence_id, 0);
        assert!(matches!(
            board_entry.kind.clone().unwrap(),
            board_entry::Kind::Board { .. }
        ));
        if let board_entry::Kind::Board(board) =
            board_entry.kind.clone().unwrap()
        {
            // the data is not returned to save network resources
            assert_eq!(board.uuid, board_uuid);
            assert_eq!(board.name, String::from("test_board"));
            assert_eq!(board.is_archived, false);
        } else {
            panic!("entry data is invalid kind - this cannot really happen");
        }

        // Check result is what we expect
        let data_entry = &response.board_entries[1];
        // Entry sequence is not zero because when creating the board, a first
        // entry with the board config is automatically added
        assert_eq!(data_entry.sequence_id, 1);
        assert!(matches!(
            data_entry.kind.clone().unwrap(),
            board_entry::Kind::EntryData { .. }
        ));
        if let board_entry::Kind::EntryData(data) =
            data_entry.kind.as_ref().unwrap()
        {
            // the data is not returned to save network resources
            assert_eq!(data.data.as_ref().unwrap(), &entry_data);
        } else {
            panic!("entry data is invalid kind - this cannot really happen");
        }

        // Listing entries starting with sequence id 1 works as expected
        let list_entries_request = Request::new(ListEntriesRequest {
            board_uuid: board_uuid.clone(),
            start_sequence_id: 1,
        });
        let list_entries_result = bulletin_board_service
            .list_entries(list_entries_request)
            .await;
        assert!(list_entries_result.is_ok());
        let response = list_entries_result.unwrap().get_ref().clone();
        assert_eq!(response.board_last_sequence_id, 1);
        assert_eq!(response.board_entries.len(), 1);
        let data_entry = &response.board_entries[0];
        // Entry sequence is not zero because when creating the board, a first
        // entry with the board config is automatically added
        assert_eq!(data_entry.sequence_id, 1);
        assert!(matches!(
            data_entry.kind.clone().unwrap(),
            board_entry::Kind::EntryData { .. }
        ));
        if let board_entry::Kind::EntryData(data) =
            data_entry.kind.as_ref().unwrap()
        {
            // the data is not returned to save network resources
            assert_eq!(data.data.as_ref().unwrap(), &entry_data);
        } else {
            panic!("entry data is invalid kind - this cannot really happen");
        }

        // Listing entries with start_sequence_id 2 or 30 returns no new entries
        let list_entries_request = Request::new(ListEntriesRequest {
            board_uuid: board_uuid.clone(),
            start_sequence_id: 2,
        });
        let list_entries_result = bulletin_board_service
            .list_entries(list_entries_request)
            .await;
        assert!(list_entries_result.is_ok());
        let response = list_entries_result.unwrap().get_ref().clone();
        assert_eq!(response.board_last_sequence_id, 1);
        assert_eq!(response.board_entries.len(), 0);

        let list_entries_request = Request::new(ListEntriesRequest {
            board_uuid: board_uuid.clone(),
            start_sequence_id: 30,
        });
        let list_entries_result = bulletin_board_service
            .list_entries(list_entries_request)
            .await;
        assert!(list_entries_result.is_ok());
        let response = list_entries_result.unwrap().get_ref().clone();
        assert_eq!(response.board_last_sequence_id, 1);
        assert_eq!(response.board_entries.len(), 0);

        // clean up
        fs::remove_dir_all(storage_dir).unwrap();
    }

    /// Test that listing entries of invalid uuid board returns Err
    #[tokio::test]
    async fn test_invalid_board_list_entries() {
        let (storage_dir, bulletin_board_service) = setup();

        // Listing entries of invalid uuid board returns Err
        let list_entries_request = Request::new(ListEntriesRequest {
            board_uuid: Uuid::new_v4().to_string(),
            start_sequence_id: 0,
        });
        let list_entries_result = bulletin_board_service
            .list_entries(list_entries_request)
            .await;
        assert!(list_entries_result.is_err());

        // clean up
        fs::remove_dir_all(storage_dir).unwrap();
    }
}
