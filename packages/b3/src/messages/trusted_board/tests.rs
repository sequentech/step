// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use crate::messages::{
    artifact::{Ballots, DkgPublicKey, Plaintexts},
    message::Signer,
    protocol_manager::ProtocolManager,
};
use std::marker::PhantomData;
use strand::{
    backend::ristretto::RistrettoCtx,
    serialization::{StrandSerialize, StrandVector},
    signature::StrandSignatureSk,
};

type Manager = ProtocolManager<RistrettoCtx>;

fn fixture() -> (
    Manager,
    Vec<Manager>,
    Configuration<RistrettoCtx>,
    Vec<Message>,
) {
    let manager = Manager::new(StrandSignatureSk::gen().unwrap());
    let trustees: Vec<_> = (0..3)
        .map(|_| Manager::new(StrandSignatureSk::gen().unwrap()))
        .collect();
    let cfg = Configuration::new(
        1,
        StrandSignaturePk::from_sk(&manager.signing_key).unwrap(),
        trustees
            .iter()
            .map(|t| StrandSignaturePk::from_sk(&t.signing_key).unwrap())
            .collect(),
        2,
        PhantomData,
    );
    let messages = vec![Message::bootstrap_msg(&cfg, &manager).unwrap()];
    (manager, trustees, cfg, messages)
}

#[test]
fn configuration_requires_external_manager_anchor_and_valid_signature() {
    let (_, _, cfg, mut messages) = fixture();
    assert!(matches!(
        configuration_state::<RistrettoCtx>(&[], &cfg.protocol_manager),
        BoardConfigurationState::Missing
    ));
    assert!(verify_board::<RistrettoCtx>(&messages, &cfg.protocol_manager).is_ok());
    let foreign = StrandSignaturePk::from_sk(&StrandSignatureSk::gen().unwrap()).unwrap();
    assert!(matches!(
        configuration_state::<RistrettoCtx>(&messages, &foreign),
        BoardConfigurationState::Foreign
    ));
    messages[0].sender.pk = foreign;
    assert!(verify_board::<RistrettoCtx>(&messages, &cfg.protocol_manager).is_err());
}

#[test]
fn message_with_mismatched_sender_is_rejected() {
    let (_, trustees, cfg, mut messages) = fixture();
    let mut valid = Message::configuration_msg(&cfg, &trustees[0]).unwrap();
    valid.sender.pk = cfg.trustees[1].clone();
    messages.push(valid);
    assert!(verify_board::<RistrettoCtx>(&messages, &cfg.protocol_manager).is_err());
    messages.pop();
    let valid = Message::configuration_msg(&cfg, &trustees[0]).unwrap();
    messages.push(valid);
    assert!(verify_board::<RistrettoCtx>(&messages, &cfg.protocol_manager).is_ok());
}

#[test]
fn key_requires_every_trustee_to_agree_on_complete_statement() {
    let (_, trustees, cfg, mut messages) = fixture();
    let ctx = RistrettoCtx;
    let pk = DkgPublicKey::new(ctx.generator().clone(), vec![]);
    let shares = SharesHashes([[1; 64]; MAX_TRUSTEES]);
    let channels = ChannelsHashes([[2; 64]; MAX_TRUSTEES]);
    messages
        .push(Message::public_key_msg(&cfg, &pk, &shares, &channels, true, &trustees[0]).unwrap());
    assert!(agreed_public_key(&messages, &cfg).is_none());
    messages
        .push(Message::public_key_msg(&cfg, &pk, &shares, &channels, false, &trustees[1]).unwrap());
    let mut conflicting = Message::public_key_msg(
        &cfg,
        &pk,
        &shares,
        &ChannelsHashes([[3; 64]; MAX_TRUSTEES]),
        false,
        &trustees[2],
    )
    .unwrap();
    assert!(conflicting.verify(&cfg).is_ok());
    messages.push(conflicting.try_clone().unwrap());
    assert!(agreed_public_key(&messages, &cfg).is_none());
    messages.pop();
    conflicting =
        Message::public_key_msg(&cfg, &pk, &shares, &channels, false, &trustees[2]).unwrap();
    messages.push(conflicting);
    verify_board::<RistrettoCtx>(&messages, &cfg.protocol_manager).unwrap();
    assert!(agreed_public_key(&messages, &cfg).is_some());
    messages.last_mut().unwrap().statement = messages[1].statement.clone();
    assert!(verify_board::<RistrettoCtx>(&messages, &cfg.protocol_manager).is_err());
}

