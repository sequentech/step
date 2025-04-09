#![allow(non_camel_case_types)]

// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use b3::messages::artifact::DkgPublicKey;
use base64::engine::general_purpose;
use base64::Engine;
use colored::*;
use serde::Serialize;
use strum::Display;
use tracing::info;

use b3::messages::artifact::Configuration;
use b3::messages::message::Message;
use b3::messages::message::VerifiedMessage;
use b3::messages::newtypes::*;
use b3::messages::statement::StatementType;

use crate::protocol::board::grpc_m::GrpcB3;
use crate::protocol::board::Board;
use crate::protocol::predicate::Predicate;
use crate::protocol::trustee2::Trustee;

use crate::util::dbg_hash;
use crate::verify::datalog::Target;
use crate::verify::datalog::Verified;

use strand::context::Ctx;
use strand::serialization::StrandDeserialize;
use strand::serialization::StrandSerialize;

/*
Verifies the election data published on the bulletin board to implement universal verifiability. The key elements
that need to be verified are:

1) Public key verification
    1.1 The public key was correctly constructed (given the public data).
    1.2 The individual trustee verification keys were correctly constructed (given the public data).
2) Ballots
    2.1 The ballots are associated to the public key by the protocol manager.
3) Mixing
    3.1 The ballots are the source of ciphertexts in the first link in the mixing chain.
    3.2 The proof of shuffle for each link in the mixing chain verifies.
    3.3 The number of links in the mixing chain is equal to the specified threshold.
    3.4 There are no duplicate trustees in the mixing chain.
4) Decryption
    4.1 The ciphertexts used as the input to decryption correspond to the output of the last link in the mixing chain.
    4.2 The proof of decryption linking the decryption ciphertexts to the plaintexts verifies with respect to the trustee verification keys.
    4.3 The combination of decryption factors matches the published plaintexts.

The verifier performs checks that map to these steps, as well as additional consistency checks. See below.
*/

///////////////////////////////////////////////////////////////////////////
// Check symbolic constants
///////////////////////////////////////////////////////////////////////////

#[derive(PartialEq, Eq, Hash, Display, Serialize)]
enum Check {
    /*
    The configuration is valid as per Configuration::is_valid:
    1) The number of trustees ranges from 2 to 12 (crate::protocol::MAX_TRUSTEES).
    2) The threshold ranges from 2 to the number of trustees.
    3) There are no duplicate trustees.
    */
    CONFIGURATION_VALID,
    /*
    All message signatures verify, with respect to their specified sender public key.
    */
    MESSAGE_SIGNATURES_VALID,
    /*
    All messages refer to the same configuration file that is checked
    with CONFIGURATION_VALID.
    */
    MESSAGES_CFG_VALID,
    /*
    The public key information has been correctly constructed (given public data):
    1) The public key has been correctly constructed.
    2) The trustee verification keys have been correctly constructed.
    3) All trustees have signed the public key statement, which asserts correctness
    of private shares (VSS).
    */
    PK_VALID,
    /*
    The protocol manager's signature on the ballots and public key verifies.
    */
    BALLOTS_PK_VALID,
    /*
    The first link in the mixing chain takes the ballots as input.
    */
    MIX_START_VALID,
    /*
    The last link in the mixing chain outputs the ciphertexts inputted to decryption.
    */
    MIX_END_VALID,
    /*
    1) The number of links in the mixing chain is equal to the threshold specified
    in the configuration.
    2) The proof of shuffle for each link in the chain verifies.
    */
    MIX_VALID,
    /*
    Each of the links in the mixing chain was produced by a different trustee.
    */
    MIX_UNIQUE_VALID,
    /*
    The proof of decryption linking the decryption ciphertexts to the decryption factors verifies
    with respect to the public key information.
    */
    DECRYPTION_VALID,
    /*
    The combination of decryption factors matches the published plaintexts.
    */
    PLAINTEXTS_VALID,
}

