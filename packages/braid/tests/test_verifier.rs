// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use std::net::SocketAddr;
use std::process::{Command, Output};
use std::sync::{mpsc, Arc, Mutex, OnceLock};

use b3::grpc::proto::b3_server::{B3Server, B3};
use b3::grpc::{
    GetBoardsReply, GetBoardsRequest, GetMessagesMultiReply, GetMessagesMultiRequest,
    GetMessagesReply, GetMessagesRequest, GrpcB3Message, PutMessagesMultiReply,
    PutMessagesMultiRequest, PutMessagesReply, PutMessagesRequest,
};
use b3::messages::artifact::{Ballots, DkgPublicKey};
use b3::messages::message::Message;
use b3::messages::newtypes::{ConfigurationHash, PublicKeyHash, MAX_TRUSTEES, NULL_TRUSTEE};
use b3::messages::statement::StatementType;
use braid::test::protocol_test_memory::create_protocol_test;
use braid::test::vector_board::VectorBoard;
use braid::test::vector_session::VectorSession;
use strand::backend::ristretto::RistrettoCtx;
use strand::context::Ctx;
use strand::elgamal::{Ciphertext, PublicKey};
use strand::serialization::StrandDeserialize;
use tonic::transport::server::TcpIncoming;
use tonic::transport::Server;
use tonic::{Request, Response, Status};

const BOARD_NAME: &str = "verifier_test_board";
const TRUSTEES: usize = 2;
const BALLOTS: usize = 10;
const MAX_CYCLES: usize = 30;

struct PublishedBoard {
    messages: Vec<GrpcB3Message>,
    cfg_h: ConfigurationHash,
}

fn published_board() -> &'static PublishedBoard {
    static BOARD: OnceLock<PublishedBoard> = OnceLock::new();
    BOARD.get_or_init(run_protocol)
}

fn threshold() -> Vec<usize> {
    (1..=TRUSTEES).collect()
}

fn board_kinds(remote: &Arc<Mutex<VectorBoard>>) -> Vec<StatementType> {
    remote
        .lock()
        .unwrap()
        .get(-1)
        .iter()
        .map(|m| {
            Message::strand_deserialize(&m.message)
                .unwrap()
                .statement
                .get_kind()
        })
        .collect()
}

fn step_until(
    sessions: &mut [VectorSession<RistrettoCtx>],
    remote: &Arc<Mutex<VectorBoard>>,
    kind: StatementType,
) {
    for _ in 0..MAX_CYCLES {
        sessions.iter_mut().for_each(|s| s.step());
        if board_kinds(remote).contains(&kind) {
            return;
        }
    }
    panic!("no {kind:?} message after {MAX_CYCLES} cycles");
}

fn step_until_idle(sessions: &mut [VectorSession<RistrettoCtx>], remote: &Arc<Mutex<VectorBoard>>) {
    for _ in 0..MAX_CYCLES {
        let before = board_kinds(remote).len();
        sessions.iter_mut().for_each(|s| s.step());
        if board_kinds(remote).len() == before {
            return;
        }
    }
    panic!("board still growing after {MAX_CYCLES} cycles");
}

fn run_protocol() -> PublishedBoard {
    let ctx = RistrettoCtx;
    let threshold = threshold();
    let test = create_protocol_test(TRUSTEES, &threshold, ctx.clone()).unwrap();
    let cfg_h = ConfigurationHash::from_configuration(&test.cfg).unwrap();

    let remote = Arc::new(Mutex::new(test.remote));
    let mut sessions: Vec<VectorSession<RistrettoCtx>> = test
        .trustees
        .into_iter()
        .map(|t| VectorSession::new(t, Arc::clone(&remote)))
        .collect();

    step_until(&mut sessions, &remote, StatementType::PublicKey);
    step_until_idle(&mut sessions, &remote);

    let pk_bytes = remote
        .lock()
        .unwrap()
        .get(-1)
        .iter()
        .map(|m| Message::strand_deserialize(&m.message).unwrap())
        .find(|m| m.statement.get_kind() == StatementType::PublicKey)
        .and_then(|m| m.artifact)
        .unwrap();
    let dkg_pk = DkgPublicKey::<RistrettoCtx>::strand_deserialize(&pk_bytes).unwrap();
    let pk = PublicKey::from_element(&dkg_pk.pk, &ctx);

    let mut rng = ctx.get_rng();
    let ballots: Vec<Ciphertext<RistrettoCtx>> = (0..BALLOTS)
        .map(|_| {
            let plaintext = ctx.rnd_plaintext(&mut rng);
            pk.encrypt(&ctx.encode(&plaintext).unwrap())
        })
        .collect();
    let mut selected = [NULL_TRUSTEE; MAX_TRUSTEES];
    selected[0..threshold.len()].copy_from_slice(&threshold);
    let pk_h = PublicKeyHash(strand::hash::hash_to_array(&pk_bytes).unwrap());
    let ballots_message = Message::ballots_msg(
        &test.cfg,
        1,
        &Ballots::new(ballots),
        selected,
        pk_h,
        &test.protocol_manager,
    )
    .unwrap();
    remote.lock().unwrap().add(ballots_message);

    step_until(&mut sessions, &remote, StatementType::Plaintexts);
    step_until_idle(&mut sessions, &remote);

    let messages = remote.lock().unwrap().get(-1);
    PublishedBoard { messages, cfg_h }
}

