// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use std::fs::{self, OpenOptions};
use std::io::{ErrorKind, Write};
use std::str::FromStr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use anyhow::{anyhow, Result};
use bb8_postgres::bb8::Pool;
use bb8_postgres::PostgresConnectionManager;
use tokio_postgres::{config::Config, NoTls};
use tonic::{Request, Response, Status};
use tracing::{error, info, warn};

use crate::grpc::{
    BoardMessages, GetBoardsReply, GetBoardsRequest, GetMessagesMultiReply,
    GetMessagesMultiRequest, GetMessagesReply, GetMessagesRequest, GrpcB3Message,
    PutMessagesMultiReply, PutMessagesMultiRequest, MESSAGE_CHUNK_SIZE,
};
use crate::grpc::{PutMessagesReply, PutMessagesRequest};
use crate::messages::statement::StatementType;

use super::validate_board_name;
use crate::client::pgsql::B3MessageRow;
use crate::client::pgsql::PgsqlDbConnectionParams;
use crate::client::pgsql::PooledPgsqlB3Client;
use crate::messages::message::Message;
use strand::serialization::{StrandDeserialize, StrandSerialize};

const BB8_POOL_SIZE: u32 = 20;

use std::path::{Path, PathBuf};

pub struct PgsqlB3Server {
    pool: Pool<PostgresConnectionManager<NoTls>>,
    blob_root: Option<PathBuf>,
}
impl PgsqlB3Server {
    pub async fn new(
        connection: PgsqlDbConnectionParams,
        blob_root: Option<PathBuf>,
    ) -> Result<PgsqlB3Server> {
        let config = Config::from_str(&connection.connection_string())?;
        let manager = PostgresConnectionManager::new(config, NoTls);
        let pool = Pool::builder()
            .max_size(BB8_POOL_SIZE)
            .build(manager)
            .await?;

        Ok(PgsqlB3Server { pool, blob_root })
    }