pub struct Verifier<C: Ctx> {
    trustee: Trustee<C>,
    board: GrpcB3,
    board_name: String,
}
impl<C: Ctx> Verifier<C> {
    pub fn new(trustee: Trustee<C>, board: GrpcB3, board_name: &str) -> Verifier<C> {
        Verifier {
            trustee,
            board,
            board_name: board_name.to_string(),
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        let mut vr = VerificationResult::new(&self.board_name);
        vr.add_target(Check::CONFIGURATION_VALID);
        vr.add_target(Check::MESSAGE_SIGNATURES_VALID);
        vr.add_target(Check::MESSAGES_CFG_VALID);
        vr.add_target(Check::PK_VALID);

        info!(
            "{}",
            format!("Verifying board '{}'", self.board_name).bold()
        );

        let messages = self.board.get_messages(&self.board_name, -1).await?;
        let messages: Vec<(Message, i64)> = messages
            .iter()
            .map(|m| (Message::strand_deserialize(&m.message).unwrap(), m.id))
            .collect();
        // discard ids here
        // let messages: Vec<Message> = messages.into_iter().map(|(m, id)| m).collect();

        let cfg_message: Vec<&Message> = messages
            .iter()
            .filter(|m| m.0.statement.get_kind() == StatementType::Configuration)
            .map(|m| &m.0)
            .collect();

        assert_eq!(cfg_message.len(), 1);

        let cfg_bytes = cfg_message
            .first()
            .as_ref()
            .unwrap()
            .artifact
            .as_ref()
            .unwrap();
        let cfg_h = strand::hash::hash_to_array(&cfg_bytes)?;
        let cfg = Configuration::<C>::strand_deserialize(&cfg_bytes)?;
        info!("Verifying configuration [{}]", dbg_hash(&cfg_h));

        vr.add_result(
            Check::CONFIGURATION_VALID,
            cfg.is_valid(),
            &dbg_hash(&cfg_h),
        );

        // Ensure that all messages refer to the correct configuration

        let correct_cfg = messages
            .iter()
            .filter(|m| m.0.statement.get_cfg_h() == cfg_h)
            .count();
        vr.add_result(
            Check::MESSAGES_CFG_VALID,
            correct_cfg == messages.len(),
            &dbg_hash(&cfg_h),
        );

        // Verify message signatures

        info!("Verifying signatures for {} messages..", messages.len());

        let vmessages: Result<Vec<VerifiedMessage>> =
            messages.iter().map(|m| m.0.verify(&cfg)).collect();
        let vmessages = vmessages?;
        vr.add_result(Check::MESSAGE_SIGNATURES_VALID, true, &vmessages.len());

        // Derive per-batch verification targets

        let mut predicates = vec![];
        // Skip the configuration message
        for message in &vmessages[1..] {
            let predicate =
                Predicate::from_statement::<C>(&message.statement, message.signer_position, &cfg)?;

            if let Predicate::PublicKey(_, hash, _, _, _) = predicate.clone() {
                if let Some(bytes) = message.artifact.clone() {
                    let dkg_public_key = DkgPublicKey::<C>::strand_deserialize(&bytes).unwrap();
                    let pk_bytes = dkg_public_key.pk.strand_serialize().unwrap();
                    let pk_b64 = general_purpose::STANDARD_NO_PAD.encode(pk_bytes);
                    info!("Public Key found: {} {:?}", pk_b64, hash);
                }
            }
            predicates.push(predicate);
        }
        predicates.push(Predicate::get_verifier_bootstrap_predicate(&cfg).unwrap());

        info!("{}", "Deriving verification targets..".blue());
        let (_, targets, _) = crate::verify::datalog::S.run(&predicates);
        for t in &targets {
            let tvr = t.get_verification_result();
            info!("Add verification target [batch {}]", t.get_batch());
            vr.add_child(tvr);
        }

        // Run verifying actions

        info!("{}", "Running verifying actions..".blue());
        // Trustee running in verifier mode
        let output_messages = self.trustee.verify(messages)?;
        info!("{}", "Verifying actions complete".blue());
        for message in output_messages {
            let predicate =
                Predicate::from_statement::<C>(&message.statement, VERIFIER_INDEX, &cfg)?;
            info!("Verifying action yields predicate [{}]", predicate);
            predicates.push(predicate);
        }

        // Collect verification results

        info!("{}", "Collecting verification results".blue());
        let (root, _targets, verified) = crate::verify::datalog::S.run(&predicates);

        let mut pk_h = None;
        let root = root.iter().next();
        if let Some(root) = root {
            pk_h = Some(root.1);
            vr.add_result(Check::PK_VALID, true, &dbg_hash(&root.1 .0));
        }

        for v in verified {
            v.add_results(
                &mut vr,
                targets.iter().find(|t| t.1 == v.1).unwrap(),
                &cfg,
                &pk_h,
            )?;
        }

        // Summary

        info!("{}", vr);

        Ok(())
    }
}

impl Target {
    fn get_verification_result(&self) -> VerificationResult {
        let mut vr = VerificationResult::new(&self.get_batch().to_string());
        vr.add_target(Check::BALLOTS_PK_VALID);
        vr.add_target(Check::MIX_START_VALID);
        vr.add_target(Check::MIX_END_VALID);
        vr.add_target(Check::MIX_VALID);
        vr.add_target(Check::MIX_UNIQUE_VALID);
        vr.add_target(Check::DECRYPTION_VALID);
        vr.add_target(Check::PLAINTEXTS_VALID);

        vr
    }
    fn _get_cfg_h(&self) -> ConfigurationHash {
        self.0
    }
    fn get_batch(&self) -> BatchNumber {
        self.1
    }
    fn get_pk_h(&self) -> PublicKeyHash {
        self.2
    }
    fn get_ballots_h(&self) -> CiphertextsHash {
        self.3
    }
    fn get_plaintexts_h(&self) -> PlaintextsHash {
        self.4
    }
}

impl Verified {
    fn add_results<C: Ctx>(
        &self,
        vr: &mut VerificationResult,
        target: &Target,
        cfg: &Configuration<C>,
        pk_h: &Option<PublicKeyHash>,
    ) -> Result<()> {
        let mixing_hs = self.get_mixing_hs();
        let filtered_mixes: Vec<[u8; 64]> = mixing_hs
            .0
            .into_iter()
            .filter(|h| *h != [0u8; 64])
            .collect();

        let b = &self.get_batch().to_string();
        let child = vr
            .children
            .get_mut(b)
            .expect(&format!("no target for batch '{}'", b));

        assert_eq!(self.get_batch(), target.get_batch());

        child.add_result(
            Check::BALLOTS_PK_VALID,
            *pk_h == Some(target.get_pk_h()),
            &dbg_hash(&target.get_pk_h().0),
        );
        child.add_result(
            Check::MIX_START_VALID,
            filtered_mixes[0] == target.get_ballots_h().0
                && filtered_mixes[0] == self.get_ballots_h().0,
            &dbg_hash(&target.get_ballots_h().0),
        );
        child.add_result(
            Check::MIX_END_VALID,
            filtered_mixes[cfg.threshold] == self.get_decryption_input_h().0,
            &cfg.threshold,
        );
        // subtract one since the number of hashes includes the source and the target, eg ballots => mix1 => mix2 has length 3, but threshold = 2
        child.add_result(
            Check::MIX_VALID,
            filtered_mixes.len() - 1 == cfg.threshold,
            &cfg.threshold,
        );
        // This is already certified by the datalog predicates
        child.add_result(Check::MIX_UNIQUE_VALID, true, &filtered_mixes.len());
        child.add_result(
            Check::DECRYPTION_VALID,
            self.get_decryption_pk_h() == target.get_pk_h(),
            &dbg_hash(&target.get_pk_h().0),
        );
        child.add_result(
            Check::PLAINTEXTS_VALID,
            self.get_plaintexts_h() == target.get_plaintexts_h(),
            &dbg_hash(&self.get_plaintexts_h().0),
        );

        Ok(())
    }

