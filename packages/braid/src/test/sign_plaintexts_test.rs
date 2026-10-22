// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use b3::messages::artifact::{Ballots, Configuration, Plaintexts};
use b3::messages::message::Message;
use b3::messages::newtypes::{DecryptionFactorsHashes, PublicKeyHash, MAX_TRUSTEES, NULL_TRUSTEE};
use b3::messages::statement::Statement;
use strand::backend::ristretto::RistrettoCtx;
use strand::context::Ctx;
use strand::elgamal::Ciphertext;
use strand::serialization::{StrandDeserialize, StrandSerialize};

use crate::protocol::datalog::NULL_HASH;
use crate::protocol::trustee2::Trustee;
use crate::test::protocol_test_memory::create_protocol_test;
use crate::test::vector_board::VectorBoard;

const N_TRUSTEES: usize = 3;
const SELECTED: [usize; 2] = [1, 2];
const DECRYPTOR: usize = SELECTED[0] - 1;
const BATCH: usize = 1;
const CIPHERTEXTS: usize = 4;
const MAX_CYCLES: usize = 30;

type EditHashes = fn(&mut DecryptionFactorsHashes);

struct DecryptionRun {
    plaintexts_signed: usize,
    step_errors: Vec<String>,
}

fn resign_plaintexts(
    message: Message,
    cfg: &Configuration<RistrettoCtx>,
    signer: &Trustee<RistrettoCtx>,
    edit: EditHashes,
) -> Message {
    let Statement::Plaintexts(_, _, batch, _, mut dfactors_hs, cipher_h, pk_h) =
        message.statement.clone()
    else {
        return message;
    };
    let artifact = message
        .artifact
        .as_ref()
        .expect("plaintexts message has an artifact");
    let plaintexts = Plaintexts::<RistrettoCtx>::strand_deserialize(artifact).unwrap();
    edit(&mut dfactors_hs);

    Message::plaintexts_msg(cfg, batch, plaintexts, dfactors_hs, cipher_h, pk_h, signer).unwrap()
}

fn step_all(
    trustees: &mut [Trustee<RistrettoCtx>],
    last_ids: &mut [i64],
    board: &mut VectorBoard,
    cfg: &Configuration<RistrettoCtx>,
    edit: Option<EditHashes>,
    step_errors: &mut Vec<String>,
) {
    for (position, trustee) in trustees.iter_mut().enumerate() {
        let messages = board.get(last_ids[position]);
        last_ids[position] += messages.len() as i64;
        match trustee.step(&messages) {
            Ok(result) => {
                for message in result.messages {
                    let message = match edit {
                        Some(edit) if position == DECRYPTOR => {
                            resign_plaintexts(message, cfg, trustee, edit)
                        }
                        _ => message,
                    };
                    board.add(message);
                }
            }
            Err(error) => step_errors.push(error.to_string()),
        }
    }
}

fn count_plaintexts_signed(board: &VectorBoard) -> usize {
    board
        .messages
        .iter()
        .map(|m| Message::strand_deserialize(&m.message).unwrap())
        .filter(|m| matches!(m.statement, Statement::PlaintextsSigned(..)))
        .count()
}

fn run_decryption(edit: Option<EditHashes>) -> DecryptionRun {
    let ctx = RistrettoCtx;
    let test = create_protocol_test(N_TRUSTEES, &SELECTED, ctx.clone()).unwrap();
    let cfg = test.cfg;
    let mut board = test.remote;
    let mut trustees = test.trustees;
    let mut last_ids = vec![-1i64; trustees.len()];
    let mut step_errors = vec![];

    let mut dkg_pk = None;
    for _ in 0..MAX_CYCLES {
        step_all(
            &mut trustees,
            &mut last_ids,
            &mut board,
            &cfg,
            edit,
            &mut step_errors,
        );
        dkg_pk = trustees[0]._get_dkg_public_key_nohash();
        if dkg_pk.is_some() {
            break;
        }
    }
    let dkg_pk = dkg_pk.expect("the key ceremony completes");
    let pk_h = strand::hash::hash_to_array(&dkg_pk.strand_serialize().unwrap()).unwrap();
    let pk = strand::elgamal::PublicKey::from_element(&dkg_pk.pk, &ctx);

    let mut rng = ctx.get_rng();
    let ballots: Vec<Ciphertext<RistrettoCtx>> = (0..CIPHERTEXTS)
        .map(|_| pk.encrypt(&ctx.encode(&ctx.rnd_plaintext(&mut rng)).unwrap()))
        .collect();
    let mut selected = [NULL_TRUSTEE; MAX_TRUSTEES];
    selected[0..SELECTED.len()].copy_from_slice(&SELECTED);
    let message = Message::ballots_msg(
        &cfg,
        BATCH,
        &Ballots::new(ballots),
        selected,
        PublicKeyHash(crate::util::hash_from_vec(&pk_h).unwrap()),
        &test.protocol_manager,
    )
    .unwrap();
    board.add(message);

    let expected_signatures = N_TRUSTEES - 1;
    for _ in 0..MAX_CYCLES {
        step_all(
            &mut trustees,
            &mut last_ids,
            &mut board,
            &cfg,
            edit,
            &mut step_errors,
        );
        let has_plaintexts = trustees[DECRYPTOR]
            ._get_plaintexts_nohash(BATCH, DECRYPTOR)
            .is_some();
        if has_plaintexts && count_plaintexts_signed(&board) >= expected_signatures {
            break;
        }
    }

    DecryptionRun {
        plaintexts_signed: count_plaintexts_signed(&board),
        step_errors,
    }
}

#[test]
fn signs_plaintexts_with_one_decryption_factor_per_selected_trustee() {
    let run = run_decryption(None);

    assert_eq!(run.plaintexts_signed, N_TRUSTEES - 1);
}

#[test]
fn rejects_plaintexts_with_fewer_decryption_factors_than_threshold() {
    let run = run_decryption(Some(|dfactors_hs| {
        dfactors_hs.0[1..].fill(NULL_HASH);
    }));

    assert_eq!(run.plaintexts_signed, 0);
    assert!(run
        .step_errors
        .iter()
        .any(|e| e.contains("Unexpected number of decryption factors: 1, expected 2")));
}
