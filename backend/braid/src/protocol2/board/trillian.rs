use anyhow::{anyhow, Result};
use tracing::info;

use bulletin_board::{
    board_entry::Kind, AddEntriesRequest, BoardEntryData, ListBoardItem, ListBoardsRequest,
    ListEntriesRequest, NewDataEntry,
};

use crate::protocol2::message::Message;
use bulletin_board::client::{CacheStore, Client};
use bulletin_board::signature::Signable;
use strand::serialization::{StrandDeserialize, StrandSerialize};
use strand::signature::StrandSignaturePk;
use strand::signature::StrandSignatureSk;

pub struct TrillianBoard<CS: CacheStore> {
    client: Client<CS>,
    name: String,
}

impl<CS: CacheStore> TrillianBoard<CS> {
    pub fn new(name: String, client: Client<CS>) -> TrillianBoard<CS> {
        TrillianBoard { name, client }
    }

    pub async fn get_board(&mut self) -> Result<String> {
        let request = ListBoardsRequest {
            board_name: Some(self.name.clone()),
            ..Default::default()
        };
        let response = self.client.list_boards(request).await?;
        let boards: &Vec<ListBoardItem> = &response.get_ref().boards;

        if boards.len() == 1 {
            let ListBoardItem { board, .. } =
                boards.get(0).ok_or(anyhow!("Board get returned error"))?;
            let board_uuid = board
                .clone()
                .ok_or(anyhow!("Board get returned None"))?
                .uuid;

            Ok(board_uuid)
        } else {
            Err(anyhow::Error::msg("Expected 1 result"))
        }
    }

    pub async fn get_messages(&mut self) -> Result<Vec<Message>> {
        let id = self.get_board().await?;
        info!("Board id is '{id}'");

        let request = ListEntriesRequest {
            board_uuid: id,
            start_sequence_id: 1,
        };

        let response = self.client.list_entries(request).await?;
        let entries = response.get_ref().board_entries.clone();
        info!("List entries response contains {} entries", entries.len());

        let mut messages = vec![];
        for entry in entries {
            if let Some(Kind::EntryData(BoardEntryData {
                data: Some(entry_data),
            })) = &entry.kind
            {
                if entry.sequence_id > 0 {
                    let message = Message::strand_deserialize(entry_data)?;
                    info!("Found message {:?}", message);
                    messages.push(message);
                }
            } else {
                info!("Skipping non entry data at {}", entry.sequence_id);
            }
        }

        Ok(messages)
    }

    pub async fn send_messages(&mut self, messages: Vec<Message>) -> Result<()> {
        let id = self.get_board().await?;

        let (sk, _pk) = get_admin_keys();

        for m in messages {
            let request = AddEntriesRequest {
                board_uuid: id.clone(),
                entries: vec![NewDataEntry {
                    data: m.strand_serialize()?,
                    ..Default::default()
                }
                .sign(&sk)
                .unwrap()],
            };

            info!("Adding message to the board..");
            let response = self.client.add_entries(request, true).await?;

            let entry_sequence_id = response.get_ref().entries[0].sequence_id;
            info!("New entry added with entry_sequence_id={entry_sequence_id:?}");
        }

        Ok(())
    }

}

const ADMIN_SK: &str = "gZai7r2m5/9bAV2vmtxFOXoUL8UEMBnOPZ//0eoBX2g";
const ADMIN_PK: &str = "NbkLVEFH7IOz9MAwpp9o7VmegTum4t9YSRo367dQ8ok";
use base64::{engine::general_purpose, Engine as _};

fn get_admin_keys() -> (StrandSignatureSk, StrandSignaturePk) {
    let bytes = general_purpose::STANDARD_NO_PAD
        .decode(ADMIN_SK)
        .map_err(|error| anyhow!(error))
        .unwrap();
    let sk = StrandSignatureSk::strand_deserialize(&bytes).unwrap();

    let bytes = general_purpose::STANDARD_NO_PAD.decode(ADMIN_PK).unwrap();

    let pk = StrandSignaturePk::strand_deserialize(&bytes).unwrap();

    (sk, pk)
}
