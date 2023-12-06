
mod artifact;
mod message;
mod statement;
mod newtypes;

// Run ignored tests with 
// cargo test <test_name> -- --include-ignored
#[cfg(test)]
pub(crate) mod tests {
    use serial_test::serial;
    use immu_board::board_client::BoardClient;
    use immu_board::BoardMessage;
    use strand::signature::StrandSignatureSk;

    use crate::electoral_log::message::{Message, SigningData};
    use crate::electoral_log::newtypes::{ContextHash, PseudonymHash};

    const INDEX_DB: &'static str = "testindexdb";
    const BOARD_DB: &'static str = "testdb";

    async fn set_up() -> BoardClient {
        let mut b = BoardClient::new("http://immudb:3322", "immudb", "immudb").await.unwrap();
        
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
        let sd = SigningData::new(sender_sk, sender_name, system_sk);
        let ctx = ContextHash([0u8; 64]);
        let pseudonym = PseudonymHash([0u8; 64]);
        let message = Message::test_message(ctx, pseudonym, &sd).unwrap();
        let mut board_message: BoardMessage = message.try_into().unwrap();
        // We do this so that the id matches the auto generated id in the db, otherwise the assert_eq fails
        board_message.id = 1;
        let messages = vec![board_message];

        b.insert_electoral_log_messages(BOARD_DB, &messages).await.unwrap();
        let ret = b.get_electoral_log_messages(BOARD_DB).await.unwrap();
        assert_eq!(messages, ret);
        tear_down(b).await;
    }
    
}