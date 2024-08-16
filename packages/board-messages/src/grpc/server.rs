
use tonic::{Request, Response, Status};
use anyhow::{anyhow, Result};
use tracing::{info, warn};

use crate::grpc::{GrpcB3Message, GetBoardsRequest, GetBoardsReply, GetMessagesRequest, GetMessagesReply};
use crate::grpc::{PutMessagesRequest, PutMessagesReply};

use crate::braid::message::Message;
use crate::grpc::pgsql::PgsqlB3Client;
use crate::grpc::pgsql::B3MessageRow;
use crate::grpc::pgsql::PgsqlConnectionParams;
use strand::serialization::{StrandSerialize, StrandDeserialize};
use super::validate_board_name;

impl TryFrom<&GrpcB3Message> for B3MessageRow {
    type Error = anyhow::Error;

    fn try_from(message: &GrpcB3Message) -> Result<Self, Self::Error> {
        if message.version != crate::get_schema_version() {
            return Err(anyhow!("Mismatched schema version: {} != {}", message.version, crate::get_schema_version()));
        }
        
        let message = Message::strand_deserialize(&message.message)?;
        let created = crate::timestamp();

        Ok(B3MessageRow {
            id: 0,
            created: created,
            statement_timestamp: message.statement.get_timestamp(),
            statement_kind: message.statement.get_kind().to_string(),
            message: message.strand_serialize()?,
            sender_pk: message.sender.pk.to_der_b64_string()?,
            version: crate::get_schema_version(),
        })
            
    }
}

pub struct PgsqlB3Server {
    params: PgsqlConnectionParams,
    dbname: String,
}
impl PgsqlB3Server {
    pub fn new(params: PgsqlConnectionParams, dbname: &str) -> PgsqlB3Server {
        PgsqlB3Server {
            params,
            dbname: dbname.to_string()
        }
    }
}

#[tonic::async_trait]
impl super::proto::b3_server::B3 for PgsqlB3Server {
    
    async fn get_messages(
        &self,
        request: Request<GetMessagesRequest>,
    ) -> Result<Response<GetMessagesReply>, Status> {
        
        let r = request.get_ref();
        validate_board_name(&r.board).map_err(|e| Status::invalid_argument(format!("Invalid board: {e}")))?;

        let c = self.params.with_database(&self.dbname);
        let mut c = PgsqlB3Client::new(&c).await.map_err(|e| Status::internal(format!("Pgsql connection failed: {e}")))?;

        let messages = c.get_messages(&r.board, r.last_id).await
            .map_err(|e| Status::internal(format!("Failed to retrieve messages from database: {e}")))?;

        let messages: Vec<GrpcB3Message> = messages.into_iter().map(|m| {
            GrpcB3Message {
                id: m.id,
                message: m.message,
                version: m.version,
            }
        }).collect();

        let reply = GetMessagesReply {
            messages: messages,
        };
        Ok(Response::new(reply))
    }

    async fn put_messages(
        &self,
        request: Request<PutMessagesRequest>,
    ) -> Result<Response<PutMessagesReply>, Status> {
        
        let r = request.get_ref();
        validate_board_name(&r.board).map_err(|e| Status::invalid_argument(format!("Invalid board: {e}")))?;

        let c = self.params.with_database(&self.dbname);
        let mut c = PgsqlB3Client::new(&c).await.map_err(|e| Status::internal(format!("Pgsql connection failed: {e}")))?;
        let messages = r.messages.iter()
            .map(|m| B3MessageRow::try_from(m))
            .collect::<Result<Vec<B3MessageRow>>>()
            .map_err(|e| Status::internal(format!("Failed to parse grpc messages: {e}")))?;

        c.insert_messages(&r.board, &messages).await.map_err(|e| Status::internal(format!("Failed to insert messages in database: {e}")))?;
        
        let reply = PutMessagesReply {
        };
        Ok(Response::new(reply))
    }

    async fn get_boards(
        &self,
        _request: Request<GetBoardsRequest>,
    ) -> Result<Response<GetBoardsReply>, Status> {
        
        let c = self.params.with_database(&self.dbname);
        let mut c = PgsqlB3Client::new(&c).await.map_err(|e| Status::internal(format!("Pgsql connection failed: {e}")))?;
        let boards = c.get_boards().await.map_err(|e| Status::internal(format!("Failed to retrieve boards from database: {e}")))?;
        let boards = boards.into_iter().map(|b| b.board_name).collect();

        let reply = GetBoardsReply {
            boards
        };
        Ok(Response::new(reply))
    }
}

// use tonic_mock::

#[cfg(test)]
pub(crate) mod tests {
    use std::marker::PhantomData;

