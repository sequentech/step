// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The tally board reading, checked against real braid trustees.
//!
//! A key generation runs to completion on a memory board; then the crate's own
//! `TallyBoard` puts ballots encrypted under the joint key on a child board,
//! and the trustees mix and decrypt them in union sessions seeded from their
//! own DKG sessions, as the trustee program runs them. The boards are read
//! back through the same `read_tally` the platform runs on fetched boards. If
//! the platform's quorum, its `Ballots` head or its idea of the tally's phases
//! ever drifted from what braid does, the trustees would stall or halt, or the
//! reading would stop matching them.
//!
//! What braid proves about the tally itself (the mix, the proofs, the
//! anti-rewrite gate, equivocation halts) is not repeated here.

use std::sync::Arc;

use base64::engine::general_purpose::STANDARD_NO_PAD;
use base64::Engine as _;
use cryptography::context::Context as _;
use cryptography::cryptosystem::{elgamal, naoryung};
use cryptography::groups::Ristretto255Group;
use cryptography::utils::serialization::Deserializable;
use sequent_core::types::ceremonies::{TallyBoardPhase, TallyFailureReason};
use uuid::Uuid;
use wbraid::board::persistence::NoOpPersistence;
use wbraid::board::transport::{MemoryBoard, MemoryTransport};
use wbraid::board::BoardClient;
use wbraid::messages::artifact::{Ballots, Plaintexts};
use wbraid::messages::newtypes::{
    hash_bytes, CiphertextsHash, ConfigurationHash, PublicKeyHash,
};
use wbraid::messages::predicate::Predicate;
use wbraid::messages::wire::{MessageType, ProtocolMessage};
use wbraid::session::Session;
use wbraid::trustee::{ballot_encryption_context, Trustee};

use super::dkg_session_tests::{Ceremony, DATE};
use crate::ballot::BallotCiphertext;
use crate::board::read_tally;
use crate::configuration::CIPHERTEXT_WIDTH;
use crate::encoding::{ElementPayload, HashHex};
use crate::tally::TallyBoardReading;
use crate::tally_board::{Quorum, TallyBoard};
use crate::tally_view::TallyView;
use crate::view::DkgStatus;
use crate::{Ctx, Element};

/// Rounds of every trustee taking a cycle before a tally is given up on.
const MAX_ROUNDS: usize = 50;

type UnionSession = Session<Ctx, MemoryTransport<Ctx>, NoOpPersistence>;

/// A ceremony whose key generation has completed, with what the platform and
/// the trustees keep of it.
struct Keyed {
    ceremony: Ceremony,
    /// Each trustee's own committed DKG predicates, in committee order: the
    /// seed of its union sessions.
    seeds: Vec<Vec<Predicate>>,
    joint_key: Element,
    /// As the keys ceremony recorded it.
    public_key_hash: HashHex,
}

impl Keyed {
    async fn new(count: usize, threshold: usize) -> Keyed {
        let ceremony = Ceremony::new(count, threshold);
        let mut sessions = ceremony.sessions().await;
        ceremony.round(&mut sessions).await;
        ceremony.round(&mut sessions).await;
        let DkgStatus::Completed {
            joint_public_key,
            public_key_hash,
        } = ceremony.view().await.status
        else {
            panic!("the key generation did not complete");
        };
        let joint_key =
            Element::deser(&STANDARD_NO_PAD.decode(joint_public_key).unwrap())
                .unwrap();
        let seeds = sessions
            .iter()
            .map(|session| session.client.committed().to_vec())
            .collect();
        Keyed {
            ceremony,
            seeds,
            joint_key,
            public_key_hash,
        }
    }

    /// Ballots carrying `payloads`, encrypted the way a voter's client
    /// encrypts them (PROTOCOL.md §5.2).
    fn ballots(&self, payloads: &[ElementPayload]) -> Vec<BallotCiphertext> {
        encrypt(self.ceremony.configuration().id, &self.joint_key, payloads)
    }

