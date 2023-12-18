use board_messages::electoral_log::message::SigningData;
use board_messages::electoral_log::message::Message;
use board_messages::electoral_log::newtypes::*;

use immu_board::BoardClient;
use immu_board::BoardMessage;
use anyhow::{anyhow, Result};

async fn dummy_board_client() -> Result<BoardClient> {
    BoardClient::new("http://immudb:3322", "immudb", "immudb").await
}

pub(crate) async fn post_election_published(event: String, election: String, ballot_pub_id: String, sd: &SigningData, log_database: &str) -> Result<()> {
    let event = EventIdString(event);
    let election = ElectionIdString(election);
    let ballot_pub_id = BallotPublicationIdString(ballot_pub_id);
    
    let message = Message::election_published_message(event, election, ballot_pub_id, &sd)?;
    let board_message: BoardMessage = message.try_into()?;
    let ms = vec![board_message];
    
    let mut client = dummy_board_client().await?;
    client.insert_electoral_log_messages(log_database, &ms).await
}