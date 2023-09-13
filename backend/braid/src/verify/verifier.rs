#![allow(non_camel_case_types)]
use anyhow::Result;
use colored::*;
use serde::Serialize;
use tracing::info;

use crate::protocol2::action::Message;
use crate::protocol2::artifact::Configuration;
use crate::protocol2::board::immudb::ImmudbBoard;
use crate::protocol2::message::VerifiedMessage;
use crate::protocol2::predicate::Predicate;
use crate::protocol2::statement::StatementType;
use crate::protocol2::trustee::Trustee;

use crate::util::dbg_hash;
use crate::verify::datalog::Target;
use crate::verify::datalog::Verified;

use strand::context::Ctx;
use strand::serialization::StrandDeserialize;

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

enum Checks {
    CONFIGURATION_VALID,
    MESSAGE_SIGNATURES_VALID,
    MESSAGES_CFG_VALID,
    PK_VALID,
    BALLOTS_PK_VALID,
    MIX_START_VALID,
    MIX_END_VALID,
    MIX_VALID,
    MIX_UNIQUE_VALID,
    DECRYPTION_VALID,
    PLAINTEXTS_VALID,
}
/*
The configuration is valid as per Configuration::is_valid:
    1) The number of trustees ranges from 2 to 12 (crate::protocol2::MAX_TRUSTEES).
    2) The threshold ranges from 2 to the number of trustees.
    3) There are no duplicate trustees.
*/
const CONFIGURATION_VALID: &str = "Configuration valid";

/*
All message signatures verify, with respect to their specified sender public key.
*/
const MESSAGE_SIGNATURES_VALID: &str = "All message signatures verified";

/*
All messages refer to the same configuration file that is checked
with CONFIGURATION_VALID.
*/
const MESSAGES_CFG_VALID: &str = "All messages refer to correct configuration";

/*
The public key information has been correctly constructed (given public data):
    1) The public key has been correctly constructed.
    2) The trustee verification keys have been correctly constructed.
    3) All trustees have signed the public key statement, which asserts correctness
    of private shares (VSS).
*/
const PK_VALID: &str = "Public key verified";

/*
The protocol manager's signature on the ballots and public key verifies.
*/
const BALLOTS_PK_VALID: &str = "Ballot public key matches root public key";

/*
The first link in the mixing chain takes the ballots as input.
*/
const MIX_START_VALID: &str = "Mixing chain start matches ballots";

/*
The last link in the mixing chain outputs the ciphertexts inputted to decryption.
*/
const MIX_END_VALID: &str = "Mixing chain end matches decrypting ballots";

/*
1) The number of links in the mixing chain is equal to the threshold specified
in the configuration.
2) The proof of shuffle for each link in the chain verifies.
*/
const MIX_VALID: &str = "Mixing chain correct length";

/*
Each of the links in the mixing chain was produced by a different trustee.
*/
const MIX_UNIQUE_VALID: &str = "Mixing chain no duplicate signers";

/*
The proof of decryption linking the decryption ciphertexts to the decryption factors verifies
with respect to the public key information.
*/
const DECRYPTION_VALID: &str = "Decryption validated with respect to ballot public key";

/*
The combination of decryption factors matches the published plaintexts.
 */
const PLAINTEXTS_VALID: &str = "Plaintexts match";

pub struct Verifier<C: Ctx> {
    trustee: Trustee<C>,
    board: ImmudbBoard,
}
impl<C: Ctx> Verifier<C> {
    pub fn new(trustee: Trustee<C>, board: ImmudbBoard) -> Verifier<C> {
        Verifier { trustee, board }
    }

