// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Portable public evidence and the same verification report on native/WASM.

use anyhow::{ensure, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use cryptography::groups::Ristretto255Group;
use cryptography::utils::serialization::{Deserializable, Serializable};
use serde::{Deserialize, Serialize};
use wbraid::messages::{
    artifact::Ballots,
    newtypes::{hash_bytes, ConfigurationHash},
    predicate::Predicate,
    wire::ProtocolMessage,
};
use wbraid::verification::{verify_dkg, verify_tally};

use crate::{BallotCiphertext, Ctx, HashHex};

/// Canonical protocol messages, encoded in padded standard base64. Trust
/// anchors deliberately live in the separate request, not in this evidence.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublicTranscript {
    pub version: u32,
    pub dkg_messages: Vec<String>,
    pub tally_messages: Option<Vec<String>>,
}

impl PublicTranscript {
    pub fn new(dkg: Vec<Vec<u8>>, tally: Option<Vec<Vec<u8>>>) -> Self {
        Self {
            version: 1,
            dkg_messages: encode(dkg),
            tally_messages: tally.map(encode),
        }
    }
}

fn encode(messages: Vec<Vec<u8>>) -> Vec<String> {
    messages.into_iter().map(|m| STANDARD.encode(m)).collect()
}
fn decode(messages: &[String]) -> Result<Vec<ProtocolMessage<Ctx>>> {
    messages
        .iter()
        .map(|m| Ok(ProtocolMessage::deser(&STANDARD.decode(m)?)?))
        .collect()
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerificationRequest {
    /// Obtained independently from the trusted election configuration.
    pub configuration_hash: HashHex,
    /// Decimal u128, avoiding JavaScript number precision loss. For the
    /// platform this is the protocol_board row UUID as an unsigned integer.
    pub tally_id: Option<String>,
    /// Optional independent pin of the signed Ballots artifact's body hash.
    #[serde(default)]
    pub ballots_hash: Option<HashHex>,
    /// Stored cast-vote ciphertexts to locate in this batch's input.
    #[serde(default)]
    pub ballot_ciphertexts: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationStatus {
    Verified,
    Incomplete,
    Invalid,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BallotInclusion {
    pub ciphertext_hash: String,
    pub occurrences: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VerificationReport {
    pub status: VerificationStatus,
    pub detail: String,
    pub configuration_hash: HashHex,
    pub tally_id: Option<String>,
    pub public_key_hash: Option<HashHex>,
    pub ballot_count: Option<usize>,
    /// Base64 of decoded 30-byte payloads, in final mix order.
    pub plaintexts: Vec<String>,
    pub ballot_inclusions: Vec<BallotInclusion>,
}

/// A malformed or dishonest transcript is a report with `invalid` status;
/// `verified` is reserved for all required proofs and publications completing.
pub fn verify_transcript(
    transcript: &PublicTranscript,
    request: &VerificationRequest,
) -> VerificationReport {
    let mut report = VerificationReport {
        status: VerificationStatus::Incomplete,
        detail: "DKG transcript is incomplete".into(),
        configuration_hash: request.configuration_hash.clone(),
        tally_id: request.tally_id.clone(),
        public_key_hash: None,
        ballot_count: None,
        plaintexts: Vec::new(),
        ballot_inclusions: Vec::new(),
    };
    if let Err(error) = check(transcript, request, &mut report) {
        report.status = VerificationStatus::Invalid;
        report.detail = error.to_string();
        report.plaintexts.clear();
        report.ballot_inclusions.clear();
    }
    report
}

fn check(
    transcript: &PublicTranscript,
    request: &VerificationRequest,
    report: &mut VerificationReport,
) -> Result<()> {
    ensure!(transcript.version == 1, "unsupported transcript version");
    ensure!(
        transcript.tally_messages.is_some() == request.tally_id.is_some(),
        "tally evidence requires an independently supplied tally identifier"
    );
    ensure!(
        transcript.tally_messages.is_some()
            || (request.ballots_hash.is_none()
                && request.ballot_ciphertexts.is_empty()),
        "ballot checks require tally evidence"
    );
    let dkg_messages = decode(&transcript.dkg_messages)?;
    // Decode all evidence even if the DKG is unfinished.
    let tally_messages = transcript
        .tally_messages
        .as_ref()
        .map(|m| decode(m))
        .transpose()?;
    let Some(dkg) = verify_dkg(
        &dkg_messages,
        ConfigurationHash(request.configuration_hash.to_hash()),
    )?
    else {
        return Ok(());
    };
    report.public_key_hash = Some(HashHex::of(&dkg.public_key_hash().0));
    let Some(tally_messages) = tally_messages else {
        report.status = VerificationStatus::Verified;
        report.detail =
            "Public DKG commitments, proofs and derived keys verified".into();
        return Ok(());
    };
    report.detail = "Tally transcript is incomplete".into();
    let tally_id = request
        .tally_id
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("missing tally identifier"))?
        .parse::<u128>()?;
    let Some(plaintexts) =
        verify_tally::<Ctx, 1>(&dkg, &tally_messages, tally_id)?
    else {
        return Ok(());
    };
    // This projection is reached only after the core verified the full tally.
    let input = tally_messages
        .iter()
        .find(|m| {
            m.message_type == wbraid::messages::wire::MessageType::Ballots
        })
        .ok_or_else(|| anyhow::anyhow!("missing ballots"))?;
    let (Predicate::Ballots(head), Some(body)) =
        wbraid::board::verify::verify(input, dkg.configuration())?
    else {
        unreachable!()
    };
    ensure!(
        request
            .ballots_hash
            .as_ref()
            .is_none_or(|h| h.to_hash() == head.ciphertexts.0),
        "ballots do not match the trusted batch hash"
    );
    let ballots = Ballots::<Ctx, 1>::deser(&body)?;
    report.ballot_count = Some(ballots.ciphertexts.len());
    for encoded in &request.ballot_ciphertexts {
        let ballot = BallotCiphertext::parse(encoded)?;
        let bytes = ballot.ciphertext.ser();
        let occurrences = ballots
            .ciphertexts
            .iter()
            .filter(|c| c.ser() == bytes)
            .count();
        ensure!(occurrences > 0, "requested ballot ciphertext is absent from the verified tally input");
        report.ballot_inclusions.push(BallotInclusion {
            ciphertext_hash: hex::encode(hash_bytes(&bytes)),
            occurrences,
        });
    }
    report.plaintexts = plaintexts
        .0
        .into_iter()
        .map(|row| {
            Ok(STANDARD.encode(Ristretto255Group::decode_30_bytes(&row[0])?))
        })
        .collect::<Result<Vec<_>>>()?;
    report.status = VerificationStatus::Verified;
    report.detail = "DKG, ballot proofs, ordered shuffles, endorsements, threshold decryption and plaintexts verified".into();
    Ok(())
}

#[cfg(feature = "browser")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub struct WasmVerifier;

#[cfg(feature = "browser")]
#[wasm_bindgen::prelude::wasm_bindgen]
impl WasmVerifier {
    /// Derive a pin from a configuration message obtained independently from
    /// a trusted election record, rather than from the board being audited.
    #[wasm_bindgen::prelude::wasm_bindgen(js_name = configurationHash)]
    pub fn configuration_hash(
        encoded: &str,
    ) -> std::result::Result<String, wasm_bindgen::JsValue> {
        let run = || -> Result<String> {
            let message =
                ProtocolMessage::<Ctx>::deser(&STANDARD.decode(encoded)?)?;
            let configuration = message.verify_configuration()?;
            Ok(hex::encode(
                ConfigurationHash::from_configuration(&configuration)?.0,
            ))
        };
        run().map_err(|e| wasm_bindgen::JsValue::from_str(&e.to_string()))
    }

    /// Pin the Ballots body from the trusted platform's original publication.
    /// The full verifier subsequently checks the manager's signature as well.
    #[wasm_bindgen::prelude::wasm_bindgen(js_name = ballotsHash)]
    pub fn ballots_hash(
        encoded: &str,
    ) -> std::result::Result<String, wasm_bindgen::JsValue> {
        let run = || -> Result<String> {
            let message =
                ProtocolMessage::<Ctx>::deser(&STANDARD.decode(encoded)?)?;
            ensure!(
                message.message_type
                    == wbraid::messages::wire::MessageType::Ballots,
                "expected a Ballots message"
            );
            let body = message
                .body
                .ok_or_else(|| anyhow::anyhow!("missing Ballots body"))?;
            Ok(hex::encode(hash_bytes(&body)))
        };
        run().map_err(|e| wasm_bindgen::JsValue::from_str(&e.to_string()))
    }
    /// Lossless public evidence, including every Configuration publication.
    /// Fetching alone does not authenticate or verify the export.
    #[wasm_bindgen::prelude::wasm_bindgen(js_name = fetchTranscript)]
    pub async fn fetch_transcript(
        board_url: String,
        dkg_board: String,
        tally_board: Option<String>,
    ) -> std::result::Result<String, wasm_bindgen::JsValue> {
        let run = async {
            let dkg = crate::BoardName::parse(&dkg_board)?;
            let tally = tally_board
                .as_deref()
                .map(crate::BoardName::parse)
                .transpose()?;
            let dkg = wbraid::wasm::transport::WasmHttpTransport::new(
                &board_url,
                dkg.as_str(),
            )
            .fetch_raw()
            .await?;
            let tally = match tally {
                Some(board) => Some(
                    wbraid::wasm::transport::WasmHttpTransport::new(
                        &board_url,
                        board.as_str(),
                    )
                    .fetch_raw()
                    .await?,
                ),
                None => None,
            };
            Ok::<_, anyhow::Error>(serde_json::to_string(
                &PublicTranscript::new(dkg, tally),
            )?)
        };
        run.await
            .map_err(|e| wasm_bindgen::JsValue::from_str(&e.to_string()))
    }
    /// Verify an offline export, without private keys or trustee registration.
    #[wasm_bindgen::prelude::wasm_bindgen(js_name = verifyTranscript)]
    pub fn verify_transcript(
        evidence: &str,
        anchors: &str,
    ) -> std::result::Result<String, wasm_bindgen::JsValue> {
        let run = || -> Result<String> {
            let evidence = serde_json::from_str(evidence)?;
            let anchors = serde_json::from_str(anchors)?;
            Ok(serde_json::to_string(&verify_transcript(
                &evidence, &anchors,
            ))?)
        };
        run().map_err(|e| wasm_bindgen::JsValue::from_str(&e.to_string()))
    }
}