    fn _get_cfg_h(&self) -> ConfigurationHash {
        self.0
    }
    fn get_batch(&self) -> BatchNumber {
        self.1
    }
    fn get_ballots_h(&self) -> CiphertextsHash {
        self.2
    }
    fn get_decryption_input_h(&self) -> CiphertextsHash {
        self.3
    }
    fn get_decryption_pk_h(&self) -> PublicKeyHash {
        self.4
    }
    fn get_plaintexts_h(&self) -> PlaintextsHash {
        self.5
    }
    fn get_mixing_hs(&self) -> MixingHashes {
        self.6
    }
}

use std::collections::HashMap;
#[derive(Serialize)]
struct VerificationResult {
    name: String,
    targets: HashMap<Check, VerificationItem>,
    children: HashMap<String, VerificationResult>,
}
impl VerificationResult {
    fn new(name: &str) -> VerificationResult {
        VerificationResult {
            name: name.to_string(),
            targets: HashMap::new(),
            children: HashMap::new(),
        }
    }
    fn add_target(&mut self, name: Check) {
        self.targets.insert(name, VerificationItem::new());
    }
    fn add_result<D: std::fmt::Display>(&mut self, name: Check, result: bool, metadata: &D) {
        let value = self
            .targets
            .get_mut(&name)
            .expect(&format!("no target for '{}'", &name));
        value.result = result;
        value.metadata = metadata.to_string();
    }
    fn add_child(&mut self, child: VerificationResult) {
        self.children.insert(child.name.clone(), child);
    }

    fn totals(&self) -> (u64, u64, usize) {
        let mut ok = 0;
        let mut not_ok = 0;
        for (_name, value) in &self.targets {
            if value.result {
                ok = ok + 1;
            } else {
                not_ok = not_ok + 1;
            }
        }
        for (_name, child) in &self.children {
            let (ok_, not_ok_, _) = child.totals();
            ok += ok_;
            not_ok += not_ok_;
        }

        (ok, not_ok, self.children.len())
    }
}

#[derive(Serialize)]
struct VerificationItem {
    pub result: bool,
    pub metadata: String,
}
impl VerificationItem {
    fn new() -> VerificationItem {
        VerificationItem {
            result: false,
            metadata: String::from(""),
        }
    }
}

impl std::fmt::Display for VerificationResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let json = serde_json::to_string_pretty(&self);
        writeln!(f, "VerificationResult:")?;
        let json = json.unwrap();
        let json_color = json.replace("true", &"true".green().to_string());
        let json_color = json_color.replace("false", &"false".red().to_string());
        writeln!(f, "{}", json_color)?;
        let (ok, not_ok, batches) = self.totals();
        let checks = format!("{} / {}", ok, (ok + not_ok));
        if not_ok == 0 {
            writeln!(f, "[{}] checks pass ({} batches)", checks.green(), batches)?;
        } else {
            writeln!(f, "[{}] checks pass ({} batches)", checks.red(), batches)?;
        }

        Ok(())
    }
}
