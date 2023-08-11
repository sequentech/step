// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{anyhow, Context, Result};
use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tonic::transport::Channel;
use tonic::{Request, Response};
use tracing::{debug, instrument};

use crate::bulletin_board_client::BulletinBoardClient;
use crate::{
    entry::Entry, proto::BoardEntry, AddEntriesRequest, AddEntriesResponse,
    Board, BoardUuid, CreateBoardRequest, CreateBoardResponse, ListBoardItem,
    ListBoardsRequest, ListBoardsResponse, ListEntriesRequest,
    ListEntriesResponse, ModifyBoardRequest, ModifyBoardResponse,
};

/// Represents the configuration of a bulletin board
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct BoardInfo {
    pub last_cached_sequence_id: Option<u64>,
    pub board: Board,
}

/// Interface of the Cache Store
#[allow(clippy::ptr_arg)]
pub trait CacheStore {
    fn set(&mut self, key: &String, value: Vec<u8>) -> Result<()>;
    fn get(&self, key: &String) -> Result<Vec<u8>>;
    fn contains(&self, key: &String) -> Result<bool>;
}

const BOARD_IDS_KEY: &str = "board_ids.json";
const BOARD_CONFIG_PREFIX: &str = "board_config_";
const BOARD_ENTRY_PREFIX: &str = "board_entry_";

/// File-based cache store. Each item is a file in the cache directory
pub struct FileCache {
    cache_dir: PathBuf,
}

impl FileCache {
    #[instrument]
    pub fn new(cache_dir: &PathBuf) -> Result<FileCache> {
        if !cache_dir.is_dir() {
            fs::create_dir_all(&cache_dir)?;
        }
        Ok(FileCache {
            cache_dir: cache_dir.clone(),
        })
    }
}

impl CacheStore for FileCache {
    fn set(&mut self, key: &String, value: Vec<u8>) -> Result<()> {
        if key.contains(std::path::MAIN_SEPARATOR) {
            return Err(io::Error::from(io::ErrorKind::InvalidFilename).into());
        }
        let filepath = self.cache_dir.join(key);
        Ok(fs::write(filepath, value)?)
    }

    fn get(&self, key: &String) -> Result<Vec<u8>> {
        if key.contains(std::path::MAIN_SEPARATOR) {
            return Err(io::Error::from(io::ErrorKind::InvalidFilename).into());
        }
        let filepath = self.cache_dir.join(key);
        if !filepath.is_file() {
            return Err(io::Error::from(io::ErrorKind::NotFound).into());
        }
        Ok(fs::read(filepath)?)
    }

    fn contains(&self, key: &String) -> Result<bool> {
        if key.contains(std::path::MAIN_SEPARATOR) {
            return Err(io::Error::from(io::ErrorKind::InvalidFilename).into());
        }
        Ok(self.cache_dir.join(key).is_file())
    }
}

/// A Bulletin Board Client that uses a local file cache to store
///
/// NOTE: although the local cache does store the board config, it's generally
/// not updated and the only cached items for efficiency purposes are the board
/// entries.
pub struct Client<T: CacheStore> {
    rpc_client: BulletinBoardClient<Channel>,
    boards: Arc<Mutex<HashMap<BoardUuid, BoardInfo>>>,
    cache_store: T,
}

impl<T: CacheStore> Client<T> {
    #[instrument(skip(cache_store))]
    pub async fn new(server_url: String, cache_store: T) -> Result<Client<T>> {
        let board_ids: Vec<BoardUuid> =
            match cache_store.get(&BOARD_IDS_KEY.to_string()) {
                Ok(data) => serde_json::from_slice(&data)
                    .context("error loading Board Ids")?,
                _ => Default::default(),
            };
        debug!("Loaded board_ids = {board_ids:?}");
        Ok(Client {
            rpc_client: BulletinBoardClient::connect(server_url).await?,
            boards: Arc::new(Mutex::new(
                board_ids
                    .into_iter()
                    .map(|board_id| {
                        let key = Self::get_board_key(&board_id);
                        let data = cache_store.get(&key)?;
                        debug!(board_id, "loading boardinfo from cache");
                        let board_info: BoardInfo =
                            serde_json::from_slice(&data)
                                .context("error loading BoardInfo")?;
                        Ok((board_id, board_info))
                    })
                    .collect::<Result<HashMap<String, BoardInfo>>>()?,
            )),
            cache_store,
        })
    }

