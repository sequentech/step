use std::str::FromStr;

use anyhow::{anyhow, Result};
use bb8_postgres::bb8::Pool;
use bb8_postgres::PostgresConnectionManager;
use tokio_postgres::{config::Config, NoTls};
use tonic::{Request, Response, Status};
use tracing::{error, info};

use crate::grpc::{
    GetBoardsReply, GetBoardsRequest, GetMessagesMultiReply, GetMessagesMultiRequest,
    GetMessagesReply, GetMessagesRequest, GrpcB3Message, KeyedMessages, PutMessagesMultiReply,
    PutMessagesMultiRequest,
};
use crate::grpc::{PutMessagesReply, PutMessagesRequest};

use super::validate_board_name;
use crate::braid::message::Message;
use crate::grpc::pgsql::B3MessageRow;
use crate::grpc::pgsql::PgsqlDbConnectionParams;
use crate::grpc::pgsql::ZPgsqlB3Client;
use strand::serialization::{StrandDeserialize, StrandSerialize};

pub struct PgsqlB3Server {
    pool: Pool<PostgresConnectionManager<NoTls>>,
}
impl PgsqlB3Server {
    pub async fn new(connection: PgsqlDbConnectionParams) -> Result<PgsqlB3Server> {
        let config = Config::from_str(&connection.connection_string())?;
        let manager = PostgresConnectionManager::new(config, NoTls);
        let pool = Pool::builder().build(manager).await?;

        Ok(PgsqlB3Server { pool })
    }

    async fn get_messages_(&self, board: &str, last_id: i64) -> Result<Vec<GrpcB3Message>, Status> {
        validate_board_name(board)
            .map_err(|e| Status::invalid_argument(format!("Invalid board: {e}")))?;

        let c = self.pool.get().await;
        let Ok(c) = c else {
            error!("Pgsql connection failed: {:?}", c.err());
            return Err(Status::internal(format!("Pgsql connection failed")));
        };
        let c = ZPgsqlB3Client::new(c);

        let messages = c.get_messages(board, last_id).await;
        let Ok(messages) = messages else {
            error!(
                "Failed to retrieve messages from database: {:?}",
                messages.err()
            );
            return Err(Status::internal(format!(
                "Failed to retrieve messages from database"
            )));
        };

        let messages: Vec<GrpcB3Message> = messages
            .into_iter()
            .map(|m| GrpcB3Message {
                id: m.id,
                message: m.message,
                version: m.version,
            })
            .collect();

        Ok(messages)
    }

    async fn put_messages_(
        &self,
        board: &str,
        messages: &Vec<GrpcB3Message>,
    ) -> Result<(), Status> {
        validate_board_name(board)
            .map_err(|e| Status::invalid_argument(format!("Invalid board: {e}")))?;

        let c = self.pool.get().await;
        let Ok(c) = c else {
            error!("Pgsql connection failed: {:?}", c.err());
            return Err(Status::internal(format!("Pgsql connection failed")));
        };
        let mut c = ZPgsqlB3Client::new(c);

        let messages = messages
            .iter()
            .map(|m| B3MessageRow::try_from(m))
            .collect::<Result<Vec<B3MessageRow>>>()
            .map_err(|e| Status::internal(format!("Failed to parse grpc messages: {e}")))?;

        let reply = c.insert_messages(board, &messages).await;
        let Ok(_) = reply else {
            error!("Failed to insert messages in database: {:?}", reply.err());
            return Err(Status::internal(format!(
                "Failed to insert messages in database"
            )));
        };

        Ok(())
    }
}

#[tonic::async_trait]
impl super::proto::b3_server::B3 for PgsqlB3Server {
    async fn get_messages(
        &self,
        request: Request<GetMessagesRequest>,
    ) -> Result<Response<GetMessagesReply>, Status> {
        let r = request.get_ref();

        let messages = self.get_messages_(&r.board, r.last_id).await?;

        info!(
            "get_messages: returning {} messages with id > {} for board '{}'",
            messages.len(),
            r.last_id,
            r.board
        );

        let reply = GetMessagesReply { messages };
        Ok(Response::new(reply))
    }

    async fn put_messages(
        &self,
        request: Request<PutMessagesRequest>,
    ) -> Result<Response<PutMessagesReply>, Status> {
        let r = request.get_ref();
        info!(
            "put_messages: inserting {} messages into board '{}'",
            r.messages.len(),
            r.board
        );

        self.put_messages_(&r.board, &r.messages).await?;

        let reply = PutMessagesReply {};
        Ok(Response::new(reply))
    }

