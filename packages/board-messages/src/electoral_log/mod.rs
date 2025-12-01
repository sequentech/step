// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

pub mod artifact;
pub mod message;
pub mod newtypes;
mod statement;

// Run ignored tests with
// cargo test <test_name> -- --include-ignored
#[cfg(test)]
pub(crate) mod tests {
    use immu_board::board_client::BoardClient;
    use immu_board::{BoardMessage, ElectoralLogMessage};
    use serial_test::serial;
    use strand::serialization::StrandDeserialize;
    use strand::signature::{StrandSignaturePk, StrandSignatureSk};

    use crate::electoral_log::message::{Message, SigningData};
    use crate::electoral_log::newtypes::*;

    const INDEX_DB: &'static str = "testindexdb";
    const BOARD_DB: &'static str = "testdb";
    const DUMMY_H: [u8; 64] = [1u8; 64];
    const DUMMY_STR: &'static str = "dummy";

    async fn set_up() -> BoardClient {
        let mut b = BoardClient::new("http://immudb:3322", "immudb", "immudb")
            .await
            .unwrap();

        // In case the previous test did not clean up properly
        b.delete_database(INDEX_DB).await.unwrap();
        b.delete_database(BOARD_DB).await.unwrap();

        b.upsert_index_db(INDEX_DB).await.unwrap();
        b.upsert_board_db(BOARD_DB).await.unwrap();
        b.create_board(INDEX_DB, BOARD_DB).await.unwrap();

        b
    }

    async fn tear_down(mut b: BoardClient) {
        b.delete_board(INDEX_DB, BOARD_DB).await.unwrap();
        b.delete_database(INDEX_DB).await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    #[serial]
    pub async fn test_create_delete() {
        let mut b = set_up().await;

        assert!(b.has_database(INDEX_DB).await.unwrap());
        assert!(b.has_database(BOARD_DB).await.unwrap());
        let board = b.get_board(INDEX_DB, BOARD_DB).await.unwrap();
        assert_eq!(board.database_name, BOARD_DB);
        let board = b.get_board(INDEX_DB, "NOT FOUND").await;
        assert!(board.is_err());
        tear_down(b).await;
    }

    #[tokio::test]
    #[ignore]
    #[serial]
    pub async fn test_message_create_retrieve() {
        let mut b = set_up().await;
        let board = b.get_board(INDEX_DB, BOARD_DB).await.unwrap();
        assert_eq!(board.database_name, BOARD_DB);
        let sender_name = "test";
        let sender_sk = StrandSignatureSk::gen().unwrap();
        let system_sk = StrandSignatureSk::gen().unwrap();
        let system_pk = StrandSignaturePk::from_sk(&system_sk).unwrap();
        let sd = SigningData::new(sender_sk, sender_name, system_sk);
        let event = EventIdString(DUMMY_STR.to_string());
        let election = ElectionIdString(Some(DUMMY_STR.to_string()));
        let pseudonym = PseudonymHash::new(DUMMY_H);
        let vote = CastVoteHash::new(DUMMY_H);
        let ip = VoterIpString(DUMMY_STR.to_string());
        let country = VoterCountryString(DUMMY_STR.to_string());
        let message =
            Message::cast_vote_message(event, election, pseudonym, vote, &sd, ip, country).unwrap();
        let mut board_message: ElectoralLogMessage = message.try_into().unwrap();
        // We do this so that the id matches the auto generated id in the db, otherwise the assert_eq fails
        board_message.id = 1;
        let messages = vec![board_message];

        b.insert_electoral_log_messages(BOARD_DB, &messages)
            .await
            .unwrap();
        let ret = b.get_electoral_log_messages(BOARD_DB).await.unwrap();
        assert_eq!(messages, ret);

        let first = &ret[0];
        let ret_m = Message::strand_deserialize(&first.message).unwrap();

        ret_m.verify(&system_pk).unwrap();

        tear_down(b).await;
    }
}
