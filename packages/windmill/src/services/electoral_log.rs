// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::protocol_manager::get_board_client;
use crate::services::protocol_manager::get_protocol_manager;
use anyhow::Result;
use board_messages::braid::message::Signer as _;
use board_messages::electoral_log::message::Message;
use board_messages::electoral_log::message::SigningData;
use board_messages::electoral_log::newtypes::*;
use immu_board::BoardMessage;
use strand::backend::ristretto::RistrettoCtx;
use strand::signature::StrandSignatureSk;
use tracing::instrument;

pub struct ElectoralLog {
    sd: SigningData,
    elog_database: String,
}

impl ElectoralLog {
    #[instrument]
    pub async fn new(elog_database: &str) -> Result<Self> {
        let protocol_manager = get_protocol_manager::<RistrettoCtx>(elog_database).await?;

        Ok(ElectoralLog {
            sd: SigningData::new(
                protocol_manager.get_signing_key().clone(),
                "",
                protocol_manager.get_signing_key().clone(),
            ),
            elog_database: elog_database.to_string(),
        })
    }

    pub async fn new_from_sk(elog_database: &str, signing_key: &StrandSignatureSk) -> Result<Self> {
        // let protocol_manager = get_protocol_manager::<RistrettoCtx>(elog_database).await?;

        Ok(ElectoralLog {
            sd: SigningData::new(signing_key.clone(), "", signing_key.clone()),
            elog_database: elog_database.to_string(),
        })
    }

    #[instrument(skip(self, pseudonym_h, vote_h))]
    pub async fn post_cast_vote(
        &self,
        event_id: String,
        election_id: Option<String>,
        pseudonym_h: PseudonymHash,
        vote_h: CastVoteHash,
    ) -> Result<()> {
        let event = EventIdString(event_id);
        let election = ElectionIdString(election_id);

        let message = Message::cast_vote_message(event, election, pseudonym_h, vote_h, &self.sd)?;

        self.post(message).await
    }

    #[instrument(skip(self, pseudonym_h))]
    pub async fn post_cast_vote_error(
        &self,
        event_id: String,
        election_id: Option<String>,
        pseudonym_h: PseudonymHash,
        error: String,
    ) -> Result<()> {
        let event = EventIdString(event_id);
        let election = ElectionIdString(election_id);
        let error = CastVoteErrorString(error);

        let message =
            Message::cast_vote_error_message(event, election, pseudonym_h, error, &self.sd)?;

        self.post(message).await
    }

    #[instrument(skip(self))]
    pub async fn post_election_published(
        &self,
        event_id: String,
        election_id: Option<String>,
        ballot_pub_id: String,
    ) -> Result<()> {
        let event = EventIdString(event_id);
        let election = ElectionIdString(election_id);
        let ballot_pub_id = BallotPublicationIdString(ballot_pub_id);

        let message =
            Message::election_published_message(event, election, ballot_pub_id, &self.sd)?;

        self.post(message).await
    }

    #[instrument(skip(self))]
    pub async fn post_election_open(
        &self,
        event_id: String,
        election_id: Option<String>,
    ) -> Result<()> {
        let event = EventIdString(event_id);
        let election = ElectionIdString(election_id);

        let message = Message::election_open_message(event, election, &self.sd)?;

        self.post(message).await
    }

    #[instrument(skip(self))]
    pub async fn post_election_pause(
        &self,
        event_id: String,
        election_id: Option<String>,
    ) -> Result<()> {
        let event = EventIdString(event_id);
        let election = ElectionIdString(election_id);

        let message = Message::election_pause_message(event, election, &self.sd)?;

        self.post(message).await
    }

    #[instrument(skip(self))]
    pub async fn post_election_close(
        &self,
        event_id: String,
        election_id: Option<String>,
    ) -> Result<()> {
        let event = EventIdString(event_id);
        let election = ElectionIdString(election_id);

        let message = Message::election_close_message(event, election, &self.sd)?;

        self.post(message).await
    }

    #[instrument(skip(self))]
    pub async fn post_keygen(&self, event_id: String) -> Result<()> {
        let event = EventIdString(event_id);

        let message = Message::keygen_message(event, &self.sd)?;

        self.post(message).await
    }

    #[instrument(skip(self))]
    pub async fn post_key_insertion_start(&self, event_id: String) -> Result<()> {
        let event = EventIdString(event_id);

        let message = Message::key_insertion_start(event, &self.sd)?;

        self.post(message).await
    }

    #[instrument(skip(self))]
    pub async fn post_key_insertion(&self, event_id: String, trustee_name: String) -> Result<()> {
        let event = EventIdString(event_id);
        let trustee_name = TrusteeNameString(trustee_name);

        let message = Message::key_insertion_message(event, trustee_name, &self.sd)?;

        self.post(message).await
    }

    #[instrument(skip(self))]
    pub async fn post_tally_open(
        &self,
        event_id: String,
        election_id: Option<String>,
    ) -> Result<()> {
        let event = EventIdString(event_id);
        let election = ElectionIdString(election_id);

        let message = Message::tally_open_message(event, election, &self.sd)?;

        self.post(message).await
    }

    #[instrument(skip(self))]
    pub(crate) async fn post_tally_close(
        &self,
        event_id: String,
        election_id: Option<String>,
    ) -> Result<()> {
        let event = EventIdString(event_id);
        let election = ElectionIdString(election_id);

        let message = Message::tally_close_message(event, election, &self.sd)?;

        self.post(message).await
    }

    #[instrument(skip(self))]
    pub(crate) async fn post_send_communication(
        &self,
        event_id: String,
        election_id: Option<String>,
    ) -> Result<()> {
        let event = EventIdString(event_id);
        let election = ElectionIdString(election_id);

        let message = Message::send_communication(event, election, &self.sd)?;

        self.post(message).await
    }

    async fn post(&self, message: Message) -> Result<()> {
        let board_message: BoardMessage = message.try_into()?;
        let ms = vec![board_message];

        let mut client = get_board_client().await?;
        client
            .insert_electoral_log_messages(self.elog_database.as_str(), &ms)
            .await
    }
}
