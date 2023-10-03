// SPDX-FileCopyrightText: 2022 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::ballot::*;
use crate::ballot_codec::RawBallotQuestion;
use crate::ballot_codec::*;
use crate::mixed_radix::{decode, encode};
use crate::plaintext::*;
use num_bigint::BigUint;

pub fn encode_bigint_to_bytes(b: &BigUint) -> Result<Vec<u8>, String> {
    Ok(b.to_radix_le(256))
}
pub fn decode_bigint_from_bytes(b: &[u8]) -> Result<BigUint, String> {
    BigUint::from_radix_le(b, 256)
        .ok_or(format!("Conversion failed for bytes {:?}", b))
}

pub trait BigUIntCodec {
    fn encode_plaintext_question_bigint(
        &self,
        plaintext: &DecodedVoteQuestion,
    ) -> Result<BigUint, String>;
    fn decode_plaintext_question_bigint(
        &self,
        bigint: &BigUint,
    ) -> Result<DecodedVoteQuestion, String>;
}

impl BigUIntCodec for Question {
    fn encode_plaintext_question_bigint(
        &self,
        plaintext: &DecodedVoteQuestion,
    ) -> Result<BigUint, String> {
        let raw_ballot = self.encode_to_raw_ballot(plaintext)?;
        encode(&raw_ballot.choices, &raw_ballot.bases)
    }

    fn decode_plaintext_question_bigint(
        &self,
        bigint: &BigUint,
    ) -> Result<DecodedVoteQuestion, String> {
        let mut bases = self.get_bases();
        let last_base = self.get_char_map().base();
        let choices = decode(&bases, &bigint, last_base)?;

        while bases.len() < choices.len() {
            bases.push(last_base);
        }

        let raw_ballot = RawBallotQuestion { bases, choices };

        self.decode_from_raw_ballot(&raw_ballot)
    }
}