    async fn get_boards(
        &self,
        _request: Request<GetBoardsRequest>,
    ) -> Result<Response<GetBoardsReply>, Status> {
        info!("get_boards");

        let c = self.pool.get().await;
        let Ok(c) = c else {
            error!("Pgsql connection failed: {:?}", c.err());
            return Err(Status::internal(format!("Pgsql connection failed")));
        };
        let c = ZPgsqlB3Client::new(c);

        let boards = c.get_boards().await;
        let Ok(boards) = boards else {
            error!(
                "Failed to retrieve boards from database: {:?}",
                boards.err()
            );
            return Err(Status::internal(format!(
                "Failed to retrieve boards from database"
            )));
        };

        let boards = boards.into_iter().map(|b| b.board_name).collect();

        let reply = GetBoardsReply { boards };
        Ok(Response::new(reply))
    }

    async fn get_messages_multi(
        &self,
        request: Request<GetMessagesMultiRequest>,
    ) -> Result<Response<GetMessagesMultiReply>, Status> {
        let r: &GetMessagesMultiRequest = request.get_ref();

        let mut keyed: Vec<KeyedMessages> = vec![];
        let mut total_bytes: u32 = 0;
        for request in &r.requests {
            let ms = self.get_messages_(&request.board, request.last_id).await?;
            let next_bytes: usize = ms.iter().map(|m| m.message.len()).sum();
            total_bytes += next_bytes as u32;
            let k = KeyedMessages {
                board: request.board.clone(),
                messages: ms,
                deferred: false,
            };
            keyed.push(k);
        }

        info!(
            "get_messages_multi: returning {} keyed messages, size = {:.2} MB",
            keyed.len(),
            f64::from(total_bytes) / (1024.0 * 1024.0)
        );

        let reply = GetMessagesMultiReply { messages: keyed };
        Ok(Response::new(reply))
    }

    async fn put_messages_multi(
        &self,
        request: Request<PutMessagesMultiRequest>,
    ) -> Result<Response<PutMessagesMultiReply>, Status> {
        let r = request.get_ref();

        for request in &r.requests {
            self.put_messages_(&request.board, &request.messages)
                .await?;
        }

        let reply = PutMessagesMultiReply {};
        Ok(Response::new(reply))
    }
}

impl TryFrom<&GrpcB3Message> for B3MessageRow {
    type Error = anyhow::Error;

    fn try_from(message: &GrpcB3Message) -> Result<Self, Self::Error> {
        if message.version != crate::get_schema_version() {
            return Err(anyhow!(
                "Mismatched schema version: {} != {}",
                message.version,
                crate::get_schema_version()
            ));
        }

        let message = Message::strand_deserialize(&message.message)?;
        let created = crate::timestamp();

        let batch: i32 = message.statement.get_batch_number().try_into()?;
        let mix_number: i32 = message.statement.get_mix_number().try_into()?;

        Ok(B3MessageRow {
            id: 0,
            created,
            statement_timestamp: message.statement.get_timestamp(),
            statement_kind: message.statement.get_kind().to_string(),
            message: message.strand_serialize()?,
            batch,
            mix_number,
            sender_pk: message.sender.pk.to_der_b64_string()?,
            version: crate::get_schema_version(),
        })
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::marker::PhantomData;

    use super::*;
    use crate::grpc::pgsql::PgsqlConnectionParams;
    use crate::grpc::pgsql::XPgsqlB3Client;
    use crate::{
        braid::{
            artifact::Configuration, newtypes::PROTOCOL_MANAGER_INDEX,
            protocol_manager::ProtocolManager,
        },
        grpc::{client::B3Client, proto::b3_server::B3},
    };
    use serial_test::serial;
    use strand::{
        backend::ristretto::RistrettoCtx,
        context::Ctx,
        signature::{StrandSignaturePk, StrandSignatureSk},
    };

    use crate::grpc::pgsql::{create_database, drop_database};

    const PG_DATABASE: &'static str = "protocoldb";
    const PG_HOST: &'static str = "localhost";
    const PG_USER: &'static str = "postgres";
    const PG_PASSW: &'static str = "postgrespw";
    const PG_PORT: u32 = 49153;
    const TEST_BOARD: &'static str = "testboard";

    async fn set_up() -> XPgsqlB3Client {
        let c = PgsqlConnectionParams::new(PG_HOST, PG_PORT, PG_USER, PG_PASSW);
        drop_database(&c, PG_DATABASE).await.unwrap();
        create_database(&c, PG_DATABASE).await.unwrap();

        let mut client = XPgsqlB3Client::new(&c.with_database(PG_DATABASE))
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
        let c = c.with_database(PG_DATABASE);
        let b3_impl = PgsqlB3Server::new(c).await.unwrap();

        let cfg = get_test_configuration::<RistrettoCtx>(3, 2);
        let messages = vec![cfg];
        let request = B3Client::put_messages_request(TEST_BOARD, &messages).unwrap();
        let put = b3_impl.put_messages(request).await.unwrap();
        let _ = put.get_ref();

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
        let c = c.with_database(PG_DATABASE);
        let b3_impl = PgsqlB3Server::new(c).await.unwrap();

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
