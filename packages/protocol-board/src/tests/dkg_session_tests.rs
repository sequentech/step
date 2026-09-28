// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The board reading, checked against real braid trustees.
//!
//! These run the actual protocol: braid trustees with their own keys, over a
//! shared in-memory board, against a Configuration this crate built and signed
//! from a committee, read back through the same `read_dkg` the platform runs
//! on a fetched board. If the platform's idea of a committee, an encoding or
//! the trustee order ever drifted from what braid expects, the trustees would
//! refuse to start or the reading would stop matching them.
//!
//! They live inside the crate rather than in `tests/` so that they can share
//! the committee fixtures, which are not part of the public surface.

use std::sync::Arc;

use base64::engine::general_purpose::STANDARD_NO_PAD;
use base64::Engine as _;
use cryptography::context::Context as _;
use cryptography::utils::serialization::Deserializable;
use sequent_core::types::ceremonies::{
    KeysCeremonyFailureReason, TrusteeStatus,
};
use uuid::Uuid;
use wbraid::board::persistence::NoOpPersistence;
use wbraid::board::transport::{MemoryBoard, MemoryTransport};
use wbraid::board::BoardClient;
use wbraid::messages::artifact::{Configuration, DkgPublicKey};
use wbraid::messages::newtypes::{hash_bytes, ConfigurationHash, MAX_TRUSTEES};
use wbraid::messages::wire::ProtocolMessage;
use wbraid::session::Session;
use wbraid::trustee::Trustee;

use crate::board::read_dkg;
use crate::committee::fixtures::{committee_of, trustees, TrusteeFixture};
use crate::committee::Committee;
use crate::configuration::DkgBoard;
use crate::encoding::{generate_manager, HashHex};
use crate::view::{DkgStatus, DkgView};
use crate::{Ctx, Element};

/// The informational date stamped on the messages a test posts.
const DATE: u64 = 0;

/// One ceremony's worth of protocol participants over a shared board.
struct Ceremony {
    fixtures: Vec<TrusteeFixture>,
    committee: Committee,
    dkg: DkgBoard,
    board: Arc<MemoryBoard<Ctx>>,
}

impl Ceremony {
    /// A committee of `count` trustees with the given threshold, on a board
    /// that already carries the stored Configuration message.
    fn new(count: usize, threshold: usize) -> Ceremony {
        let fixtures = trustees(count);
        let committee = committee_of(&fixtures);
        let dkg = DkgBoard::new(
            &Uuid::new_v4(),
            &generate_manager(),
            &committee,
            threshold,
        )
        .unwrap();

        let board = MemoryBoard::<Ctx>::new();
        board.push(dkg.configuration.message().clone());

        Ceremony {
            fixtures,
            committee,
            dkg,
            board,
        }
    }

    /// The Configuration the trustees run, read back from the stored message
    /// as they read it off the board.
    fn configuration(&self) -> Configuration<Ctx> {
        self.dkg
            .configuration
            .message()
            .verify_configuration()
            .unwrap()
    }

    fn configuration_hash(&self) -> ConfigurationHash {
        ConfigurationHash::from_configuration(&self.configuration()).unwrap()
    }

    /// The braid trustee behind one of the committee's members.
    fn trustee(&self, position: usize) -> Trustee<Ctx> {
        let fixture = &self.fixtures[position];
        Trustee::<Ctx>::new(
            fixture.input.name.clone(),
            fixture.signing_key.clone(),
            fixture.share_encryption.clone(),
            &self.configuration(),
        )
        .expect("the committee's trustee belongs to the configuration it built")
    }

    async fn sessions(
        &self,
    ) -> Vec<Session<Ctx, MemoryTransport<Ctx>, NoOpPersistence>> {
        let mut sessions = Vec::new();
        for position in 0..self.fixtures.len() {
            let client = BoardClient::connect(
                MemoryTransport::new(Arc::clone(&self.board)),
                NoOpPersistence,
            )
            .await
            .unwrap();
            sessions.push(Session::new(self.trustee(position), client));
        }
        sessions
    }

    /// Let every trustee take one update-step-post cycle, in order.
    async fn round(
        &self,
        sessions: &mut [Session<Ctx, MemoryTransport<Ctx>, NoOpPersistence>],
    ) {
        for session in sessions.iter_mut() {
            session.advance().await.unwrap();
        }
    }

