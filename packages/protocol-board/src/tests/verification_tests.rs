// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use cryptography::utils::serialization::Serializable;
use wbraid::messages::artifact::{
    DkgPublicKey, Mix, PartialDecryption, Shares,
};
use wbraid::messages::wire::{
    BallotsHead, MixHead, PartialDecryptionsHead, PlaintextsHead,
};
use wbraid::verification::{verify_dkg, verify_tally};

async fn completed(
    count: usize,
    threshold: usize,
    payloads: &[[u8; 30]],
) -> (Keyed, Vec<ProtocolMessage<Ctx>>, u128, Vec<String>) {
    let keyed = Keyed::new(count, threshold).await;
    let ballots = keyed.ballots(payloads);
    let stored = ballots.iter().map(|b| b.encode()).collect();
    let chosen = (1..=threshold)
        .rev()
        .map(|i| format!("trustee{}", if i == threshold { count } else { i }))
        .collect::<Vec<_>>();
    let refs = chosen.iter().map(String::as_str).collect::<Vec<_>>();
    let tally = keyed.tally(&refs, ballots);
    tally.post_ballots();
    tally.run(&mut keyed.sessions(&tally).await).await;
    let id = BallotsHead::deser(&tally.board.input.message().head)
        .unwrap()
        .tally_id;
    let messages = tally.child.snapshot();
    (keyed, messages, id, stored)
}

fn signer(keyed: &Keyed, message: &ProtocolMessage<Ctx>) -> Trustee<Ctx> {
    keyed.ceremony.trustee(
        keyed
            .ceremony
            .configuration()
            .get_trustee_position(&message.sender.pk)
            .unwrap(),
    )
}

fn check(
    keyed: &Keyed,
    messages: &[ProtocolMessage<Ctx>],
    id: u128,
) -> anyhow::Result<Option<Plaintexts<Ctx, 1>>> {
    let dkg = verify_dkg(
        &keyed.ceremony.board.snapshot(),
        keyed.ceremony.configuration_hash(),
    )?
    .unwrap();
    verify_tally::<Ctx, 1>(&dkg, messages, id)
}

#[tokio::test]
async fn public_verifier_handles_threshold_and_committee_boundaries() {
    for (count, threshold) in [(2, 2), (4, 2), (4, 3), (4, 4), (8, 2), (8, 8)] {
        let (keyed, messages, id, _) =
            completed(count, threshold, &[[3; 30]]).await;
        let plaintexts = check(&keyed, &messages, id).unwrap().unwrap();
        assert_eq!(
            Ristretto255Group::decode_30_bytes(&plaintexts.0[0][0]).unwrap(),
            [3; 30]
        );
    }
}

#[tokio::test]
async fn public_verifier_rejects_invalid_configuration_before_generic_dispatch()
{
    let keyed = Keyed::new(3, 2).await;
    for (threshold, width) in [(1, 1), (4, 1), (2, 0), (2, 9)] {
        let mut cfg = keyed.ceremony.configuration();
        cfg.threshold = threshold;
        cfg.ciphertext_width = width;
        let expected = ConfigurationHash::from_configuration(&cfg).unwrap();
        let message =
            ProtocolMessage::configuration(&keyed.ceremony.manager, DATE, &cfg);
        assert!(verify_dkg(&[message], expected).is_err());
    }
}

#[tokio::test]
async fn public_verifier_checks_a_complete_noncontiguous_quorum() {
    let keyed = Keyed::new(3, 2).await;
    let tally = keyed.tally(
        &["trustee3", "trustee1"],
        keyed.ballots(&[[7; 30], [9; 30]]),
    );
    tally.post_ballots();
    let mut sessions = keyed.sessions(&tally).await;
    tally.run(&mut sessions).await;
    let dkg = verify_dkg(
        &keyed.ceremony.board.snapshot(),
        keyed.ceremony.configuration_hash(),
    )
    .unwrap()
    .expect("completed DKG");
    let input = wbraid::messages::wire::BallotsHead::deser(
        &tally.board.input.message().head,
    )
    .unwrap();
    let plaintexts =
        verify_tally::<Ctx, 1>(&dkg, &tally.child.snapshot(), input.tally_id)
            .unwrap()
            .expect("completed tally");
    let mut decoded = plaintexts
        .0
        .into_iter()
        .map(|row| Ristretto255Group::decode_30_bytes(&row[0]).unwrap())
        .collect::<Vec<_>>();
    decoded.sort();
    assert_eq!(decoded, vec![[7; 30], [9; 30]]);
}