    fn tally(
        &self,
        chosen: &[&str],
        ballots: Vec<BallotCiphertext>,
    ) -> Tally<'_> {
        Tally::new(&self.ceremony, &self.public_key_hash, chosen, ballots)
    }

    /// Every trustee's union session over the tally board.
    async fn sessions(&self, tally: &Tally<'_>) -> Vec<UnionSession> {
        let mut sessions = Vec::new();
        for (position, seed) in self.seeds.iter().enumerate() {
            let client = BoardClient::connect_union(
                MemoryTransport::new(Arc::clone(&tally.child)),
                MemoryTransport::new(Arc::clone(&self.ceremony.board)),
                NoOpPersistence,
                seed.clone(),
            )
            .await
            .unwrap();
            sessions
                .push(Session::new(self.ceremony.trustee(position), client));
        }
        sessions
    }
}

fn encrypt(
    configuration_id: u128,
    joint_key: &Element,
    payloads: &[ElementPayload],
) -> Vec<BallotCiphertext> {
    let context = ballot_encryption_context::<Ctx>(configuration_id, joint_key);
    let key = naoryung::PublicKey::augment(
        &elgamal::PublicKey::new(joint_key.clone()),
        &context,
    )
    .unwrap();
    payloads
        .iter()
        .map(|payload| {
            let element = Ristretto255Group::encode_30_bytes(payload).unwrap();
            BallotCiphertext::new(
                vec!["contest".to_string()],
                key.encrypt(&[element], &context).unwrap(),
            )
        })
        .collect()
}

/// Ballots under some key, for readings that never get to decrypting them.
fn some_ballots() -> Vec<BallotCiphertext> {
    encrypt(0, &Ctx::random_element(), &[[1u8; 30], [2u8; 30]])
}

fn names(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| name.to_string()).collect()
}

/// One tally board over a ceremony's DKG board, on its own child board.
struct Tally<'a> {
    ceremony: &'a Ceremony,
    public_key_hash: HashHex,
    board: TallyBoard,
    child: Arc<MemoryBoard<Ctx>>,
}

impl<'a> Tally<'a> {
    fn new(
        ceremony: &'a Ceremony,
        public_key_hash: &HashHex,
        chosen: &[&str],
        ballots: Vec<BallotCiphertext>,
    ) -> Tally<'a> {
        let board = TallyBoard::new(
            &Uuid::new_v4(),
            &Uuid::new_v4(),
            1,
            &ceremony.dkg,
            public_key_hash,
            &quorum(ceremony, chosen),
            &ceremony.manager,
            ballots,
        )
        .unwrap();
        Tally {
            ceremony,
            public_key_hash: public_key_hash.clone(),
            board,
            child: MemoryBoard::<Ctx>::new(),
        }
    }

    /// A tally board with made-up content, for readings no trustee runs.
    fn fabricated(ceremony: &'a Ceremony, chosen: &[&str]) -> Tally<'a> {
        let tally = Tally::new(
            ceremony,
            &HashHex::of(&hash_bytes(b"the joint key")),
            chosen,
            some_ballots(),
        );
        tally.post_ballots();
        tally
    }

    fn post_ballots(&self) {
        self.child.push(self.board.input.message().clone());
    }

    /// The board as the platform reads it.
    async fn view(&self) -> TallyView {
        read_tally(
            Arc::clone(&self.ceremony.board),
            Arc::clone(&self.child),
            &self.ceremony.dkg.configuration,
            &self.board.input,
        )
        .await
    }

    fn public_key(&self) -> PublicKeyHash {
        PublicKeyHash(self.public_key_hash.to_hash())
    }

    fn ciphertexts(&self) -> CiphertextsHash {
        CiphertextsHash(hash_bytes(b"the mixed ciphertexts"))
    }

    /// A `Plaintexts` message from the committee member at `position`.
    fn plaintexts(
        &self,
        position: usize,
        ciphertexts: CiphertextsHash,
        body: &Plaintexts<Ctx, CIPHERTEXT_WIDTH>,
    ) -> ProtocolMessage<Ctx> {
        ProtocolMessage::<Ctx>::plaintexts(
            &self.ceremony.trustee(position),
            DATE,
            self.ceremony.configuration_hash(),
            self.public_key(),
            ciphertexts,
            body,
        )
    }

    /// How many `Plaintexts` messages the board carries.
    fn plaintexts_posted(&self) -> usize {
        self.child
            .snapshot()
            .iter()
            .filter(|message| message.message_type == MessageType::Plaintexts)
            .count()
    }

    /// Let the trustees run until none of them has anything left to do, one
    /// cycle at a time, reading the board after each cycle.
    async fn run(
        &self,
        sessions: &mut [UnionSession],
    ) -> Vec<(usize, TallyView)> {
        let mut readings = Vec::new();
        for _ in 0..MAX_ROUNDS {
            let mut produced_any = false;
            for session in sessions.iter_mut() {
                produced_any |= session.advance().await.unwrap();
                readings.push((self.plaintexts_posted(), self.view().await));
            }
            if !produced_any {
                return readings;
            }
        }
        panic!("the trustees did not settle within {MAX_ROUNDS} rounds");
    }
}