    async fn get_messages_(
        &self,
        board: &str,
        last_id: i64,
    ) -> Result<(Vec<GrpcB3Message>, bool), Status> {
        validate_board_name(board)
            .map_err(|e| Status::invalid_argument(format!("Invalid board: {e}")))?;

        let c = self.pool.get().await;
        let Ok(c) = c else {
            error!("Pgsql connection failed: {:?}", c.err());
            return Err(Status::internal(format!("Pgsql connection failed")));
        };
        let c = PooledPgsqlB3Client::new(c);

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

        // Need to indicate to the caller that we ran into a limit
        let mut truncated = false;
        let mut ret = if let Some(blob_root) = &self.blob_root {
            // Retrieve the message bytes from the blob store
            let now = Instant::now();
            let mut ret: Vec<GrpcB3Message> = vec![];
            let mut total_bytes = 0;

            let blob_path = Path::new(&blob_root).join(board);
            if !blob_path.exists() {
                fs::create_dir_all(&blob_path)?;
            }

            // We do a first pass to write any messages bytes that may have been written
            // directly into the database (as opposed to the blob store)
            for m in messages.iter().filter(|m| m.message.len() > 0) {
                let name = format!(
                    "{}-{}-{}-{}",
                    m.statement_kind, m.sender_pk, m.batch, m.mix_number
                );
                let path = blob_path.join(name.replace("/", ":"));
                if !path.exists() {
                    StagedBlob::write(path.clone(), &m.message)?.publish()?;
                    info!("Wrote {} bytes to {:?}", m.message.len(), path);
                }
            }

            for m in messages.into_iter() {
                let name = format!(
                    "{}-{}-{}-{}",
                    m.statement_kind, m.sender_pk, m.batch, m.mix_number
                );
                let path = blob_path.join(name.replace("/", ":"));

                let Some(buffer) = read_blob(&path)? else {
                    warn!(
                        "get_messages_: no blob for message {} at {:?}, returning the messages before it",
                        m.id, path
                    );
                    break;
                };
                let bytes = buffer.len();
                info!("read {} bytes from {:?}", bytes, path);
                if bytes > MESSAGE_CHUNK_SIZE {
                    error!(
                        "get_messages_: artifact size exceeds limit {} > {}",
                        bytes, MESSAGE_CHUNK_SIZE
                    );
                    return Err(Status::internal(format!(
                        "get_messages_: artifact size exceeds limit {} > {}",
                        bytes, MESSAGE_CHUNK_SIZE
                    )));
                }
                if total_bytes + bytes > MESSAGE_CHUNK_SIZE {
                    warn!(
                        "get_messages_: truncating response to respect limit {} > {}",
                        total_bytes, MESSAGE_CHUNK_SIZE
                    );
                    truncated = true;
                    break;
                }
                total_bytes += bytes;

                let next = GrpcB3Message {
                    id: m.id,
                    message: buffer,
                    version: m.version,
                };
                ret.push(next);
            }
            info!("Total reads: {}ms", now.elapsed().as_millis());
            ret
        } else {
            // otherwise the message bytes are in the database
            messages
                .into_iter()
                .map(|m| GrpcB3Message {
                    id: m.id,
                    message: m.message,
                    version: m.version,
                })
                .collect()
        };

        ret.sort_unstable_by_key(|m| m.id);

        Ok((ret, truncated))
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
        let mut c = PooledPgsqlB3Client::new(c);

        let mut messages = messages
            .iter()
            .map(|m| B3MessageRow::try_from(m))
            .collect::<Result<Vec<B3MessageRow>>>()
            .map_err(|e| Status::internal(format!("Failed to parse grpc messages: {e}")))?;

        // optionally retrieve the message bytes from the blob store
        let mut staged = vec![];
        if let Some(blob_root) = &self.blob_root {
            let now = Instant::now();

            let blob_path = Path::new(blob_root).join(board);
            if !blob_path.exists() {
                fs::create_dir_all(&blob_path)?;
            }

            for m in messages.iter_mut() {
                let name = format!(
                    "{}-{}-{}-{}",
                    m.statement_kind, m.sender_pk, m.batch, m.mix_number
                );
                let path = blob_path.join(name.replace("/", ":"));
                info!("Writing {} bytes to {:?}", m.message.len(), path);
                staged.push(StagedBlob::write(path, &m.message)?);

                // FIXME this is a hack
                // Allows testing and democode to retrieve this artifact
                // directly from the database.
                // If the blob store is to be used effectively, we need
                // an equivalent of PgsqlB3Client that takes the blob
                // store into account, so that it can retrieve
                // metadata from the database and messages (m.message abovev)
                // bytes from the blob store (Could be a submodule direct
                // with a direct function client that then calls
                // PgsqlB3Client for metadata and a BlobStore struct)
                if m.statement_kind != StatementType::PublicKey.to_string() {
                    m.message = vec![];
                }
            }
            info!("Total writes: {}ms", now.elapsed().as_millis());
        }

        // Publish the blobs once the messages are inserted and before the
        // transaction commits, so that a failed insert leaves the stored blobs
        // as they were and every committed message has its blob.
        let reply = if self.blob_root.is_some() {
            c.insert_messages_then_commit(board, &messages, move || {
                for blob in staged {
                    blob.publish()?;
                }
                Ok(())
            })
            .await
        } else {
            c.insert_messages(board, &messages).await
        };
        let Ok(_) = reply else {
            error!("Failed to insert messages in database: {:?}", reply.err());
            return Err(Status::internal(format!(
                "Failed to insert messages in database"
            )));
        };

        Ok(())
    }
}

static STAGED_BLOB_COUNT: AtomicU64 = AtomicU64::new(0);

/// Message bytes written beside their blob path. `publish` moves them into
/// place; dropping an unpublished blob removes them.
struct StagedBlob {
    staged: PathBuf,
    path: PathBuf,
}

impl StagedBlob {
    fn write(path: PathBuf, bytes: &[u8]) -> std::io::Result<StagedBlob> {
        let mut staged = path.clone().into_os_string();
        staged.push(format!(
            ".{}.{}.tmp",
            std::process::id(),
            STAGED_BLOB_COUNT.fetch_add(1, Ordering::Relaxed)
        ));
        let blob = StagedBlob {
            staged: PathBuf::from(staged),
            path,
        };
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&blob.staged)?;
        file.write_all(bytes)?;

        Ok(blob)
    }

    fn publish(self) -> std::io::Result<()> {
        fs::rename(&self.staged, &self.path)
    }
}

impl Drop for StagedBlob {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.staged);
    }
}

fn read_blob(path: &Path) -> std::io::Result<Option<Vec<u8>>> {
    match fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
    }
}

#[tonic::async_trait]
impl super::proto::b3_server::B3 for PgsqlB3Server {
    async fn get_messages(
        &self,
        request: Request<GetMessagesRequest>,
    ) -> Result<Response<GetMessagesReply>, Status> {
        let r = request.get_ref();

        let (messages, _) = self.get_messages_(&r.board, r.last_id).await?;

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
        let c = self.pool.get().await;
        let Ok(c) = c else {
            error!("Pgsql connection failed: {:?}", c.err());
            return Err(Status::internal(format!("Pgsql connection failed")));
        };
        let c = PooledPgsqlB3Client::new(c);

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

        let boards: Vec<String> = boards.into_iter().map(|b| b.board_name).collect();
        // info!("get_boards returns {} boards", boards.len());

        let reply = GetBoardsReply { boards };
        Ok(Response::new(reply))
    }