#[test]
fn results_wait_for_matching_selected_trustee_signatures() {
    let (manager, trustees, cfg, mut messages) = fixture();
    let mut selected = [NULL_TRUSTEE; MAX_TRUSTEES];
    selected[0] = 1;
    selected[1] = 2;
    let pk = PublicKeyHash([4; 64]);
    messages.push(
        Message::ballots_msg(&cfg, 9, &Ballots::new(vec![]), selected, pk, &manager).unwrap(),
    );
    let result = Message::plaintexts_msg(
        &cfg,
        9,
        Plaintexts(StrandVector(vec![])),
        DecryptionFactorsHashes([[5; 64]; MAX_TRUSTEES]),
        CiphertextsHash([6; 64]),
        pk,
        &trustees[0],
    )
    .unwrap();
    let Statement::Plaintexts(_, _, _, ph, df, ch, kh) = &result.statement else {
        unreachable!()
    };
    messages.push(result.try_clone().unwrap());
    assert!(!plaintexts_agreed(&result, &messages, &cfg));
    messages.push(
        Message::plaintexts_signed_msg(
            &cfg,
            9,
            *ph,
            *df,
            CiphertextsHash([7; 64]),
            *kh,
            &trustees[1],
        )
        .unwrap(),
    );
    verify_board::<RistrettoCtx>(&messages, &cfg.protocol_manager).unwrap();
    assert!(!plaintexts_agreed(&result, &messages, &cfg));
    messages.pop();
    messages
        .push(Message::plaintexts_signed_msg(&cfg, 9, *ph, *df, *ch, *kh, &trustees[1]).unwrap());
    verify_board::<RistrettoCtx>(&messages, &cfg.protocol_manager).unwrap();
    assert!(plaintexts_agreed(&result, &messages, &cfg));
    messages.last_mut().unwrap().artifact = Some(vec![1]);
    assert!(verify_board::<RistrettoCtx>(&messages, &cfg.protocol_manager).is_err());
}

#[test]
fn statement_kinds_are_limited_to_their_signers() {
    let (manager, trustees, cfg, mut messages) = fixture();
    messages.push(Message::configuration_msg(&cfg, &manager).unwrap());
    assert!(verify_board::<RistrettoCtx>(&messages, &cfg.protocol_manager).is_err());
    messages.pop();
    let mut selected = [NULL_TRUSTEE; MAX_TRUSTEES];
    selected[0] = 1;
    selected[1] = 2;
    let mut ballots = Message::ballots_msg(
        &cfg,
        1,
        &Ballots::new(vec![]),
        selected,
        PublicKeyHash([0; 64]),
        &trustees[0],
    )
    .unwrap();
    assert!(ballots.verify(&cfg).is_err());
    ballots.artifact = None;
    assert!(ballots.verify(&cfg).is_err());
    let bytes = cfg.strand_serialize().unwrap();
    let valid = manager
        .sign(
            Statement::configuration_stmt(ConfigurationHash(
                strand::hash::hash_to_array(&bytes).unwrap(),
            )),
            Some(bytes),
        )
        .unwrap();
    assert!(valid.verify(&cfg).is_ok());
}

#[test]
fn message_from_outside_the_configuration_is_rejected() {
    let (_, _, cfg, mut messages) = fixture();
    let outsider = Manager::new(StrandSignatureSk::gen().unwrap());
    messages.push(Message::configuration_msg(&cfg, &outsider).unwrap());
    assert!(verify_board::<RistrettoCtx>(&messages, &cfg.protocol_manager).is_err());
}

#[test]
fn key_artifact_must_be_posted_by_the_first_trustee() {
    let (_, trustees, cfg, mut messages) = fixture();
    let ctx = RistrettoCtx;
    let pk = DkgPublicKey::new(ctx.generator().clone(), vec![]);
    let shares = SharesHashes([[1; 64]; MAX_TRUSTEES]);
    let channels = ChannelsHashes([[2; 64]; MAX_TRUSTEES]);
    messages
        .push(Message::public_key_msg(&cfg, &pk, &shares, &channels, false, &trustees[0]).unwrap());
    messages
        .push(Message::public_key_msg(&cfg, &pk, &shares, &channels, true, &trustees[1]).unwrap());
    messages
        .push(Message::public_key_msg(&cfg, &pk, &shares, &channels, false, &trustees[2]).unwrap());
    verify_board::<RistrettoCtx>(&messages, &cfg.protocol_manager).unwrap();
    assert!(agreed_public_key(&messages, &cfg).is_none());
}

#[test]
fn results_artifact_must_be_posted_by_the_first_selected_trustee() {
    let (manager, trustees, cfg, mut messages) = fixture();
    let mut selected = [NULL_TRUSTEE; MAX_TRUSTEES];
    selected[0] = 1;
    selected[1] = 2;
    let pk = PublicKeyHash([4; 64]);
    messages.push(
        Message::ballots_msg(&cfg, 9, &Ballots::new(vec![]), selected, pk, &manager).unwrap(),
    );
    let dfactors = DecryptionFactorsHashes([[5; 64]; MAX_TRUSTEES]);
    let ciphertexts = CiphertextsHash([6; 64]);
    let result = Message::plaintexts_msg(
        &cfg,
        9,
        Plaintexts(StrandVector(vec![])),
        dfactors,
        ciphertexts,
        pk,
        &trustees[1],
    )
    .unwrap();
    let Statement::Plaintexts(_, _, _, plaintexts, ..) = &result.statement else {
        unreachable!()
    };
    messages.push(
        Message::plaintexts_signed_msg(
            &cfg,
            9,
            *plaintexts,
            dfactors,
            ciphertexts,
            pk,
            &trustees[0],
        )
        .unwrap(),
    );
    messages.push(result.try_clone().unwrap());
    verify_board::<RistrettoCtx>(&messages, &cfg.protocol_manager).unwrap();
    assert!(!plaintexts_agreed(&result, &messages, &cfg));
}