    pub async fn run(&mut self) -> Result<()> {
        let mut vr = VerificationResult::new(&self.board.board_dbname);
<<<<<<< HEAD
        vr.add_target(CONFIGURATION_VALID);
        vr.add_target(MESSAGE_SIGNATURES_VALID);
        vr.add_target(MESSAGES_CFG_VALID);
        vr.add_target(PK_VALID);
=======
        vr.add_target("Configuration valid");
        vr.add_target("Message signatures verified");
        vr.add_target("All messages have correct configuration");
>>>>>>> main

        info!(
            "{}",
            format!("Verifying board '{}'", self.board.board_dbname).bold()
        );

        let messages = self.board.get_messages(-1).await?;

        let cfg_message: Vec<Message> = messages
            .clone()
            .into_iter()
            .filter(|m| m.statement.get_kind() == StatementType::Configuration)
            .collect();

        assert_eq!(cfg_message.len(), 1);

        let cfg_bytes = cfg_message
            .first()
            .as_ref()
            .unwrap()
            .artifact
            .as_ref()
            .unwrap();
        let cfg_h = crate::util::hash_from_vec(&strand::util::hash(&cfg_bytes)).unwrap();
        let cfg = Configuration::<C>::strand_deserialize(&cfg_bytes)?;
        info!("Verifying configuration [{}]", dbg_hash(&cfg_h));

<<<<<<< HEAD
        vr.add_result(CONFIGURATION_VALID, cfg.is_valid(), &dbg_hash(&cfg_h));
=======
        vr.add_result("Configuration valid", cfg.is_valid(), &dbg_hash(&cfg_h));
>>>>>>> main

        // Verify message signatures

        info!("Verifying signatures for {} messages..", messages.len());
        let vmessages: Result<Vec<VerifiedMessage>> = messages
            .clone()
            .into_iter()
            .map(|m| m.verify(&cfg))
            .collect();
        let vmessages = vmessages?;
<<<<<<< HEAD
        vr.add_result(MESSAGE_SIGNATURES_VALID, true, &vmessages.len());
=======
        vr.add_result("Message signatures verified", true, &vmessages.len());
>>>>>>> main

        let correct_cfg = messages
            .clone()
            .into_iter()
            .filter(|m| m.statement.get_cfg_h() == cfg_h)
            .count();
        vr.add_result(
<<<<<<< HEAD
            MESSAGES_CFG_VALID,
=======
            "All messages have correct configuration",
>>>>>>> main
            correct_cfg == messages.len(),
            &dbg_hash(&cfg_h),
        );

        // Derive per-batch verification targets

        let mut predicates = vec![];
        // Skip the configuration message
        for message in &vmessages[1..] {
            let predicate =
                Predicate::from_statement::<C>(&message.statement, message.signer_position, &cfg);
            predicates.push(predicate);
        }
        predicates.push(Predicate::get_verifier_bootstrap_predicate(&cfg).unwrap());

        info!("{}", "Deriving verification targets..".blue());
<<<<<<< HEAD
        let (_, targets, _) = crate::verify::datalog::S.run(&predicates);
        for t in &targets {
            let tvr = t.get_verification_result();
            info!("Add verification target [batch {}]", t.get_batch());
=======
        let (targets, _) = crate::verify::datalog::S.run(&predicates);
        for t in &targets {
            let mut tvr = t.get_verification_target();
            tvr.add_result(
                "Configuration matches parent",
                t.0 .0 == cfg_h,
                &dbg_hash(&cfg_h),
            );

            info!("Add verification target [{}]", t.get_batch());
>>>>>>> main
            vr.add_child(tvr);
        }

        // Run verifying actions

        info!("{}", "Running verifying actions..".blue());
        let (messages, _) = self.trustee.verify(messages)?;
        info!("{}", "Verifying actions complete".blue());
        for message in messages.clone() {
            let predicate = Predicate::from_statement::<C>(
                &message.statement,
                crate::protocol2::VERIFIER_INDEX,
                &cfg,
            );
            info!("Verifying action yields predicate [{}]", predicate);
            predicates.push(predicate);
        }

        // Collect verification results

        info!("{}", "Collecting verification results".blue());
<<<<<<< HEAD
        let (root, _targets, verified) = crate::verify::datalog::S.run(&predicates);

        let mut pk_h = None;
        let root = root.iter().next();
        if let Some(root) = root {
            pk_h = Some(root.1);
            vr.add_result("Public key verified", true, &dbg_hash(&root.1 .0));
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

=======
        let (_targets, verified) = crate::verify::datalog::S.run(&predicates);
        for v in verified {
            v.add_results(&mut vr, targets.iter().find(|t| t.1 == v.1).unwrap(), &cfg);
        }

        // Summary

>>>>>>> main
        info!("{}", vr);

        Ok(())
    }
}

use crate::protocol2::predicate::*;

impl Target {
<<<<<<< HEAD
    fn get_verification_result(&self) -> VerificationResult {
        let mut vr = VerificationResult::new(&self.get_batch().to_string());
        vr.add_target(BALLOTS_PK_VALID);
        vr.add_target(MIX_START_VALID);
        vr.add_target(MIX_END_VALID);
        vr.add_target(MIX_VALID);
        vr.add_target(MIX_UNIQUE_VALID);
        vr.add_target(DECRYPTION_VALID);
        vr.add_target(PLAINTEXTS_VALID);

        vr
    }
    fn _get_cfg_h(&self) -> ConfigurationHash {
=======
    fn get_verification_target(&self) -> VerificationResult {
        let mut vr = VerificationResult::new(&self.1.to_string());
        vr.add_target("Configuration matches parent");
        vr.add_target("Configuration matches");
        vr.add_target("Batch matches");
        vr.add_target("Public key is verified");
        vr.add_target("Ballots' public key matches");
        vr.add_target("Decryption validated with respect to public key");
        vr.add_target("Plaintexts match");
        vr.add_target("Mixing chain start matches ballots");
        vr.add_target("Mixing chain end matches decrypting ballots");
        vr.add_target("Mixing chain correct length");
        vr.add_target("Mixing chain no duplicate signers");

        vr
    }
    fn get_cfg_h(&self) -> ConfigurationHash {
>>>>>>> main
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
<<<<<<< HEAD
        pk_h: &Option<PublicKeyHash>,
    ) -> Result<()> {
=======
    ) {
>>>>>>> main
        let mixing_hs = self.get_mixing_hs();
        let filtered_mixes: Vec<[u8; 64]> = mixing_hs
            .0
            .into_iter()
            .filter(|h| *h != [0u8; 64])
            .collect();

<<<<<<< HEAD
        let b = &self.get_batch().to_string();
        let child = vr
            .children
            .get_mut(b)
            .expect(&format!("no target for batch '{}'", b));

        assert_eq!(self.get_batch(), target.get_batch());

        child.add_result(
            BALLOTS_PK_VALID,
            *pk_h == Some(target.get_pk_h()),
            &dbg_hash(&target.get_pk_h().0),
        );
        child.add_result(
            MIX_START_VALID,
            filtered_mixes[0] == target.get_ballots_h().0
                && filtered_mixes[0] == self.get_ballots_h().0,
            &dbg_hash(&target.get_ballots_h().0),
        );
        child.add_result(
            MIX_END_VALID,
            filtered_mixes[cfg.threshold] == self.get_decryption_input_h().0,
            &cfg.threshold,
        );
        // subtract one since the number of hashes includes the source and the target, eg ballots => mix1 => mix2 has length 3, but threshold = 2
        child.add_result(
            MIX_VALID,
            filtered_mixes.len() - 1 == cfg.threshold,
            &cfg.threshold,
        );
        // This is already certified by the datalog predicates
        child.add_result(MIX_UNIQUE_VALID, true, &filtered_mixes.len());
        child.add_result(
            DECRYPTION_VALID,
            self.get_decryption_pk_h() == target.get_pk_h(),
            &dbg_hash(&target.get_pk_h().0),
        );
        child.add_result(
            PLAINTEXTS_VALID,
            self.get_plaintexts_h() == target.get_plaintexts_h(),
            &dbg_hash(&self.get_plaintexts_h().0),
        );

        Ok(())
    }

    fn _get_cfg_h(&self) -> ConfigurationHash {
=======
        let child = vr.children.get_mut(&self.1.to_string()).unwrap();
        child.add_result(
            "Configuration matches",
            self.get_cfg_h() == target.get_cfg_h(),
            &dbg_hash(&self.0 .0),
        );
        child.add_result(
            "Batch matches",
            self.get_batch() == target.get_batch(),
            &dbg_hash(&self.0 .0),
        );
        // This is already certified by the datalog predicates
        child.add_result(
            "Mixing chain no duplicate signers",
            true,
            &filtered_mixes.len(),
        );
        // subtract one since the number of hashes includes the source and the target, eg ballots => mix1 => mix2 has length 3, but threshold = 2
        child.add_result(
            "Mixing chain correct length",
            filtered_mixes.len() - 1 == cfg.threshold,
            &cfg.threshold,
        );
        child.add_result(
            "Mixing chain start matches ballots",
            filtered_mixes[0] == target.get_ballots_h().0
                && filtered_mixes[0] == self.get_ballots_h().0,
            &dbg_hash(&target.3 .0),
        );
        child.add_result(
            "Mixing chain end matches decrypting ballots",
            filtered_mixes[cfg.threshold] == self.get_decryption_input_h().0,
            &cfg.threshold,
        );
        child.add_result(
            "Public key is verified",
            self.get_verified_pk_h() == target.get_pk_h(),
            &dbg_hash(&self.2 .0),
        );
        child.add_result(
            "Ballots' public key matches",
            self.get_ballots_pk_h() == target.get_pk_h(),
            &dbg_hash(&self.3 .0),
        );
        child.add_result(
            "Decryption validated with respect to public key",
            self.get_decryption_pk_h() == target.get_pk_h(),
            &dbg_hash(&self.5 .0),
        );
        child.add_result(
            "Plaintexts match",
            self.get_plaintexts_h() == target.get_plaintexts_h(),
            &dbg_hash(&self.7 .0),
        );
    }

    fn get_cfg_h(&self) -> ConfigurationHash {
>>>>>>> main
        self.0
    }
    fn get_batch(&self) -> BatchNumber {
        self.1
    }
<<<<<<< HEAD
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
=======
    fn get_verified_pk_h(&self) -> PublicKeyHash {
        self.2
    }
    fn get_ballots_pk_h(&self) -> PublicKeyHash {
        self.3
    }
    fn get_ballots_h(&self) -> CiphertextsHash {
        self.4
    }
    fn get_decryption_input_h(&self) -> CiphertextsHash {
        self.5
    }
    fn get_decryption_pk_h(&self) -> PublicKeyHash {
        self.6
    }
    fn get_plaintexts_h(&self) -> PlaintextsHash {
        self.7
    }
    fn get_mixing_hs(&self) -> MixingHashes {
        self.8
    }
>>>>>>> main
}

use std::collections::HashMap;
#[derive(Serialize)]
struct VerificationResult {
    name: String,
    targets: HashMap<String, VerificationItem>,
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
    fn add_target(&mut self, name: &str) {
        self.targets
            .insert(name.to_string(), VerificationItem::new());
    }
    fn add_result<D: std::fmt::Display>(&mut self, name: &str, result: bool, metadata: &D) {
<<<<<<< HEAD
        let value = self
            .targets
            .get_mut(name)
            .expect(&format!("no target for '{}'", &name));
=======
        let value = self.targets.get_mut(name).unwrap();
>>>>>>> main
        value.result = result;
        value.metadata = metadata.to_string();
    }
    fn add_child(&mut self, child: VerificationResult) {
        self.children.insert(child.name.clone(), child);
    }

<<<<<<< HEAD
    fn totals(&self) -> (u64, u64, usize) {
=======
    fn totals(&self) -> (u64, u64) {
>>>>>>> main
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
<<<<<<< HEAD
            let (ok_, not_ok_, _) = child.totals();
=======
            let (ok_, not_ok_) = child.totals();
>>>>>>> main
            ok += ok_;
            not_ok += not_ok_;
        }

<<<<<<< HEAD
        (ok, not_ok, self.children.len())
=======
        (ok, not_ok)
>>>>>>> main
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
<<<<<<< HEAD
        let (ok, not_ok, batches) = self.totals();
        let checks = format!("{} / {}", ok, (ok + not_ok));
        if not_ok == 0 {
            writeln!(f, "[{}] checks pass ({} batches)", checks.green(), batches)?;
        } else {
            writeln!(f, "[{}] checks pass ({} batches)", checks.red(), batches)?;
=======
        let (ok, not_ok) = self.totals();
        let checks = format!("{} / {}", ok, (ok + not_ok));
        if not_ok == 0 {
            writeln!(f, "[{}] checks pass", checks.green())?;
        } else {
            writeln!(f, "[{}] checks pass", checks.red())?;
>>>>>>> main
        }

        Ok(())
    }
}
