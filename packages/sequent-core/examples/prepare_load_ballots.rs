// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Stream independently encrypted ballots without a browser or a voter session.
//!
//! Usage: prepare_load_ballots STYLE.json CHOICES.json COUNT > ballots.jsonl
//! Inputs are the published BallotStyle and a Vec<DecodedVoteContest>. Only cast
//! content is emitted: encryption randomness and auditable plaintext stay private.

use anyhow::{ensure, Context, Result};
use sequent_core::ballot::{
    sign_hashable_ballot_with_ephemeral_voter_signing_key, BallotStyle,
    ContestEncryptionPolicy, HashableBallot, SignedHashableBallot,
    VoterSigningPolicy,
};
use sequent_core::encrypt::{
    encrypt_decoded_contest, encrypt_decoded_multi_contest,
};
use sequent_core::multi_ballot::{
    sign_hashable_multi_ballot_with_ephemeral_voter_signing_key,
    HashableMultiBallot, SignedHashableMultiBallot,
};
use sequent_core::plaintext::DecodedVoteContest;
use std::io::{self, BufWriter, Write};
use strand::backend::ristretto::RistrettoCtx;

/// Use the portal's encryption and signing policies, with fresh randomness per row.
fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    ensure!(
        args.len() == 4,
        "Usage: prepare_load_ballots STYLE CHOICES COUNT"
    );
    let style: BallotStyle =
        serde_json::from_reader(std::fs::File::open(&args[1])?)?;
    let choices: Vec<DecodedVoteContest> =
        serde_json::from_reader(std::fs::File::open(&args[2])?)?;
    let count: usize = args[3].parse().context("COUNT must be positive")?;
    ensure!(count > 0, "COUNT must be positive");
    let presentation = style
        .election_event_presentation
        .clone()
        .unwrap_or_default();
    let multi = presentation.contest_encryption_policy
        == Some(ContestEncryptionPolicy::MULTIPLE_CONTESTS);
    let sign = presentation.voter_signing_policy
        == Some(VoterSigningPolicy::WITH_SIGNATURE);
    let ctx = RistrettoCtx::default();
    let mut output = BufWriter::new(io::stdout().lock());
    for _ in 0..count {
        let (id, content) = if multi {
            let ballot = encrypt_decoded_multi_contest(&ctx, &choices, &style)?;
            let mut signed = SignedHashableMultiBallot::try_from(&ballot)?;
            if sign {
                let hashable = HashableMultiBallot::try_from(&signed)?;
                let signature = sign_hashable_multi_ballot_with_ephemeral_voter_signing_key(
                    &ballot.ballot_hash, &style.election_id, &hashable,
                ).map_err(anyhow::Error::msg)?;
                signed.voter_signing_pk = Some(signature.public_key);
                signed.voter_ballot_signature = Some(signature.signature);
            }
            (ballot.ballot_hash, serde_json::to_string(&signed)?)
        } else {
            let ballot = encrypt_decoded_contest(&ctx, &choices, &style)?;
            let mut signed = SignedHashableBallot::try_from(&ballot)?;
            if sign {
                let hashable = HashableBallot::try_from(&signed)?;
                let signature =
                    sign_hashable_ballot_with_ephemeral_voter_signing_key(
                        &ballot.ballot_hash,
                        &style.election_id,
                        &hashable,
                    )
                    .map_err(anyhow::Error::msg)?;
                signed.voter_signing_pk = Some(signature.public_key);
                signed.voter_ballot_signature = Some(signature.signature);
            }
            (ballot.ballot_hash, serde_json::to_string(&signed)?)
        };
        serde_json::to_writer(
            &mut output,
            &serde_json::json!({
                "ballotId": id, "content": content, "electionId": style.election_id,
            }),
        )?;
        writeln!(output)?;
    }
    output.flush()?;
    Ok(())
}