fn quorum(ceremony: &Ceremony, chosen: &[&str]) -> Quorum {
    let committee = ceremony
        .fixtures
        .iter()
        .map(|fixture| fixture.input.name.clone())
        .collect::<Vec<_>>();
    Quorum::of_names(&ceremony.dkg, &committee, &names(chosen)).unwrap()
}

/// 30 bytes starting with these, zero padded.
fn padded(bytes: &[u8]) -> ElementPayload {
    let mut payload = [0u8; 30];
    payload[..bytes.len()].copy_from_slice(bytes);
    payload
}

/// 30 bytes repeating this pattern.
fn repeating(pattern: [u8; 4]) -> ElementPayload {
    std::array::from_fn(|position| pattern[position % pattern.len()])
}

/// A plaintexts body that reads, decoding to made-up payloads.
fn readable_plaintexts() -> Plaintexts<Ctx, CIPHERTEXT_WIDTH> {
    Plaintexts(vec![[Ctx::random_element()]])
}

/// The phases a sequence of readings passes through, each once.
fn phases(readings: &[(usize, TallyView)]) -> Vec<TallyBoardReading> {
    let mut phases: Vec<TallyBoardReading> = Vec::new();
    for (_, view) in readings {
        let reading = view.reading();
        if phases.last() != Some(&reading) {
            phases.push(reading);
        }
    }
    phases
}

fn assert_unusable(view: &TallyView, reason: TallyFailureReason) -> &str {
    match view {
        TallyView::Unusable {
            reason: found,
            detail,
        } if *found == reason => detail,
        other => panic!("expected an unusable board ({reason}), got {other:?}"),
    }
}

#[tokio::test]
async fn a_tally_decrypts_the_payloads_its_ballots_encrypted() {
    let keyed = Keyed::new(3, 2).await;
    let encrypted = vec![
        padded(b"testing encryption"),
        repeating([0xde, 0xad, 0xbe, 0xef]),
        repeating([0xca, 0xfe, 0xba, 0xbe]),
    ];
    // Not the committee's first trustees, and not in committee order.
    let tally =
        keyed.tally(&["trustee3", "trustee1"], keyed.ballots(&encrypted));

    assert_eq!(tally.view().await, TallyView::BallotsPending);
    tally.post_ballots();
    let posted = tally.view().await;
    assert_eq!(posted, TallyView::BallotsPosted);

    let mut sessions = keyed.sessions(&tally).await;
    let readings = [vec![(0, posted)], tally.run(&mut sessions).await].concat();

    assert_eq!(
        phases(&readings),
        [
            TallyBoardPhase::BALLOTS_POSTED,
            TallyBoardPhase::MIXING,
            TallyBoardPhase::DECRYPTING,
            TallyBoardPhase::DECRYPTED,
        ]
        .map(TallyBoardReading::Phase)
    );
    let Some((_, TallyView::Decrypted { payloads })) = readings.last() else {
        panic!("expected a decrypted board, got {readings:?}");
    };
    // The mix shuffles the ballots.
    let mut decrypted = payloads.clone();
    decrypted.sort();
    let mut expected = encrypted;
    expected.sort();
    assert_eq!(decrypted, expected);
}