    pub async fn list_boards(
        &mut self,
        request: ListBoardsRequest,
    ) -> Result<Response<ListBoardsResponse>> {
        let request = Request::new(request);

        let response = self.rpc_client.list_boards(request).await?;

        Ok(response)
    }

    fn get_board_entry_key(board_uuid: &BoardUuid, sequence_id: u64) -> String {
        format!("{}_{}_{}", BOARD_ENTRY_PREFIX, board_uuid, sequence_id)
    }

    fn get_board_key(board_uuid: &BoardUuid) -> String {
        format!("{}_{}.json", BOARD_CONFIG_PREFIX, board_uuid)
    }

    pub async fn create_board(
        &mut self,
        request: CreateBoardRequest,
    ) -> Result<Response<CreateBoardResponse>> {
        let request = Request::new(request);

        // Create the board
        let response = self.rpc_client.create_board(request).await?;

        // Save the board info in memory (`self.boards`) and to the cache store
        // (`self.cache_store`)
        let board_info = BoardInfo {
            last_cached_sequence_id: None,
            board: response
                .get_ref()
                .bulletin_board
                .clone()
                .ok_or_else(|| anyhow!("EmptyBoard"))?,
        };
        let board_key = Self::get_board_key(&board_info.board.uuid);
        let board_data = serde_json::to_string_pretty(&board_info)?.into();
        self.cache_store.set(&board_key, board_data)?;
        let mut boards = self.boards.lock().await;
        boards.insert(board_info.board.uuid.clone(), board_info.clone());

        // Save the updated board list
        let board_list: Vec<&BoardUuid> = boards.keys().collect();
        self.cache_store.set(
            &BOARD_IDS_KEY.to_string(),
            serde_json::to_string_pretty(&board_list)?.into(),
        )?;

        Ok(response)
    }

    pub async fn modify_board(
        &mut self,
        request: ModifyBoardRequest,
    ) -> Result<Response<ModifyBoardResponse>> {
        let request = Request::new(request);

        let response = self.rpc_client.modify_board(request).await?;

        Ok(response)
    }