    /// The board as the platform reads it.
    async fn view(&self) -> DkgView {
        read_dkg(Arc::clone(&self.board), self.dkg.configuration.hash()).await
    }

    fn publish_public_key(&self, position: usize, key: &DkgPublicKey<Ctx>) {
        self.board.push(ProtocolMessage::<Ctx>::public_key(
            &self.trustee(position),
            DATE,
            self.configuration_hash(),
            key,
        ));
    }
}

fn a_public_key() -> DkgPublicKey<Ctx> {
    DkgPublicKey::<Ctx>::new(Ctx::random_element(), vec![Ctx::random_element()])
}

fn assert_unusable(view: &DkgView, reason: KeysCeremonyFailureReason) -> &str {
    match &view.status {
        DkgStatus::Unusable {
            reason: found,
            detail,
        } if *found == reason => detail,
        other => panic!("expected an unusable board ({reason}), got {other:?}"),
    }
}

#[tokio::test]
async fn a_key_generation_runs_to_a_joint_public_key() {
    let ceremony = Ceremony::new(3, 2);
    let mut sessions = ceremony.sessions().await;

    let waiting = ceremony.view().await;
    assert_eq!(waiting.trustee_statuses, vec![TrusteeStatus::WAITING; 3]);
    assert_eq!(waiting.status, DkgStatus::InProgress);

    ceremony.round(&mut sessions).await;
    let dealt = ceremony.view().await;
    assert_eq!(
        dealt.trustee_statuses,
        vec![TrusteeStatus::SHARES_POSTED; 3]
    );
    assert_eq!(dealt.status, DkgStatus::InProgress);

    ceremony.round(&mut sessions).await;
    let finished = ceremony.view().await;
    assert_eq!(
        finished.trustee_statuses,
        vec![TrusteeStatus::KEY_GENERATED; 3]
    );
    let DkgStatus::Completed {
        joint_public_key,
        public_key_hash,
    } = finished.status
    else {
        panic!("expected a completed key generation, got {finished:?}");
    };

    // The stored key is the group element the trustees agreed on, and the
    // stored hash names the message that carried it.
    let (key, body) = ceremony
        .board
        .snapshot()
        .into_iter()
        .find_map(|message| {
            let body = message.body.clone()?;
            DkgPublicKey::<Ctx>::deser(&body)
                .ok()
                .map(|key| (key, body))
        })
        .expect("the board carries a DKG public key");
    // Decoded the way the voter path decodes it (sequent-core's encrypt.rs).
    let stored = STANDARD_NO_PAD.decode(&joint_public_key).unwrap();
    assert_eq!(Element::deser(&stored).unwrap(), key.pk);
    assert_eq!(public_key_hash, HashHex::of(&hash_bytes(&body)));
}

#[tokio::test]
async fn phases_are_reported_in_configuration_order() {
    let ceremony = Ceremony::new(3, 2);
    let mut sessions = ceremony.sessions().await;
    // Only the second trustee deals.
    sessions[1].advance().await.unwrap();

    let view = ceremony.view().await;
    assert_eq!(
        view.trustee_statuses,
        vec![
            TrusteeStatus::WAITING,
            TrusteeStatus::SHARES_POSTED,
            TrusteeStatus::WAITING
        ]
    );
    assert_eq!(view.status, DkgStatus::InProgress);
}

#[tokio::test]
async fn a_board_serving_another_configuration_is_not_this_ceremonys() {
    // Someone else's Configuration reached the board first: the board serves
    // that one, whatever this ceremony posts after it.
    let ceremony = Ceremony::new(3, 2);
    let squatter = Ceremony::new(3, 2);
    squatter
        .board
        .push(ceremony.dkg.configuration.message().clone());

    let view = read_dkg(
        Arc::clone(&squatter.board),
        ceremony.dkg.configuration.hash(),
    )
    .await;
    assert_unusable(
        &view,
        KeysCeremonyFailureReason::BOARD_CONFIGURATION_MISMATCH,
    );
    assert!(view.trustee_statuses.is_empty());
}