#[tokio::test]
async fn the_plaintexts_count_once_the_whole_quorum_published_them() {
    // Three quorum members out of four: two readings in between.
    let keyed = Keyed::new(4, 3).await;
    let tally = keyed.tally(
        &["trustee2", "trustee4", "trustee1"],
        keyed.ballots(&[padded(b"one"), padded(b"two")]),
    );
    tally.post_ballots();
    let mut sessions = keyed.sessions(&tally).await;
    let readings = tally.run(&mut sessions).await;

    let mut seen = Vec::new();
    for (posted, view) in &readings {
        match *posted {
            0 => assert!(
                !matches!(view, TallyView::Decrypted { .. }),
                "{view:?}"
            ),
            1 | 2 => assert_eq!(*view, TallyView::Decrypting, "{posted}"),
            _ => assert!(
                matches!(view, TallyView::Decrypted { .. }),
                "{posted}: {view:?}"
            ),
        }
        seen.push(*posted);
    }
    for posted in 1..=3 {
        assert!(seen.contains(&posted), "no reading with {posted} posted");
    }
}

#[tokio::test]
async fn an_empty_batch_decrypts_to_no_payloads() {
    let keyed = Keyed::new(3, 2).await;
    let tally = keyed.tally(&["trustee1", "trustee2"], Vec::new());
    tally.post_ballots();
    let mut sessions = keyed.sessions(&tally).await;
    let readings = tally.run(&mut sessions).await;

    assert_eq!(
        readings.last().map(|(_, view)| view),
        Some(&TallyView::Decrypted {
            payloads: Vec::new()
        })
    );
}

#[tokio::test]
async fn what_the_quorum_did_not_post_under_this_configuration_is_no_progress()
{
    let ceremony = Ceremony::new(3, 2);
    let tally = Tally::fabricated(&ceremony, &["trustee1", "trustee2"]);
    let outsider = ceremony.trustee(2);
    let member = ceremony.trustee(0);
    let elsewhere = ConfigurationHash(hash_bytes(b"another ceremony"));
    let body = vec![1u8, 2, 3];

    // The trustee outside the quorum.
    tally.child.push(ProtocolMessage::<Ctx>::mix(
        &outsider,
        DATE,
        ceremony.configuration_hash(),
        tally.public_key(),
        tally.ciphertexts(),
        &body,
    ));
    tally
        .child
        .push(ProtocolMessage::<Ctx>::partial_decryptions(
            &outsider,
            DATE,
            ceremony.configuration_hash(),
            tally.public_key(),
            tally.ciphertexts(),
            &body,
        ));
    tally.child.push(tally.plaintexts(
        2,
        tally.ciphertexts(),
        &readable_plaintexts(),
    ));
    // A quorum member, under another Configuration.
    tally.child.push(ProtocolMessage::<Ctx>::mix(
        &member,
        DATE,
        elsewhere,
        tally.public_key(),
        tally.ciphertexts(),
        &body,
    ));
    tally
        .child
        .push(ProtocolMessage::<Ctx>::partial_decryptions(
            &member,
            DATE,
            elsewhere,
            tally.public_key(),
            tally.ciphertexts(),
            &body,
        ));
    tally.child.push(ProtocolMessage::<Ctx>::plaintexts(
        &member,
        DATE,
        elsewhere,
        tally.public_key(),
        tally.ciphertexts(),
        &readable_plaintexts(),
    ));

    assert_eq!(tally.view().await, TallyView::BallotsPosted);
}

