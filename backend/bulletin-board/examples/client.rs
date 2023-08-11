// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{anyhow, Result};
use borsh::{BorshDeserialize, BorshSerialize};
use bulletin_board::client::{Client, FileCache};
use bulletin_board::signature::Signable;
use bulletin_board::util::{init_log, KeyPairConfig};
use bulletin_board::{
    board_entry::Kind, AddEntriesRequest, BoardEntry, BoardEntryData,
    CreateBoardRequest, ListBoardItem, ListBoardsRequest, ListEntriesRequest,
    NewDataEntry, Permissions, Role, User, UserRole,
};
use clap::Parser;
use std::fs;
use std::path::PathBuf;
use strand::signature::StrandSignatureSk as SecretKey;
use tracing::{error, info, instrument};
use uuid::Uuid;

/// Struct used to receive the command line arguments of this example
#[derive(Parser)]
struct Cli {
    #[arg(long, value_name = "DIR_PATH")]
    cache_dir: PathBuf,

    #[arg(long)]
    server_url: String,

    #[arg(long, value_name = "FILE_PATH")]
    key_pair_path: PathBuf,
}

/// Simple example demonstrating how to use the bulletin board client, using a
/// local file-based cache. We list boards, create one if it doesn't exist, add
/// an entry and retrieve the added entry
#[tokio::main]
#[instrument]
async fn main() -> Result<()> {
    init_log().map_err(|error| anyhow!(error))?;
    let args = Cli::parse();
    let keys_pair_str: String = fs::read_to_string(args.key_pair_path)?;
    let key_pair: KeyPairConfig = toml::from_str(&keys_pair_str)?;
    let secret_key: SecretKey = key_pair.secret_key.try_into()?;

    let cache = FileCache::new(&args.cache_dir)?;
    let mut client = Client::new(args.server_url, cache).await?;
    info!("client created");

    // list boards, and find if ours exists
    let board_name = "example-board".to_string();
    let request = ListBoardsRequest {
        board_name: Some(board_name.clone()),
        ..Default::default()
    };
    info!("Retrieving board with board_name='{board_name}'..");
    let response = client.list_boards(request).await?;
    let boards: &Vec<ListBoardItem> = &response.get_ref().boards;
    info!("Received {} board(s)", boards.len());

    // obtain the board uuid, creating the board if need be
    let board_uuid: String = match boards.len() {
        0 => {
            // create the board and return board uuid
            let board_uuid = Uuid::new_v4().to_string();
            let request = CreateBoardRequest {
                board_uuid: board_uuid.clone(),
                board_name: board_name.clone(),
                permissions: Some(Permissions {
                    users: vec![User {
                        name: "admin".into(),
                        public_key: key_pair.public_key.clone(),
                        ..Default::default()
                    }],
                    roles: vec![Role {
                        name: "admins".into(),
                        permissions: vec!["AddEntries".into()],
                        ..Default::default()
                    }],
                    user_roles: vec![UserRole {
                        user_name: "admin".into(),
                        role_names: vec!["admins".into()],
                    }],
                    ..Default::default()
                }),
                ..Default::default()
            }
            .sign(&secret_key)
            .unwrap();
            info!("Board not found, creating it..");
            let _response = client.create_board(request).await?;
            info!("Board created with uuid={board_uuid}");
            board_uuid
        }
        _ => {
            // get the board uuid from response
            let ListBoardItem { board, .. } =
                boards.get(0).ok_or(anyhow!("BoardRetrievalError"))?;

            let board_uuid = board.clone().ok_or(anyhow!("EmptyBoard"))?.uuid;
            info!("Board with name='{board_name}' has uuid='{board_uuid}'");
            board_uuid
        }
    };

    // let's list entries in the board
    let request = ListEntriesRequest {
        board_uuid: board_uuid.clone(),
        start_sequence_id: 0,
    };
    info!("Listing board entries..");
    let response = client.list_entries(request).await?;
    info!(
        "List board entries response contains {} entries",
        response.get_ref().board_entries.len()
    );

    // let's add a new entry
    let new_entry_string = String::from("Hello World!");
    let request = AddEntriesRequest {
        board_uuid: board_uuid.clone(),
        entries: vec![NewDataEntry {
            data: new_entry_string.try_to_vec()?,
            ..Default::default()
        }
        .sign(&secret_key)
        .unwrap()],
    };
    info!("Adding new entry with data '{new_entry_string}' to the board..");
    let response = client
        .add_entries(request, /* update_cache = */ true)
        .await?;

    // print the entry sequence id
    let entry_sequence_id = response.get_ref().entries[0].sequence_id;
    info!("New entry added with entry_sequence_id={entry_sequence_id:?}");

    // if we try to get the new entry, it will be cached - you won't see a new
    // call to list_entries in the bulletin board service
    let request = ListEntriesRequest {
        board_uuid: board_uuid.clone(),
        start_sequence_id: entry_sequence_id,
    };
    info!("Retrieving the added entry..");
    let response = client.list_entries(request).await?;
    info!(
        "List entries response contains {} entries",
        response.get_ref().board_entries.len()
    );

    // deserialize the entry we just created and retrieved
    let entry: &BoardEntry = response
        .get_ref()
        .board_entries
        .get(0)
        .ok_or(anyhow!("MisingBoardEntry"))?;
    if let Some(Kind::EntryData(BoardEntryData {
        data: Some(entry_data),
    })) = &entry.kind
    {
        let hello_world = String::try_from_slice(entry_data)?;
        info!("The added entry data string is '{hello_world}'");
    } else {
        error!("Unexpected/invalid entry: {entry:?}");
    }

    Ok(())
}