    async fn get_messages_multi(
        &self,
        request: Request<GetMessagesMultiRequest>,
    ) -> Result<Response<GetMessagesMultiReply>, Status> {
        let r: &GetMessagesMultiRequest = request.get_ref();

        let now = Instant::now();

        let mut keyed: Vec<BoardMessages> = vec![];
        let mut total_bytes: usize = 0;
        let mut truncated = false;

        for request in &r.requests {
            let (ms, t): (Vec<GrpcB3Message>, bool) =
                self.get_messages_(&request.board, request.last_id).await?;
            if self.blob_root.is_some() {
                if ms.len() > 0 {
                    let mut send: Vec<GrpcB3Message> = vec![];
                    for m in ms.into_iter() {
                        total_bytes += m.message.len();
                        send.push(m);
                    }
                    let k = BoardMessages {
                        board: request.board.clone(),
                        messages: send,
                    };
                    keyed.push(k);

                    if t {
                        truncated = t;
                        break;
                    }
                }
            } else {
                if ms.len() > 0 {
                    let mut send: Vec<GrpcB3Message> = vec![];
                    for m in ms.into_iter() {
                        let next_bytes: usize = m.message.len();
                        if next_bytes > MESSAGE_CHUNK_SIZE {
                            error!("get_messages_multi: encountered single message exceeding limit {} > {}", next_bytes, MESSAGE_CHUNK_SIZE);
                            return Err(Status::internal(format!("get_messages_multi: encountered single message exceeding limit {} > {}", next_bytes, MESSAGE_CHUNK_SIZE)));
                        }
                        total_bytes += next_bytes;
                        if total_bytes > MESSAGE_CHUNK_SIZE {
                            warn!(
                                "get_messages_multi: truncating response to respect limit {} > {}",
                                total_bytes, MESSAGE_CHUNK_SIZE
                            );
                            total_bytes -= next_bytes;
                            truncated = true;
                            break;
                        }
                        send.push(m);
                    }

                    let k = BoardMessages {
                        board: request.board.clone(),
                        messages: send,
                    };
                    keyed.push(k);

                    if truncated {
                        break;
                    }
                }
            }
        }

        if keyed.len() > 0 {
            info!(
                "get_messages_multi: returning {} keyed messages in {}ms, size = {:.3} MB",
                keyed.len(),
                now.elapsed().as_millis(),
                f64::from(total_bytes as u32) / (1024.0 * 1024.0),
            );
        }

        let reply = GetMessagesMultiReply {
            messages: keyed,
            truncated,
        };
        Ok(Response::new(reply))
    }