#[tokio::test]
async fn public_verifier_rejects_false_plaintexts_even_with_unanimous_signatures(
) {
    let (keyed, mut messages, id, _) =
        completed(3, 2, &[[7; 30], [9; 30]]).await;
    let false_plaintexts = Plaintexts::<Ctx, 1>(vec![
        [Ctx::random_element()],
        [Ctx::random_element()],
    ]);
    for message in messages
        .iter_mut()
        .filter(|m| m.message_type == MessageType::Plaintexts)
    {
        let head = PlaintextsHead::deser(&message.head).unwrap();
        *message = ProtocolMessage::plaintexts(
            &signer(&keyed, message),
            DATE,
            head.configuration,
            head.public_key,
            head.ciphertexts,
            &false_plaintexts,
        );
    }
    let error = check(&keyed, &messages, id).unwrap_err().to_string();
    assert!(
        error.contains("independently verified decryption"),
        "{error}"
    );
}

#[tokio::test]
async fn public_verifier_checks_resigned_shuffle_and_decryption_proofs() {
    let (keyed, messages, id, _) = completed(3, 2, &[[7; 30], [9; 30]]).await;
    let mixes = messages
        .iter()
        .filter(|m| m.message_type == MessageType::Mix)
        .collect::<Vec<_>>();
    let mut bad = messages.clone();
    let message = bad
        .iter_mut()
        .find(|m| m.message_type == MessageType::Mix)
        .unwrap();
    let head = MixHead::deser(&message.head).unwrap();
    let mut mix = Mix::<Ctx, 1>::deser(message.body.as_ref().unwrap()).unwrap();
    mix.proof = Mix::<Ctx, 1>::deser(mixes[1].body.as_ref().unwrap())
        .unwrap()
        .proof;
    *message = ProtocolMessage::mix(
        &signer(&keyed, message),
        DATE,
        head.configuration,
        head.public_key,
        head.input,
        &mix,
    );
    assert!(check(&keyed, &bad, id)
        .unwrap_err()
        .to_string()
        .contains("shuffle"));
    let mut bad = messages.clone();
    let message = bad
        .iter_mut()
        .find(|m| m.message_type == MessageType::PartialDecryptions)
        .unwrap();
    let head = PartialDecryptionsHead::deser(&message.head).unwrap();
    let mut partial =
        PartialDecryption::<Ctx, 1>::deser(message.body.as_ref().unwrap())
            .unwrap();
    partial.factors[0][0] = Ctx::random_element();
    *message = ProtocolMessage::partial_decryptions(
        &signer(&keyed, message),
        DATE,
        head.configuration,
        head.public_key,
        head.ciphertexts,
        &partial,
    );
    assert!(check(&keyed, &bad, id).is_err());
}