    pub async fn list_entries(
        &mut self,
        request: ListEntriesRequest,
    ) -> Result<Response<ListEntriesResponse>> {
        // Retrieve the board from the server to know what's the latest
        // sequence id.
        let board_uuid: BoardUuid = request.board_uuid.clone();
        let start_sequence_id: u64 = request.start_sequence_id;
        let get_board_request = Request::new(ListBoardsRequest {
            board_uuid: Some(board_uuid.clone()),
            ..Default::default()
        });
        let get_board_response: Response<ListBoardsResponse> =
            self.rpc_client.list_boards(get_board_request).await?;
        let boards: &Vec<ListBoardItem> = &get_board_response.get_ref().boards;
        let ListBoardItem {
            // last_sequence_id is the last sequence id existing in the board
            board_last_sequence_id: last_sequence_id,
            board,
        } = boards
            .get(0)
            .ok_or_else(|| anyhow!("BoardRetrievalError"))?;
        debug!(board_uuid, last_sequence_id, "retrieve board from server");

        let mut boards = self.boards.lock().await;
        // If we don't have this board at all, we will add it first to our
        // cache
        if !boards.contains_key(&board_uuid) {
            debug!("local cache does not contains board, adding it");
            // insert `board` into `boards` cache
            let board_info = BoardInfo {
                last_cached_sequence_id: None,
                board: board.clone().ok_or_else(|| anyhow!("EmptyBoard"))?,
            };
            let board_key = Self::get_board_key(&board_info.board.uuid);
            let board_data = serde_json::to_string_pretty(&board_info)?.into();
            self.cache_store.set(&board_key, board_data)?;
            boards.insert(board_info.board.uuid.clone(), board_info);

            // Save the updated board list on disk
            let board_list: Vec<&BoardUuid> = boards.keys().collect();
            self.cache_store.set(
                &BOARD_IDS_KEY.to_string(),
                serde_json::to_string_pretty(&board_list)?.into(),
            )?;
        }

        // Check if we have all the entries in the cache
        let board_info: &mut BoardInfo = boards
            .get_mut(&board_uuid)
            .ok_or_else(|| anyhow!("NotPossible"))?;
        let next_sequence_id = match board_info.last_cached_sequence_id {
            None => 0,
            Some(last_cached_sequence_id) => last_cached_sequence_id + 1,
        };
        debug!(
            "first entry not cached has id next_sequence_id={next_sequence_id}"
        );

        // check if we need to update our entries cache
        if last_sequence_id >= &next_sequence_id {
            debug!(next_sequence_id, "need to retrieve more entries");
            // Download missing entries and save them in our cache, then update
            // the last_cached_sequence_id in the cache too
            let list_entries_request = ListEntriesRequest {
                board_uuid: board_uuid.clone(),
                start_sequence_id: next_sequence_id,
            };
            debug!("retrieving entries starting with {next_sequence_id}");
            let get_entries_response =
                self.rpc_client.list_entries(list_entries_request).await?;
            debug!("retrieved entries: {:?}", get_entries_response.get_ref());
            let board_last_sequence_id =
                get_entries_response.get_ref().board_last_sequence_id;
            for board_entry in
                get_entries_response.get_ref().board_entries.iter()
            {
                debug!(board_entry.sequence_id, "caching entry");
                let entry_key = Self::get_board_entry_key(
                    &board_uuid,
                    board_entry.sequence_id,
                );
                let entry: Entry = board_entry.clone().into();
                self.cache_store.set(&entry_key, entry.try_to_vec()?)?;
            }

            // Update last_cached_sequence_id in memory and disk cache
            board_info.last_cached_sequence_id = Some(board_last_sequence_id);
            debug!(
                "updated board_info.last_cached_sequence_id={:?}",
                board_info.last_cached_sequence_id
            );
            let board_key = Self::get_board_key(&board_info.board.uuid);
            let board_data = serde_json::to_string_pretty(&board_info)?.into();
            self.cache_store.set(&board_key, board_data)?;
        }

        // Get the entries from the cache and return
        let board_last_sequence_id = board_info
            .last_cached_sequence_id
            .ok_or_else(|| anyhow!("NotPossible"))?;
        Ok(Response::new(ListEntriesResponse {
            board_last_sequence_id,
            board_entries: (start_sequence_id..board_last_sequence_id + 1)
                .map(|sequence_id| {
                    debug!("adding sequence_id={sequence_id} to result");
                    let entry_key =
                        Self::get_board_entry_key(&board_uuid, sequence_id);
                    let entry_bytes = self.cache_store.get(&entry_key)?;
                    let entry = Entry::try_from_slice(&entry_bytes)?;
                    let board_entry = entry.into_board_entry(sequence_id);
                    Ok(board_entry)
                })
                .collect::<Result<Vec<BoardEntry>>>()?,
        }))
    }

    pub async fn add_entries(
        &mut self,
        add_entries_request: AddEntriesRequest,
        update_cache: bool,
    ) -> Result<Response<AddEntriesResponse>> {
        let board_uuid = add_entries_request.board_uuid.clone();
        let request = Request::new(add_entries_request);
        let response = self.rpc_client.add_entries(request).await?;

        // if we need to update the cache, we call the `list_entries` method
        if update_cache {
            let list_entries_request = ListEntriesRequest {
                board_uuid: board_uuid.clone(),
                // we use the maximum start_sequence_id so that the list_entries
                // returns no entry, as we don't intend to use them
                start_sequence_id: u64::MAX,
            };
            self.list_entries(list_entries_request).await?;
        }

        Ok(response)
    }
}