#[tokio::test]
async fn a_quorum_members_partial_decryptions_make_the_board_decrypting() {
    let ceremony = Ceremony::new(3, 2);
    let tally = Tally::fabricated(&ceremony, &["trustee1", "trustee2"]);
    let member = ceremony.trustee(0);
    let body = vec![1u8, 2, 3];

    tally.child.push(ProtocolMessage::<Ctx>::mix(
        &member,
        DATE,
        ceremony.configuration_hash(),
        tally.public_key(),
        tally.ciphertexts(),
        &body,
    ));
    assert_eq!(tally.view().await, TallyView::Mixing);

    tally
        .child
        .push(ProtocolMessage::<Ctx>::partial_decryptions(
            &member,
            DATE,
            ceremony.configuration_hash(),
            tally.public_key(),
            tally.ciphertexts(),
            &body,
        ));
    assert_eq!(tally.view().await, TallyView::Decrypting);
}

#[tokio::test]
async fn the_payloads_are_those_of_the_plaintexts_the_quorum_agreed_on() {
    let ceremony = Ceremony::new(3, 2);
    let tally = Tally::fabricated(&ceremony, &["trustee1", "trustee2"]);
    let plaintexts_of = |payload: &[u8]| {
        let element =
            Ristretto255Group::encode_30_bytes(&padded(payload)).unwrap();
        Plaintexts(vec![[element]])
    };

    // The trustee outside the quorum publishes first.
    tally.child.push(tally.plaintexts(
        2,
        tally.ciphertexts(),
        &plaintexts_of(b"outsider"),
    ));
    for position in 0..2 {
        tally.child.push(tally.plaintexts(
            position,
            tally.ciphertexts(),
            &plaintexts_of(b"agreed"),
        ));
    }

    assert_eq!(
        tally.view().await,
        TallyView::Decrypted {
            payloads: vec![padded(b"agreed")]
        }
    );
}

#[tokio::test]
async fn plaintexts_the_platform_cannot_read_leave_nothing_to_decrypt() {
    let ceremony = Ceremony::new(3, 2);
    let tally = Tally::fabricated(&ceremony, &["trustee1", "trustee2"]);
    let not_plaintexts = vec![9u8, 9, 9];
    for position in 0..2 {
        tally.child.push(ProtocolMessage::<Ctx>::plaintexts(
            &ceremony.trustee(position),
            DATE,
            ceremony.configuration_hash(),
            tally.public_key(),
            tally.ciphertexts(),
            &not_plaintexts,
        ));
    }

    let view = tally.view().await;
    let detail =
        assert_unusable(&view, TallyFailureReason::INVALID_BOARD_CONTENT);
    assert!(detail.contains("cannot be read as plaintexts"), "{detail}");
}

#[tokio::test]
async fn plaintexts_the_quorum_disagrees_on_are_final_at_once() {
    // Two of a quorum of three: the third never publishes once the others
    // disagree, so waiting for it would wait forever.
    let ceremony = Ceremony::new(3, 3);
    let chosen = ["trustee1", "trustee2", "trustee3"];

    let different_plaintexts = Tally::fabricated(&ceremony, &chosen);
    for position in 0..2 {
        different_plaintexts
            .child
            .push(different_plaintexts.plaintexts(
                position,
                different_plaintexts.ciphertexts(),
                &readable_plaintexts(),
            ));
    }

    let different_ciphertexts = Tally::fabricated(&ceremony, &chosen);
    let body = readable_plaintexts();
    for (position, ciphertexts) in [b"first".as_slice(), b"second".as_slice()]
        .into_iter()
        .enumerate()
    {
        different_ciphertexts
            .child
            .push(different_ciphertexts.plaintexts(
                position,
                CiphertextsHash(hash_bytes(ciphertexts)),
                &body,
            ));
    }

    for tally in [different_plaintexts, different_ciphertexts] {
        let view = tally.view().await;
        let detail =
            assert_unusable(&view, TallyFailureReason::INVALID_BOARD_CONTENT);
        assert!(detail.contains("different plaintexts"), "{detail}");
    }
}