#[tokio::test]
async fn public_verifier_checks_commitment_proofs_and_rederives_public_keys() {
    let keyed = Keyed::new(3, 2).await;
    let cfg = keyed.ceremony.configuration_hash();
    let mut messages = keyed.ceremony.board.snapshot();
    let message = messages
        .iter_mut()
        .find(|m| m.message_type == MessageType::Shares)
        .unwrap();
    let mut shares =
        Shares::<Ctx>::deser(message.body.as_ref().unwrap()).unwrap();
    shares.commitments[0].value = Ctx::random_element();
    *message =
        ProtocolMessage::shares(&signer(&keyed, message), DATE, cfg, &shares);
    assert!(verify_dkg(&messages, cfg)
        .err()
        .unwrap()
        .to_string()
        .contains("commitment proof"));
    let mut messages = keyed.ceremony.board.snapshot();
    let false_key = DkgPublicKey::<Ctx>::new(
        Ctx::random_element(),
        (0..3).map(|_| Ctx::random_element()).collect(),
    );
    for message in messages
        .iter_mut()
        .filter(|m| m.message_type == MessageType::PublicKey)
    {
        *message = ProtocolMessage::public_key(
            &signer(&keyed, message),
            DATE,
            cfg,
            &false_key,
        );
    }
    assert!(verify_dkg(&messages, cfg)
        .err()
        .unwrap()
        .to_string()
        .contains("commitments"));
    assert!(verify_dkg(
        &keyed.ceremony.board.snapshot(),
        ConfigurationHash(hash_bytes(b"another election"))
    )
    .is_err());
}

#[tokio::test]
async fn public_verifier_rejects_bad_ballots_and_duplicate_quorum_and_sibling_replays(
) {
    let (keyed, messages, id, _) = completed(3, 2, &[[7; 30]]).await;
    for variant in 0..3 {
        let mut bad = messages.clone();
        let message = bad
            .iter_mut()
            .find(|m| m.message_type == MessageType::Ballots)
            .unwrap();
        let head = BallotsHead::deser(&message.head).unwrap();
        let ballots = if variant == 0 {
            let foreign = naoryung::PublicKey::<Ctx>::augment(
                &elgamal::PublicKey::new(keyed.joint_key.clone()),
                b"wrong domain",
            )
            .unwrap();
            Ballots::<Ctx, 1>::new(vec![foreign
                .encrypt(&[Ctx::random_element()], b"wrong domain")
                .unwrap()])
        } else {
            Ballots::<Ctx, 1>::deser(message.body.as_ref().unwrap()).unwrap()
        };
        let quorum = if variant == 1 {
            vec![1, 1]
        } else {
            head.trustees
        };
        let new_id = if variant == 2 { id.wrapping_add(1) } else { id };
        *message = ProtocolMessage::ballots(
            &keyed.ceremony.manager,
            DATE,
            head.configuration,
            head.public_key,
            quorum,
            new_id,
            &ballots,
        );
        assert!(check(&keyed, &bad, new_id).is_err(), "variant {variant}");
    }
    assert!(check(&keyed, &messages, id.wrapping_add(1))
        .unwrap_err()
        .to_string()
        .contains("trusted identifier"));
}