struct StaticBoard {
    messages: Vec<GrpcB3Message>,
    page_size: usize,
}

#[tonic::async_trait]
impl B3 for StaticBoard {
    async fn get_boards(
        &self,
        _request: Request<GetBoardsRequest>,
    ) -> Result<Response<GetBoardsReply>, Status> {
        Ok(Response::new(GetBoardsReply {
            boards: vec![BOARD_NAME.to_string()],
        }))
    }

    async fn get_messages(
        &self,
        request: Request<GetMessagesRequest>,
    ) -> Result<Response<GetMessagesReply>, Status> {
        let last_id = request.get_ref().last_id;
        let remaining: Vec<GrpcB3Message> = self
            .messages
            .iter()
            .filter(|m| m.id > last_id)
            .cloned()
            .collect();
        let truncated = remaining.len() > self.page_size;
        let messages = remaining.into_iter().take(self.page_size).collect();

        Ok(Response::new(GetMessagesReply {
            messages,
            truncated,
        }))
    }

    async fn put_messages(
        &self,
        _request: Request<PutMessagesRequest>,
    ) -> Result<Response<PutMessagesReply>, Status> {
        Err(Status::unimplemented("read only board"))
    }

    async fn get_messages_multi(
        &self,
        _request: Request<GetMessagesMultiRequest>,
    ) -> Result<Response<GetMessagesMultiReply>, Status> {
        Err(Status::unimplemented("single board only"))
    }

    async fn put_messages_multi(
        &self,
        _request: Request<PutMessagesMultiRequest>,
    ) -> Result<Response<PutMessagesMultiReply>, Status> {
        Err(Status::unimplemented("read only board"))
    }
}

fn serve(board: StaticBoard) -> String {
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async move {
            let incoming = TcpIncoming::bind(SocketAddr::from(([127, 0, 0, 1], 0))).unwrap();
            sender.send(incoming.local_addr().unwrap()).unwrap();
            Server::builder()
                .add_service(B3Server::new(board))
                .serve_with_incoming(incoming)
                .await
                .unwrap();
        });
    });

    format!("http://{}", receiver.recv().unwrap())
}

fn verify(board: StaticBoard, args: &[&str]) -> Output {
    let url = serve(board);
    Command::new(env!("CARGO_BIN_EXE_verify"))
        .args(["--server-url", &url, "--board", BOARD_NAME])
        .args(args)
        .output()
        .unwrap()
}

fn complete_board() -> StaticBoard {
    StaticBoard {
        messages: published_board().messages.clone(),
        page_size: usize::MAX,
    }
}

fn combined_output(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

#[test]
fn verify_succeeds_on_complete_board() {
    let output = verify(complete_board(), &[]);

    assert!(output.status.success(), "{}", combined_output(&output));
}

#[test]
fn verify_fails_when_mixes_are_missing() {
    let messages = published_board()
        .messages
        .iter()
        .filter(|m| {
            Message::strand_deserialize(&m.message)
                .unwrap()
                .statement
                .get_kind()
                != StatementType::Mix
        })
        .cloned()
        .collect();

    let output = verify(
        StaticBoard {
            messages,
            page_size: usize::MAX,
        },
        &[],
    );

    assert!(!output.status.success(), "{}", combined_output(&output));
}

#[test]
fn verify_rejects_ballot_hash_argument() {
    let output = verify(complete_board(), &["--ballot-hash", "00"]);

    assert!(!output.status.success(), "{}", combined_output(&output));
}

#[test]
fn verify_succeeds_with_expected_configuration_hash() {
    let expected = hex::encode(published_board().cfg_h.0);

    let output = verify(complete_board(), &["--expected-cfg-hash", &expected]);

    assert!(output.status.success(), "{}", combined_output(&output));
}

#[test]
fn verify_fails_with_other_configuration_hash() {
    let other = create_protocol_test(TRUSTEES, &threshold(), RistrettoCtx).unwrap();
    let other_h = ConfigurationHash::from_configuration(&other.cfg).unwrap();

    let output = verify(
        complete_board(),
        &["--expected-cfg-hash", &hex::encode(other_h.0)],
    );

    let text = combined_output(&output);
    assert!(!output.status.success(), "{text}");
    assert!(
        text.contains(&hex::encode(published_board().cfg_h.0)),
        "{text}"
    );
}

#[test]
fn verify_reads_every_page_of_a_truncated_reply() {
    let board = StaticBoard {
        messages: published_board().messages.clone(),
        page_size: 1,
    };

    let output = verify(board, &[]);

    assert!(output.status.success(), "{}", combined_output(&output));
}