#[tokio::test]
async fn a_ballots_message_other_than_the_boards_own_makes_it_unusable() {
    let ceremony = Ceremony::new(3, 2);
    let public_key_hash = HashHex::of(&hash_bytes(b"the joint key"));
    let row_id = Uuid::new_v4();
    let session_id = Uuid::new_v4();
    let ballots = some_ballots();
    let board_of = |chosen: &[&str]| {
        TallyBoard::new(
            &row_id,
            &session_id,
            1,
            &ceremony.dkg,
            &public_key_hash,
            &quorum(&ceremony, chosen),
            &ceremony.manager,
            ballots.clone(),
        )
        .unwrap()
    };
    let own = board_of(&["trustee1", "trustee2"]);
    // The same ciphertexts for the same tally, with another quorum.
    let other = board_of(&["trustee2", "trustee3"]);

    let alongside = MemoryBoard::<Ctx>::new();
    alongside.push(own.input.message().clone());
    alongside.push(other.input.message().clone());
    // Only the other one: the board will never carry its own.
    let instead = MemoryBoard::<Ctx>::new();
    instead.push(other.input.message().clone());
    // The manager's Ballots for another Configuration takes the same slot.
    let elsewhere = MemoryBoard::<Ctx>::new();
    elsewhere.push(own.input.message().clone());
    elsewhere.push(ProtocolMessage::<Ctx>::ballots(
        &ceremony.manager,
        DATE,
        ConfigurationHash(hash_bytes(b"another ceremony")),
        PublicKeyHash(public_key_hash.to_hash()),
        vec![1, 2],
        1,
        &Ballots::<Ctx, CIPHERTEXT_WIDTH>::new(Vec::new()),
    ));

    for child in [alongside, instead, elsewhere] {
        let view = read_tally(
            Arc::clone(&ceremony.board),
            child,
            &ceremony.dkg.configuration,
            &own.input,
        )
        .await;
        assert_unusable(&view, TallyFailureReason::INVALID_BOARD_CONTENT);
    }
}

#[tokio::test]
async fn a_parent_serving_another_configuration_is_not_this_tallys() {
    // Someone else's Configuration reached the parent board first.
    let ceremony = Ceremony::new(3, 2);
    let squatter = Ceremony::new(3, 2);
    squatter
        .board
        .push(ceremony.dkg.configuration.message().clone());
    let tally = Tally::fabricated(&ceremony, &["trustee1", "trustee2"]);

    let view = read_tally(
        Arc::clone(&squatter.board),
        Arc::clone(&tally.child),
        &ceremony.dkg.configuration,
        &tally.board.input,
    )
    .await;
    assert_unusable(&view, TallyFailureReason::BOARD_CONFIGURATION_MISMATCH);
}

#[tokio::test]
async fn a_message_braid_refuses_makes_the_board_unusable() {
    // Signed by a trustee the Configuration does not name.
    let ceremony = Ceremony::new(3, 2);
    let other = Ceremony::new(3, 2);
    let stranger = Trustee::<Ctx>::new(
        other.fixtures[0].input.name.clone(),
        other.fixtures[0].signing_key.clone(),
        other.fixtures[0].share_encryption.clone(),
        &other.configuration(),
    )
    .unwrap();
    let tally = Tally::fabricated(&ceremony, &["trustee1", "trustee2"]);
    tally.child.push(ProtocolMessage::<Ctx>::mix(
        &stranger,
        DATE,
        ceremony.configuration_hash(),
        tally.public_key(),
        tally.ciphertexts(),
        &vec![1u8, 2, 3],
    ));

    let view = tally.view().await;
    let detail =
        assert_unusable(&view, TallyFailureReason::INVALID_BOARD_CONTENT);
    assert!(detail.contains("not part of the configuration"), "{detail}");
}