    async fn put_messages_multi(
        &self,
        request: Request<PutMessagesMultiRequest>,
    ) -> Result<Response<PutMessagesMultiReply>, Status> {
        let r = request.get_ref();

        for request in &r.requests {
            let bytes: usize = request.messages.iter().map(|m| m.message.len()).sum();
            info!(
                "post_messages_multi: received post for '{}' with {} messages",
                request.board,
                request.messages.len()
            );
            let now = std::time::Instant::now();
            self.put_messages_(&request.board, &request.messages)
                .await?;
            info!(
                "messages posted in {}ms ({:.3} MB)",
                now.elapsed().as_millis(),
                f64::from(bytes as u32) / (1024.0 * 1024.0)
            );
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
    use crate::client::pgsql::PgsqlB3Client;
    use crate::client::pgsql::PgsqlConnectionParams;
    use crate::{
        client::grpc::B3Client,
        grpc::proto::b3_server::B3,
        messages::{
            artifact::Configuration,
            message::Signer,
            newtypes::{ConfigurationHash, PROTOCOL_MANAGER_INDEX},
            protocol_manager::ProtocolManager,
            statement::Statement,
        },
    };
    use serial_test::serial;
    use strand::{
        backend::ristretto::RistrettoCtx,
        context::Ctx,
        signature::{StrandSignaturePk, StrandSignatureSk},
    };

    use crate::client::pgsql::{create_database, drop_database};

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
        let c = c.with_database(PG_DATABASE);
        let b3_impl = PgsqlB3Server::new(c, None).await.unwrap();

        let cfg = get_test_configuration::<RistrettoCtx>(3, 2);
        let messages = vec![cfg];
        let request = B3Client::put_messages_request(TEST_BOARD, &messages).unwrap();
        let put = b3_impl
            .put_messages(tonic::Request::new(request))
            .await
            .unwrap();
        let _ = put.get_ref();

        let request = B3Client::get_messages_request(TEST_BOARD, -1);
        let messages_returned = b3_impl
            .get_messages(tonic::Request::new(request))
            .await
            .unwrap();
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
        let b3_impl = PgsqlB3Server::new(c, None).await.unwrap();

        let request = B3Client::get_boards_request();
        let boards = b3_impl.get_boards(request).await.unwrap();
        let boards = boards.get_ref();

        assert_eq!(boards.boards.len(), 1);
        assert_eq!(boards.boards[0], TEST_BOARD);
    }

    #[tokio::test]
    #[ignore]
    #[serial]
    async fn test_blob_store_keeps_stored_message_when_insert_fails() {
        let mut database = set_up().await;
        let blob_root = std::env::temp_dir().join(format!("b3-blobs-{}", std::process::id()));
        let _ = fs::remove_dir_all(&blob_root);

        let manager = ProtocolManager::<RistrettoCtx>::new(StrandSignatureSk::gen().unwrap());
        let trustees: Vec<ProtocolManager<RistrettoCtx>> = (0..2)
            .map(|_| ProtocolManager::new(StrandSignatureSk::gen().unwrap()))
            .collect();
        let cfg = Configuration::<RistrettoCtx>::new(
            0,
            StrandSignaturePk::from_sk(&manager.signing_key).unwrap(),
            trustees
                .iter()
                .map(|t| StrandSignaturePk::from_sk(&t.signing_key).unwrap())
                .collect(),
            2,
            PhantomData,
        );
        let bootstrap = Message::bootstrap_msg(&cfg, &manager).unwrap();
        database
            .insert_messages(
                TEST_BOARD,
                &vec![B3MessageRow::try_from(bootstrap).unwrap()],
            )
            .await
            .unwrap();

        let c = PgsqlConnectionParams::new(PG_HOST, PG_PORT, PG_USER, PG_PASSW);
        let b3_impl = PgsqlB3Server::new(c.with_database(PG_DATABASE), Some(blob_root.clone()))
            .await
            .unwrap();

        let cfg_h = Message::configuration_msg(&cfg, &trustees[0])
            .unwrap()
            .statement
            .get_cfg_h();
        let stored = trustees[0]
            .sign(
                Statement::ConfigurationSigned(1, ConfigurationHash(cfg_h)),
                None,
            )
            .unwrap();
        let same_slot = trustees[0]
            .sign(
                Statement::ConfigurationSigned(2, ConfigurationHash(cfg_h)),
                None,
            )
            .unwrap();

        let request =
            B3Client::put_messages_request(TEST_BOARD, &[stored.try_clone().unwrap()]).unwrap();
        b3_impl
            .put_messages(tonic::Request::new(request))
            .await
            .unwrap();
        let request = B3Client::put_messages_request(TEST_BOARD, &[same_slot]).unwrap();
        assert!(b3_impl
            .put_messages(tonic::Request::new(request))
            .await
            .is_err());

        let request = B3Client::get_messages_request(TEST_BOARD, -1);
        let returned = b3_impl
            .get_messages(tonic::Request::new(request))
            .await
            .unwrap();
        let returned = &returned.get_ref().messages;
        assert_eq!(returned.len(), 2);
        assert_eq!(returned[1].message, stored.strand_serialize().unwrap());

        let _ = fs::remove_dir_all(&blob_root);
    }

    fn blob_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("b3-{}-{}", name, std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        dir
    }

    #[test]
    fn staged_blob_does_not_change_stored_blob_until_published() {
        let dir = blob_dir("staged_blob_does_not_change_stored_blob");
        let path = dir.join("Channel-sender-0-0");
        fs::write(&path, b"stored").unwrap();

        let staged = StagedBlob::write(path.clone(), b"incoming").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"stored");
        drop(staged);

        assert_eq!(fs::read(&path).unwrap(), b"stored");
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 1);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn published_blob_replaces_slot_file() {
        let dir = blob_dir("published_blob_replaces_slot_file");
        let path = dir.join("Channel-sender-0-0");

        StagedBlob::write(path.clone(), b"incoming")
            .unwrap()
            .publish()
            .unwrap();

        assert_eq!(fs::read(&path).unwrap(), b"incoming");
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 1);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn read_blob_returns_none_for_missing_blob() {
        let dir = blob_dir("read_blob_returns_none_for_missing_blob");

        let result = read_blob(&dir.join("Channel-sender-0-0")).unwrap();

        assert!(result.is_none());
        let _ = fs::remove_dir_all(&dir);
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
