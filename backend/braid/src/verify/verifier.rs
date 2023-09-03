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

pub struct Verifier<C: Ctx> {
    trustee: Trustee<C>,
    board: ImmudbBoard,
}
impl<C: Ctx> Verifier<C> {
    pub fn new(trustee: Trustee<C>, board: ImmudbBoard) -> Verifier<C> {
        Verifier { trustee, board }
    }

    pub async fn run(&mut self) -> Result<()> {
        // A verification result is function of 4 inputs
        // (public key, ballots, plaintexts, trustees)
        let mut vr = VerificationResult::new(&self.board.board_dbname);
        vr.add_target("Configuration valid");
        vr.add_target("Message signatures verified");
        vr.add_target("All messages have correct configuration");

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

        vr.add_result("Configuration valid", cfg.is_valid(), &dbg_hash(&cfg_h));

        // Verify message signatures

        info!("Verifying signatures for {} messages..", messages.len());
        let vmessages: Result<Vec<VerifiedMessage>> = messages
            .clone()
            .into_iter()
            .map(|m| m.verify(&cfg))
            .collect();
        let vmessages = vmessages?;
        vr.add_result("Message signatures verified", true, &vmessages.len());

        let correct_cfg = messages
            .clone()
            .into_iter()
            .filter(|m| m.statement.get_cfg_h() == cfg_h)
            .count();
        vr.add_result(
            "All messages have correct configuration",
            correct_cfg == messages.len(),
            &dbg_hash(&cfg_h),
        );

        // Obtain verification targets

        let mut predicates = vec![];
        // Skip the configuration message
        for message in &vmessages[1..] {
            let predicate =
                Predicate::from_statement::<C>(&message.statement, message.signer_position, &cfg);
            predicates.push(predicate);
        }
        predicates.push(Predicate::get_verifier_bootstrap_predicate(&cfg).unwrap());

        info!("{}", "Deriving verification targets..".blue());
        let (targets, _) = crate::verify::datalog::S.run(&predicates);
        for t in &targets {
            let mut tvr = t.get_verification_target();
            tvr.add_result(
                "Configuration matches parent",
                t.0 .0 == cfg_h,
                &dbg_hash(&cfg_h),
            );

            info!("Add verification target [{}]", t.get_batch());
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
        let (_targets, verified) = crate::verify::datalog::S.run(&predicates);
        for v in verified {
            v.add_results(&mut vr, targets.iter().find(|t| t.1 == v.1).unwrap(), &cfg);
        }

        // Summary

        info!("{}", vr);

        Ok(())
    }
}

use crate::protocol2::predicate::*;

impl Target {
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
    ) {
        let mixing_hs = self.get_mixing_hs();
        let filtered_mixes: Vec<[u8; 64]> = mixing_hs
            .0
            .into_iter()
            .filter(|h| *h != [0u8; 64])
            .collect();

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
        self.0
    }
    fn get_batch(&self) -> BatchNumber {
        self.1
    }
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
        let value = self.targets.get_mut(name).unwrap();
        value.result = result;
        value.metadata = metadata.to_string();
    }
    fn add_child(&mut self, child: VerificationResult) {
        self.children.insert(child.name.clone(), child);
    }

    fn totals(&self) -> (u64, u64) {
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
            let (ok_, not_ok_) = child.totals();
            ok += ok_;
            not_ok += not_ok_;
        }

        (ok, not_ok)
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
        let (ok, not_ok) = self.totals();
        let checks = format!("{} / {}", ok, (ok + not_ok));
        if not_ok == 0 {
            writeln!(f, "[{}] checks pass", checks.green())?;
        } else {
            writeln!(f, "[{}] checks pass", checks.red())?;
        }

        Ok(())
    }
}
