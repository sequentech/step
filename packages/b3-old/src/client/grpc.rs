// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use std::collections::HashMap;
use std::time::Duration;

use crate::messages::message::Message;

use strand::serialization::StrandSerialize;
use tonic::transport::Endpoint;

use crate::grpc::{
    B3Client as B3ClientInner, GetBoardsReply, GetBoardsRequest, GetMessagesMultiReply,
    GetMessagesMultiRequest, GetMessagesReply, GetMessagesRequest, GrpcB3Message,
    PutMessagesMultiReply, PutMessagesMultiRequest, PutMessagesReply, PutMessagesRequest,
};
use anyhow::Result;
use rayon::prelude::*;
use tonic::Request;
use tonic::{transport::Channel, Response};

pub struct B3Client {
    // grpc url
    url: String,
    // grpc max message size in bytes
    max_message_size: usize,
    // grpc message timeout
    timeout_secs: u64,
}

impl B3Client {
    pub fn new(url: &str, max_message_size: usize, timeout_secs: u64) -> B3Client {
        B3Client {
            url: url.to_string(),
            max_message_size,
            timeout_secs,
        }
    }

    pub async fn get_messages(
        &self,
        board: &str,
        last_id: i64,
    ) -> Result<Response<GetMessagesReply>> {
        let request = Self::get_messages_request(board, last_id);
        let request = Request::new(request);

        let mut client = self.get_grpc_client().await?;
        let response = client.get_messages(request).await?;

        Ok(response)
    }

    pub async fn put_messages(
        &self,
        board: &str,
        messages: &[Message],
    ) -> Result<Response<PutMessagesReply>> {
        let request = Self::put_messages_request(board, messages)?;
        let request = Request::new(request);

        let mut client = self.get_grpc_client().await?;
        let response = client.put_messages(request).await?;

        Ok(response)
    }

    pub async fn get_messages_multi(
        &self,
        requests: &Vec<(String, i64)>,
    ) -> Result<Response<GetMessagesMultiReply>> {
        let mut rs = vec![];
        for r in requests {
            let next = Self::get_messages_request(&r.0, r.1);
            rs.push(next);
        }

        let request = GetMessagesMultiRequest { requests: rs };
        let request = Request::new(request);

        let mut client = self.get_grpc_client().await?;
        let response = client.get_messages_multi(request).await?;

        Ok(response)
    }

    pub async fn put_messages_multi(
        &self,
        requests: Vec<(String, Vec<Message>)>,
    ) -> Result<Vec<Response<PutMessagesMultiReply>>> {
        if requests.len() == 0 {
            return Ok(vec![]);
        }

        let mut chunker = Chunker::new();
        let mut client = self.get_grpc_client().await?;
        let mut responses = vec![];

        for (board, messages) in requests {
            for m in messages {
                let chunk = chunker.add_message(board.clone(), m)?;
                if let Some(chunk) = chunk {
                    let response = self.put_message_batch(&chunk, &mut client).await?;
                    responses.push(response);
                }
            }
        }
        let last_chunk = std::mem::replace(&mut chunker.next_chunk, HashMap::new());
        if last_chunk.len() > 0 {
            let response = self.put_message_batch(&last_chunk, &mut client).await?;
            responses.push(response);
        }

        Ok(responses)
    }

    pub async fn get_boards(&self) -> Result<Response<GetBoardsReply>> {
        let request = Self::get_boards_request();

        let mut client = self.get_grpc_client().await?;
        let response = client.get_boards(request).await?;

        Ok(response)
    }

    async fn put_message_batch(
        &self,
        chunk: &HashMap<String, Vec<Message>>,
        client: &mut B3ClientInner<Channel>,
    ) -> Result<Response<PutMessagesMultiReply>> {
        let mut rs = vec![];
        for r in chunk {
            let next = Self::put_messages_request(&r.0, &r.1);
            rs.push(next?);
        }

        let put_request = PutMessagesMultiRequest { requests: rs };
        let put_request = Request::new(put_request);

        let response = client.put_messages_multi(put_request).await?;

        Ok(response)
    }

    pub(crate) fn put_messages_request(
        board: &str,
        messages: &[Message],
    ) -> Result<PutMessagesRequest> {
        let now = std::time::Instant::now();
        let messages: Result<Vec<GrpcB3Message>> = messages
            .into_par_iter()
            .map(|m| {
                let message = GrpcB3Message {
                    id: 0,
                    message: m.strand_serialize()?,
                    version: crate::get_schema_version(),
                };

                Ok(message)
            })
            .collect();

        println!(
            "put_messages_request: serialization in {}ms ",
            now.elapsed().as_millis()
        );

        Ok(PutMessagesRequest {
            board: board.to_string(),
            messages: messages?,
        })
    }

    pub(crate) fn get_messages_request(board: &str, last_id: i64) -> GetMessagesRequest {
        GetMessagesRequest {
            board: board.to_string(),
            last_id,
        }
    }

    pub(crate) fn get_boards_request() -> Request<GetBoardsRequest> {
        Request::new(GetBoardsRequest {})
    }

    pub(crate) async fn get_grpc_client(&self) -> Result<B3ClientInner<Channel>> {
        let endpoint = Endpoint::from_shared(self.url.clone())?;
        let endpoint = endpoint.timeout(Duration::from_secs(self.timeout_secs));
        let client = B3ClientInner::connect(endpoint).await?;
        let client = client
            .max_decoding_message_size(self.max_message_size)
            .max_encoding_message_size(self.max_message_size);

        Ok(client)
    }
}

struct Chunker {
    next_chunk: HashMap<String, Vec<Message>>,
    size: usize,
}
impl Chunker {
    fn new() -> Self {
        Self {
            next_chunk: HashMap::new(),
            size: 0,
        }
    }
    fn add_message(
        &mut self,
        board: String,
        message: Message,
    ) -> Result<Option<HashMap<String, Vec<Message>>>> {
        let size = message.artifact.as_ref().map(|m| m.len()).unwrap_or(0);
        if size > crate::grpc::MESSAGE_CHUNK_SIZE {
            return Err(anyhow::anyhow!(
                "Found single message with size above limit {} > {}",
                size,
                crate::grpc::MESSAGE_CHUNK_SIZE
            ));
        }
        let mut ret: Option<HashMap<String, Vec<Message>>> = None;

        if self.size + size > crate::grpc::MESSAGE_CHUNK_SIZE {
            if self.next_chunk.len() > 0 {
                tracing::info!("grpc::client chunking at {}", self.size);
                ret = Some(std::mem::replace(&mut self.next_chunk, HashMap::new()));
                self.size = 0;
            }
        }

        let v = self.next_chunk.get_mut(&board);
        if let Some(v) = v {
            v.push(message);
        } else {
            self.next_chunk.insert(board, vec![message]);
        }
        self.size += size;

        return Ok(ret);
    }
}
