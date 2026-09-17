// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
#![cfg(feature = "native")]

use b4::messages::{artifact::Configuration, message::Message, protocol_manager::ProtocolManager};
use braid::{native::board::NoOpStorage, protocol::trustee::Trustee, util::ProtocolError};
use std::marker::PhantomData;
use strand::{
    backend::ristretto::RistrettoCtx,
    signature::{StrandSignaturePk, StrandSignatureSk},
    symm::SymmetricKey,
};

fn key(seed: u8) -> StrandSignatureSk {
    let mut der = vec![
        0x30, 0x2e, 2, 1, 0, 0x30, 5, 6, 3, 0x2b, 0x65, 0x70, 4, 0x22, 4, 0x20,
    ];
    der.extend([seed; 32]);
    StrandSignatureSk::from_der(&der).unwrap()
}
fn fixture() -> (Trustee<RistrettoCtx, NoOpStorage>, Message, Message) {
    let manager = ProtocolManager::<RistrettoCtx>::new(key(1));
    let signer = ProtocolManager::<RistrettoCtx>::new(key(2));
    let cfg = Configuration::<RistrettoCtx>::new(
        42,
        StrandSignaturePk::from_sk(&key(1)).unwrap(),
        vec![
            StrandSignaturePk::from_sk(&key(2)).unwrap(),
            StrandSignaturePk::from_sk(&key(3)).unwrap(),
        ],
        2,
        PhantomData,
    );
    let trustee = Trustee::new(
        "synthetic trustee".into(),
        "synthetic board".into(),
        key(2),
        SymmetricKey::from([7; 32]),
        NoOpStorage::new(),
        None,
    );
    (
        trustee,
        Message::bootstrap_msg(&cfg, &manager).unwrap(),
        Message::configuration_msg(&cfg, &signer).unwrap(),
    )
}

#[test]
fn bootstrap_count_includes_configuration_and_following_acknowledgement() {
    let (mut trustee, bootstrap, ack) = fixture();
    assert_eq!(
        trustee
            .update_local_board(vec![(bootstrap, 1), (ack, 2)])
            .unwrap(),
        2
    );
    assert_eq!(trustee.local_board.get_statement_entries().len(), 1);
    assert_eq!(trustee.update_local_board(vec![]).unwrap(), 0);
}

#[test]
fn bootstrap_rejections_leave_the_trustee_ready_for_a_valid_configuration() {
    let (mut trustee, bootstrap, ack) = fixture();
    assert!(
        matches!(trustee.update_local_board(vec![]),Err(ProtocolError::BootstrapError(ref e)) if e.contains("Zero messages"))
    );
    assert!(
        matches!(trustee.update_local_board(vec![(ack,1)]),Err(ProtocolError::BootstrapError(ref e)) if e.contains("Invalid statement type"))
    );
    let mut missing = bootstrap.try_clone().unwrap();
    missing.artifact = None;
    assert!(
        matches!(trustee.update_local_board(vec![(missing,1)]),Err(ProtocolError::BootstrapError(ref e)) if e.contains("No artifact"))
    );
    let mut corrupt = bootstrap.try_clone().unwrap();
    corrupt.artifact = Some(vec![0]);
    assert!(matches!(
        trustee.update_local_board(vec![(corrupt, 1)]),
        Err(ProtocolError::StrandError(_))
    ));
    assert_eq!(trustee.update_local_board(vec![(bootstrap, 1)]).unwrap(), 1);
}

#[test]
fn modified_signatures_are_rejected_without_adding_statements() {
    let (mut trustee, bootstrap, ack) = fixture();
    trustee.update_local_board(vec![(bootstrap, 1)]).unwrap();
    let mut bad = ack.try_clone().unwrap();
    // A valid signature on another statement must not authenticate this acknowledgement.
    bad.signature = key(2).sign(b"different statement bytes").unwrap();
    assert!(matches!(
        trustee.update_local_board(vec![(bad, 2)]),
        Err(ProtocolError::VerificationError(_))
    ));
    assert!(trustee.local_board.get_statement_entries().is_empty());
    assert_eq!(trustee.update_local_board(vec![(ack, 2)]).unwrap(), 1);
    assert_eq!(trustee.local_board.get_statement_entries().len(), 1);
}
