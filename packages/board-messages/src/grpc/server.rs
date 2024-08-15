
use tonic::{Request, Response, Status};
use anyhow::{anyhow, Result};
use tracing::{info, warn};

use crate::grpc::{GrpcB3Message, GetBoardsRequest, GetBoardsReply, GetMessagesRequest, GetMessagesReply};
use crate::grpc::{PutMessagesRequest, PutMessagesReply};

use crate::braid::message::Message;
use crate::grpc::pgsql::PgsqlBoardClient;
use crate::grpc::pgsql::B3MessageRow;
use crate::grpc::pgsql::PgsqlConnectionParams;
use strand::serialization::{StrandSerialize, StrandDeserialize};

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

pub struct PgsqlB3 {
    params: PgsqlConnectionParams,
    dbname: String,
}
impl PgsqlB3 {
    pub fn new(params: PgsqlConnectionParams, dbname: &str) -> PgsqlB3 {
        PgsqlB3 {
            params,
            dbname: dbname.to_string()
        }
    }
}

#[tonic::async_trait]
impl super::proto::b3_server::B3 for PgsqlB3 {
    
    async fn get_messages(
        &self,
        request: Request<GetMessagesRequest>,
    ) -> Result<Response<GetMessagesReply>, Status> {
        
        let r = request.get_ref();
        validate_board_name(&r.board).map_err(|_| Status::invalid_argument("Invalid board"))?;

        let c = self.params.with_database(&self.dbname);
        let mut c = PgsqlBoardClient::new(&c).await.map_err(|_| Status::internal("Pgsql connection failed"))?;

        let messages = c.get_messages(&r.board, r.last_id).await
            .map_err(|_| Status::internal("Failed to retrieve messages from database"))?;

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
        validate_board_name(&r.board).map_err(|_| Status::invalid_argument("Invalid board"))?;

        let c = self.params.with_database(&self.dbname);
        let mut c = PgsqlBoardClient::new(&c).await.map_err(|_| Status::internal("Pgsql connection failed"))?;
        let messages = r.messages.iter()
            .map(|m| B3MessageRow::try_from(m))
            .collect::<Result<Vec<B3MessageRow>>>()
            .map_err(|_| Status::internal("Failed to parse grpc messages"))?;

        c.insert_messages(&r.board, &messages).await.map_err(|_| Status::internal("Failed to insert messages in database"))?;
        
        let reply = PutMessagesReply {
        };
        Ok(Response::new(reply))
    }

    async fn get_boards(
        &self,
        request: Request<GetBoardsRequest>,
    ) -> Result<Response<GetBoardsReply>, Status> {
        let ret: Vec<String> = vec![];

        let reply = GetBoardsReply {
            boards: ret
        };
        Ok(Response::new(reply))
    }
}

fn validate_board_name(board: &str) -> Result<()> {
    // FIXME
    Ok(())
}

// use tonic_mock::

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::grpc::proto::b3_server::B3;
    use serial_test::serial;
    use tonic::{client::GrpcService, service::Routes, transport::{server::Router, Server}, IntoRequest};

    const PG_DATABASE: &'static str = "protocoldb";
    const PG_HOST: &'static str = "localhost";
    const PG_USER: &'static str = "postgres";
    const PG_PASSW: &'static str = "postgrespw";
    const PG_PORT: u32 = 49154;
    const TEST_BOARD: &'static str = "testboard";
    
    #[tokio::test]
    #[ignore]
    #[serial]
    async fn test_get_messages() {
        
        let request = GetMessagesRequest {
            board: "default".to_string(),
            last_id: -1,
        };
        let request = tonic::Request::new(request);

        let c = PgsqlConnectionParams::new(PG_HOST, PG_PORT, PG_USER, PG_PASSW);
        let b3_impl = PgsqlB3::new(c, "protocoldb");

        let messages = b3_impl.get_messages(request).await.unwrap();
    }
}