    use super::*;
    use crate::{braid::{artifact::Configuration, newtypes::PROTOCOL_MANAGER_INDEX, protocol_manager::ProtocolManager}, grpc::{client::B3Client, proto::b3_server::B3}};
    use serial_test::serial;
    use strand::{backend::ristretto::RistrettoCtx, context::Ctx, signature::{StrandSignaturePk, StrandSignatureSk}};
    
    use crate::grpc::pgsql::{create_database, drop_database};

    const PG_DATABASE: &'static str = "protocoldb";
    const PG_HOST: &'static str = "localhost";
    const PG_USER: &'static str = "postgres";
    const PG_PASSW: &'static str = "postgrespw";
    const PG_PORT: u32 = 49153;
    const TEST_BOARD: &'static str = "testboard";
    
    async fn set_up() -> PgsqlB3Client {
        let c = PgsqlConnectionParams::new(PG_HOST, PG_PORT, PG_USER, PG_PASSW);
        drop_database(&c, PG_DATABASE).await.unwrap();
        create_database(&c, PG_DATABASE).await.unwrap();

        let mut client = PgsqlB3Client::new(&c.with_database(PG_DATABASE))
            .await
            .unwrap();
        client.create_index_ine().await.unwrap();
        client.create_board_ine(TEST_BOARD).await.unwrap();

        client
    }

    #[tokio::test]
    #[ignore]
    #[serial]
    async fn test_put_get_messages() {
        
        let _ = set_up().await;

        let c = PgsqlConnectionParams::new(PG_HOST, PG_PORT, PG_USER, PG_PASSW);
        let b3_impl = PgsqlB3Server::new(c, "protocoldb");
        
        
        let cfg = get_test_configuration::<RistrettoCtx>(3, 2);
        let messages = vec![cfg];
        let request = B3Client::put_messages_request(TEST_BOARD, &messages).unwrap();
        /*let message = GrpcB3Message{
            // does not matter when putting messages
            id: 1,
            message: cfg.strand_serialize().unwrap(),
            version: crate::get_schema_version(),
        };
        messages.push(message.clone());
        let request = PutMessagesRequest {
            messages,
            board: TEST_BOARD.to_string(),
        };
        let request = tonic::Request::new(request);*/
        let put = b3_impl.put_messages(request).await.unwrap();
        let _ = put.get_ref();
        
        /*let request = GetMessagesRequest {
            board: TEST_BOARD.to_string(),
            last_id: -1,
        };
        let request = tonic::Request::new(request);
        let messages_returned = b3_impl.get_messages(request).await.unwrap();*/
        let request = B3Client::get_messages_request(TEST_BOARD, -1);
        let messages_returned = b3_impl.get_messages(request).await.unwrap();
        let messages_returned = messages_returned.get_ref();

        assert_eq!(messages_returned.messages.len(), 1);

        let cfg_msg = Message::strand_deserialize(&messages_returned.messages[0].message).unwrap();
        let bytes = cfg_msg.artifact.clone().unwrap();
        let cfg_artifact = Configuration::<RistrettoCtx>::strand_deserialize(&bytes).unwrap();

        let verified = cfg_msg.verify(&cfg_artifact).unwrap();
        assert_eq!(verified.signer_position, PROTOCOL_MANAGER_INDEX);

    }

    #[tokio::test]
    #[ignore]
    #[serial]
    async fn test_get_boards() {
        let _ = set_up().await;

        let c = PgsqlConnectionParams::new(PG_HOST, PG_PORT, PG_USER, PG_PASSW);
        let b3_impl = PgsqlB3Server::new(c, "protocoldb");

        let request = B3Client::get_boards_request();
        let boards = b3_impl.get_boards(request).await.unwrap();
        let boards = boards.get_ref();

        assert_eq!(boards.boards.len(), 1);
        assert_eq!(boards.boards[0], TEST_BOARD);
    }

    fn get_test_configuration<C: Ctx>(n_trustees: usize, threshold: usize) -> Message {
        let pmkey: StrandSignatureSk = StrandSignatureSk::gen().unwrap();
        let pm: ProtocolManager<C> = ProtocolManager {
            signing_key: pmkey,
            phantom: PhantomData,
        };
        let trustee_pks: Vec<StrandSignaturePk> = (0..n_trustees)
            .map(|_| {
                let sk = StrandSignatureSk::gen().unwrap();
                // let encryption_key = strand::symm::gen_key();
                let pk = StrandSignaturePk::from_sk(&sk).unwrap();
                pk
            })
            .collect();

        let cfg = Configuration::<C>::new(
            0,
            StrandSignaturePk::from_sk(&pm.signing_key).unwrap(),
            trustee_pks,
            threshold,
            PhantomData,
        );

        Message::bootstrap_msg(&cfg, &pm).unwrap()
    }
}