#[tokio::test]
async fn a_message_braid_refuses_makes_the_board_unusable() {
    // Signed by a key the Configuration does not name.
    let ceremony = Ceremony::new(3, 2);
    ceremony.board.push(ProtocolMessage::<Ctx>::shares(
        &generate_manager(),
        DATE,
        ceremony.configuration_hash(),
        &vec![1u8, 2, 3],
    ));

    let view = ceremony.view().await;
    let detail = assert_unusable(
        &view,
        KeysCeremonyFailureReason::INVALID_BOARD_CONTENT,
    );
    assert!(detail.contains("not part of the configuration"), "{detail}");
    assert!(view.trustee_statuses.is_empty());
}

#[tokio::test]
async fn messages_of_another_configuration_are_not_progress() {
    let ceremony = Ceremony::new(3, 2);
    ceremony.board.push(ProtocolMessage::<Ctx>::shares(
        &ceremony.trustee(0),
        DATE,
        ConfigurationHash(hash_bytes(b"another ceremony")),
        &vec![1u8, 2, 3],
    ));

    assert_eq!(
        ceremony.view().await.trustee_statuses,
        vec![TrusteeStatus::WAITING; 3]
    );
}

#[tokio::test]
async fn different_joint_public_keys_leave_no_key_to_take() {
    // Two trustees disagree before the third has published. The third halts
    // on the disagreement and never publishes, so waiting for every key
    // would wait forever: the disagreement has to be final on its own.
    let ceremony = Ceremony::new(3, 2);
    let mut sessions = ceremony.sessions().await;
    ceremony.round(&mut sessions).await;
    for position in 0..2 {
        ceremony.publish_public_key(position, &a_public_key());
    }
    assert!(
        sessions[2].advance().await.is_err(),
        "braid's trustee should halt on the disagreement"
    );

    let view = ceremony.view().await;
    let detail = assert_unusable(
        &view,
        KeysCeremonyFailureReason::INVALID_BOARD_CONTENT,
    );
    assert!(detail.contains("different joint public keys"), "{detail}");
    assert_eq!(
        view.trustee_statuses,
        vec![
            TrusteeStatus::KEY_GENERATED,
            TrusteeStatus::KEY_GENERATED,
            TrusteeStatus::SHARES_POSTED
        ]
    );
}

#[tokio::test]
async fn a_trustee_publishing_a_second_key_leaves_no_key_to_take() {
    // Every trustee agrees on one key, but one of them also published
    // another: the platform cannot tell which is the joint key.
    let ceremony = Ceremony::new(3, 2);
    let mut sessions = ceremony.sessions().await;
    ceremony.round(&mut sessions).await;
    ceremony.round(&mut sessions).await;
    assert!(matches!(
        ceremony.view().await.status,
        DkgStatus::Completed { .. }
    ));

    ceremony.publish_public_key(1, &a_public_key());
    assert_unusable(
        &ceremony.view().await,
        KeysCeremonyFailureReason::INVALID_BOARD_CONTENT,
    );
}

#[tokio::test]
async fn a_joint_key_the_platform_cannot_read_leaves_no_key_to_take() {
    // The platform hands the joint key to the voters, so a key it cannot
    // decode is a ceremony that cannot finish, even though every trustee
    // published the same thing.
    let ceremony = Ceremony::new(3, 2);
    let mut sessions = ceremony.sessions().await;
    ceremony.round(&mut sessions).await;

    let not_a_key = vec![9u8, 9, 9];
    for position in 0..3 {
        ceremony.board.push(ProtocolMessage::<Ctx>::public_key(
            &ceremony.trustee(position),
            DATE,
            ceremony.configuration_hash(),
            &not_a_key,
        ));
    }

    let view = ceremony.view().await;
    let detail = assert_unusable(
        &view,
        KeysCeremonyFailureReason::INVALID_BOARD_CONTENT,
    );
    assert!(
        detail.contains("cannot be deserialized as a DKG public key"),
        "{detail}"
    );
}

#[test]
fn the_platform_orders_the_committee_the_way_braid_numbers_senders() {
    // braid numbers a message's sender by where its signing key sits in the
    // Configuration, counting from one. The ceremony's trustee list is matched
    // to the board's phases by position, so the two orders have to agree.
    let ceremony = Ceremony::new(MAX_TRUSTEES, 2);
    let configuration = ceremony.configuration();

    for (position, member) in ceremony.committee.members().iter().enumerate() {
        assert_eq!(
            configuration.get_trustee_position(&member.signing_key),
            Some(position),
            "{}",
            member.name
        );
    }
}