#[tokio::test]
async fn public_verifier_never_passes_truncated_transcripts_and_handles_empty_batches(
) {
    let (keyed, messages, id, _) = completed(2, 2, &[]).await;
    assert!(check(&keyed, &messages, id).unwrap().unwrap().0.is_empty());
    for missing in [
        MessageType::Ballots,
        MessageType::Mix,
        MessageType::MixSignature,
        MessageType::PartialDecryptions,
        MessageType::Plaintexts,
    ] {
        let mut truncated = messages.clone();
        truncated.remove(
            truncated
                .iter()
                .position(|m| m.message_type == missing)
                .unwrap(),
        );
        assert!(
            check(&keyed, &truncated, id).unwrap().is_none(),
            "{missing:?}"
        );
    }
    let mut dkg = keyed.ceremony.board.snapshot();
    dkg.remove(
        dkg.iter()
            .position(|m| m.message_type == MessageType::PublicKey)
            .unwrap(),
    );
    assert!(verify_dkg(&dkg, keyed.ceremony.configuration_hash())
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn public_verifier_rejects_equivocation_manager_roles_and_bad_signatures()
{
    let (keyed, messages, id, _) = completed(3, 2, &[[7; 30]]).await;
    let mut bad = messages.clone();
    bad[0].head[0] ^= 1;
    assert!(check(&keyed, &bad, id).is_err());
    let mut bad = messages.clone();
    let first = bad
        .iter()
        .find(|m| m.message_type == MessageType::Mix)
        .unwrap();
    let head = MixHead::deser(&first.head).unwrap();
    let body = Mix::<Ctx, 1>::deser(first.body.as_ref().unwrap()).unwrap();
    bad.push(ProtocolMessage::mix(
        &signer(&keyed, first),
        DATE,
        head.configuration,
        head.public_key,
        CiphertextsHash(hash_bytes(b"fork")),
        &body,
    ));
    assert!(check(&keyed, &bad, id)
        .unwrap_err()
        .to_string()
        .contains("equivocating"));
    let mut bad = messages.clone();
    bad.push(ProtocolMessage::mix(
        &keyed.ceremony.manager,
        DATE,
        head.configuration,
        head.public_key,
        head.input,
        &body,
    ));
    assert!(check(&keyed, &bad, id)
        .unwrap_err()
        .to_string()
        .contains("committee member"));
    let mut duplicates = messages.clone();
    duplicates.extend(messages.clone());
    duplicates.reverse();
    assert!(check(&keyed, &duplicates, id).unwrap().is_some());
}

#[tokio::test]
async fn portable_report_checks_ballot_inclusion_and_pinned_batch_hash() {
    use crate::verification::*;
    let (keyed, messages, id, stored) =
        completed(3, 2, &[[7; 30], [9; 30]]).await;
    let evidence = PublicTranscript::new(
        keyed
            .ceremony
            .board
            .snapshot()
            .iter()
            .map(|m| m.ser())
            .collect(),
        Some(messages.iter().map(|m| m.ser()).collect()),
    );
    let input = messages
        .iter()
        .find(|m| m.message_type == MessageType::Ballots)
        .unwrap();
    let mut request = VerificationRequest {
        configuration_hash: HashHex::of(&keyed.ceremony.configuration_hash().0),
        tally_id: Some(id.to_string()),
        ballots_hash: Some(HashHex::of(&hash_bytes(
            input.body.as_ref().unwrap(),
        ))),
        ballot_ciphertexts: stored,
    };
    let report = verify_transcript(&evidence, &request);
    assert_eq!(report.status, VerificationStatus::Verified, "{report:?}");
    assert_eq!(report.ballot_count, Some(2));
    assert_eq!(report.plaintexts.len(), 2);
    assert!(report.ballot_inclusions.iter().all(|b| b.occurrences == 1));
    // Explicit export permits real CLI and browser integration tests to use
    // precisely the same canonical, proof-bearing messages as these tests.
    if let Ok(directory) = std::env::var("WBRAID_VERIFIER_TEST_EXPORT_DIR") {
        std::fs::create_dir_all(&directory).unwrap();
        for (name, value) in [
            ("transcript", serde_json::to_value(&evidence).unwrap()),
            ("request", serde_json::to_value(&request).unwrap()),
            ("report", serde_json::to_value(&report).unwrap()),
        ] {
            std::fs::write(
                format!("{directory}/{name}.json"),
                serde_json::to_vec_pretty(&value).unwrap(),
            )
            .unwrap();
        }
    }
    request.ballot_ciphertexts = vec![keyed.ballots(&[[5; 30]])[0].encode()];
    let report = verify_transcript(&evidence, &request);
    assert_eq!(report.status, VerificationStatus::Invalid);
    assert!(report.detail.contains("absent"));
    assert!(report.plaintexts.is_empty());
    request.ballot_ciphertexts.clear();
    request.ballots_hash = Some(HashHex::of(&hash_bytes(b"substituted batch")));
    assert_eq!(
        verify_transcript(&evidence, &request).status,
        VerificationStatus::Invalid
    );
    request.ballots_hash = None;
    request.tally_id = None;
    assert_eq!(
        verify_transcript(&evidence, &request).status,
        VerificationStatus::Invalid
    );
}
