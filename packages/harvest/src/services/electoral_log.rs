use board_messages::electoral_log::message::SigningData;
use board_messages::electoral_log::message::Message;
use board_messages::electoral_log::newtypes::*;

use immu_board::BoardClient;
use immu_board::BoardMessage;
use anyhow::{anyhow, Result};

async fn dummy_board_client() -> Result<BoardClient> {
    BoardClient::new("http://immudb:3322", "immudb", "immudb").await
}

pub(crate) async fn post_cast_vote(event_id: String, election_id: String, pseudonym_h: PseudonymHash, vote_h: CastVoteHash, sd: &SigningData, elog_database: &str) -> Result<()> {
    let event = EventIdString(event_id);
    let election = ElectionIdString(election_id);
    
    let message = Message::cast_vote_message(event, election, pseudonym_h, vote_h, &sd)?;
    
    post(message, sd, elog_database).await
}

pub(crate) async fn post_cast_vote_error(event_id: String, election_id: String, pseudonym_h: PseudonymHash, error: String, sd: &SigningData, elog_database: &str) -> Result<()> {
    let event = EventIdString(event_id);
    let election = ElectionIdString(election_id);
    let error = CastVoteErrorString(error);
    
    let message = Message::cast_vote_error_message(event, election, pseudonym_h, error, &sd)?;
    
    post(message, sd, elog_database).await
}

pub(crate) async fn post_election_published(event_id: String, election_id: String, ballot_pub_id: String, sd: &SigningData, elog_database: &str) -> Result<()> {
    let event = EventIdString(event_id);
    let election = ElectionIdString(election_id);
    let ballot_pub_id = BallotPublicationIdString(ballot_pub_id);
    
    let message = Message::election_published_message(event, election, ballot_pub_id, &sd)?;
    
    post(message, sd, elog_database).await
}

pub(crate) async fn post_election_open(event_id: String, election_id: String, ballot_pub_id: String, sd: &SigningData, elog_database: &str) -> Result<()> {
    let event = EventIdString(event_id);
    let election = ElectionIdString(election_id);
    
    let message = Message::election_open_message(event, election, &sd)?;
    
    post(message, sd, elog_database).await
}

pub(crate) async fn post_election_close(event_id: String, election_id: String, ballot_pub_id: String, sd: &SigningData, elog_database: &str) -> Result<()> {
    let event = EventIdString(event_id);
    let election = ElectionIdString(election_id);
    
    let message = Message::election_close_message(event, election, &sd)?;
    
    post(message, sd, elog_database).await
}

pub(crate) async fn post_keygen(event_id: String, sd: &SigningData, elog_database: &str) -> Result<()> {
    let event = EventIdString(event_id);
    
    let message = Message::keygen_message(event, &sd)?;
    
    post(message, sd, elog_database).await
}

pub(crate) async fn post_key_insertion(event_id: String, sd: &SigningData, elog_database: &str) -> Result<()> {
    let event = EventIdString(event_id);
    
    let message = Message::key_insertion_message(event, &sd)?;
    
    post(message, sd, elog_database).await
}

pub(crate) async fn post_tally_open(event_id: String, election_id: String, sd: &SigningData, elog_database: &str) -> Result<()> {
    let event = EventIdString(event_id);
    let election = ElectionIdString(election_id);
    
    let message = Message::tally_open_message(event, election, &sd)?;
    
    post(message, sd, elog_database).await
}

pub(crate) async fn post_tally_close(event_id: String, election_id: String, sd: &SigningData, elog_database: &str) -> Result<()> {
    let event = EventIdString(event_id);
    let election = ElectionIdString(election_id);
    
    let message = Message::tally_close_message(event, election, &sd)?;
    
    post(message, sd, elog_database).await
}

async fn post(message: Message, sd: &SigningData, elog_database: &str) -> Result<()> {
    let board_message: BoardMessage = message.try_into()?;
    let ms = vec![board_message];
    
    let mut client = dummy_board_client().await?;
    client.insert_electoral_log_messages(elog_database, &ms).await
}