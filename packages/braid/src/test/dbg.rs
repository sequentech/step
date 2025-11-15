// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use ascii_table::{Align, AsciiTable};

use log::{info, warn};
use reedline_repl_rs::clap::{Arg, ArgMatches, Command};
use reedline_repl_rs::{Repl, Result};
use std::collections::HashSet;
use std::marker::PhantomData;

use tracing_attributes::instrument;
use tracing_subscriber::filter;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::registry::Registry;
use tracing_subscriber::reload::Handle;

use strand::context::Ctx;
use strand::elgamal::Ciphertext;
use strand::serialization::{StrandDeserialize, StrandSerialize};
use strand::signature::{StrandSignaturePk, StrandSignatureSk};

use crate::protocol::action::Action;
use crate::protocol::board::local::{ArtifactEntryIdentifier, StatementEntryIdentifier};
use b3::messages::artifact::Ballots;
use b3::messages::artifact::Configuration;
use b3::messages::message::Message;
use b3::messages::newtypes::PublicKeyHash;
use b3::messages::newtypes::NULL_TRUSTEE;
use b3::messages::protocol_manager::ProtocolManager;

use crate::protocol::trustee::Trustee;
use crate::test::vector_board::VectorBoard;
use b3::messages::newtypes::MAX_TRUSTEES;

/// Runs a simple interactive ncurses terminal to simulate or
/// debug a protocol execution.
///
/// The following steps execute a protocol run:
///
/// reset <trustees> <threshold>    - initialize the protocol
///
/// step                            - executes one step of the protocol
/// This command is repeated until the protocol has completed key generation
///
/// ballots                          - generates plaintexts, encrypts and posts
///
/// step                            - executes one step of the protocol
/// This command is repeated until the protocol has completed shuffling
/// and decryption.
///
/// decrypted                       - shows decrypted plaintexts and if they match
///
/// The protocol simulates the trustee's data, the bulletin board and their
/// communication in memory. The status command can be used to inspect the messages
/// in the bulletin board and each trustee's view of it. Use the help command
/// to display information on each command.
///
/// When launching, the initial number of trustees is 2, with threshold
/// participants = (1,2).
#[instrument(skip(log_reload))]
pub fn dbg<C: Ctx>(ctx: C, log_reload: Handle<LevelFilter, Registry>) -> Result<()> {
    let trustees = 2;
    let threshold = [1, 2];

    let mut demo = mk_context(ctx, trustees, &threshold);
    demo.log_reload = Some(log_reload);

    let mut repl = Repl::new(demo)
        .with_name(&format!("Braid (t={})", trustees))
        .with_version("v0.1")
        .with_description("")
        .with_banner("")
        .with_stop_on_ctrl_c(true)
        .with_prompt("")
        .with_command(
            Command::new("decrypted").about("Shows the last decrypted plaintexts and whether they match the encrypted plaintexts"),
            decrypted,
        )
        .with_command(
            Command::new("step")
                .arg(Arg::new("trustee").required(false).help("The trustee to run the step for, or all if none is supplied."))
                .about("Execute one step of the protocol, optionally specifying one trustee to run."),
            step,
        )
        .with_command(
            Command::new("status").about("Shows the bulletin board board and each trustee's view of it"),
            status,
        )
        .with_command(
            Command::new("plaintexts").about("Shows the last encrypted plaintexts"),
            plaintexts,
        )
        .with_command(
            Command::new("reset")
                .arg(Arg::new("trustees").required(true).help("The total number of trustees"))
                .arg(Arg::new("threshold").required(true).help("Comma separated list of trustees that will participate (eg '1,2')"))
                .about("Reset the run, passing in the number of trustees and a comma separated list for the threshold participants"),
            reset,
        )
        .with_command(
            Command::new("log")
                .arg(Arg::new("level").required(false).help("The new log level; 0 = OFF, 5 = TRACE."))
                .about("Set log level (0-5). If no level is specified prints the current level"),
            log,
        )
        .with_command(
            Command::new("ballots")
                .arg(Arg::new("count").required(false).help("The number of ballots to post, default = 10."))
                .arg(Arg::new("batch").required(false).help("The batch number to use, 0 or greater, default = 1."))
                .about("Post a ballot batch, optionally passing the number of ballots to post and the batch number"),
            ballots,
        )
        .with_command(Command::new("quit").about("quit"), quit);

    repl.run()
}

/// Contains all the information necessary to interact with the protocol from the repl.
struct ReplContext<C: Ctx> {
    pub ctx: C,
    pub cfg: Configuration<C>,
    pub protocol_manager: ProtocolManager<C>,
    pub trustees: Vec<Trustee<C>>,
    pub trustee_pks: Vec<StrandSignaturePk>,
    pub remote: VectorBoard,
    pub last_messages: Vec<Message>,
    pub last_actions: HashSet<Action>,
    pub log_reload: Option<Handle<LevelFilter, Registry>>,
    pub plaintexts: Vec<C::P>,
    pub selected_trustees: [usize; 12],
}

/// The information that is displayed with the status command.
///
/// This includes
///
/// * The bulletin board messages.
/// * Each trustee's view of the bulletin board.
/// * The Messages posted in the last step.
/// * The Actions executed in the last step.
struct Status<C: Ctx> {
    cfg: Configuration<C>,
    // locals: Vec<LocalBoard<C>>,
    statement_keys: Vec<Vec<StatementEntryIdentifier>>,
    artifact_keys: Vec<Vec<ArtifactEntryIdentifier>>,
    remote: VectorBoard,
    last_messages: Vec<Message>,
    last_actions: HashSet<Action>,
}
impl<C: Ctx> Status<C> {
    fn new(
        cfg: Configuration<C>,
        statement_keys: Vec<Vec<StatementEntryIdentifier>>,
        artifact_keys: Vec<Vec<ArtifactEntryIdentifier>>,
        remote: VectorBoard,
        last_messages: Vec<Message>,
        last_actions: HashSet<Action>,
    ) -> Status<C> {
        Status {
            cfg,
            statement_keys,
            artifact_keys,
            remote,
            last_messages,
            last_actions,
        }
    }
    /// Shows status information using ascii tables.
    fn to_string(&self) -> String {
        let mut boards = vec![];

        boards.push("Trustees".to_string());
        for (i, _) in self.statement_keys.iter().enumerate() {
            let mut ascii_table = AsciiTable::default();
            ascii_table.set_max_width(205);
            ascii_table
                .column(0)
                .set_header(format!("trustee {}", i.to_string()))
                .set_align(Align::Left);

            let data1: Vec<String> = self.statement_keys[i]
                .iter()
                .map(|k| format!("{}-{}", k.kind.to_string(), k.signer_position))
                .collect();
            let data2: Vec<String> = self.artifact_keys[i]
                .iter()
                .map(|k| {
                    format!(
                        "{}-{}",
                        k.statement_entry.kind, k.statement_entry.signer_position
                    )
                })
                .collect();

            let data: Vec<Vec<String>> = vec![
                vec!["stm:".to_string(), data1.join(" ")],
                vec!["art:".to_string(), data2.join(" ")],
            ];

            boards.push(ascii_table.format(data));
        }

        let mut ascii_table = AsciiTable::default();
        ascii_table.set_max_width(205);
        ascii_table
            .column(0)
            .set_header("type")
            .set_align(Align::Left);
        ascii_table
            .column(1)
            .set_header("sender")
            .set_align(Align::Left);
        ascii_table
            .column(2)
            .set_header("artifact")
            .set_align(Align::Left);
        let mut data: Vec<Vec<String>> = vec![];
        for m in self.last_messages.iter() {
            let sender = self.cfg.get_trustee_position(&m.sender.pk).unwrap();
            data.push(vec![
                format!("{:?}", m.statement.get_kind()),
                format!("{}", sender),
                m.artifact.is_some().to_string(),
            ])
        }
        boards.push("Last step messages".to_string());
        if data.len() > 0 {
            boards.push(ascii_table.format(data));
        } else {
            boards.push("-".to_string());
            boards.push("".to_string());
        }

        let mut ascii_table = AsciiTable::default();
        ascii_table.set_max_width(205);
        ascii_table
            .column(0)
            .set_header("type")
            .set_align(Align::Left);
        ascii_table
            .column(1)
            .set_header("sender")
            .set_align(Align::Left);
        ascii_table
            .column(2)
            .set_header("artifact")
            .set_align(Align::Left);
        ascii_table
            .column(3)
            .set_header("batch")
            .set_align(Align::Left);
        let mut data: Vec<Vec<String>> = vec![];
        for m in self.remote.messages.iter() {
            let m = Message::strand_deserialize(&m.message).unwrap();
            let sender = self.cfg.get_trustee_position(&m.sender.pk).unwrap();
            data.push(vec![
                format!("{:?}", m.statement.get_kind()),
                format!("{}", sender),
                m.artifact.is_some().to_string(),
                format!("{}", m.statement.get_data().3),
            ])
        }
        boards.push("Remote".to_string());
        boards.push(ascii_table.format(data));

        boards.push("Last actions".to_string());
        if self.last_actions.len() > 0 {
            boards.push(format!("{:?}", self.last_actions));
        } else {
            boards.push("-".to_string());
        }

        boards.join("\r\n")
    }
}

/// Constructs the repl context used to interact with the protocol.
fn mk_context<C: Ctx>(ctx: C, n_trustees: u8, threshold: &[usize]) -> ReplContext<C> {
    let mut selected = [NULL_TRUSTEE; MAX_TRUSTEES];
    selected[0..threshold.len()].copy_from_slice(&threshold);

    let pmkey: StrandSignatureSk = StrandSignatureSk::gen().unwrap();
    let pm: ProtocolManager<C> = ProtocolManager {
        signing_key: pmkey,
        phantom: PhantomData,
    };

    let trustees: Vec<Trustee<C>> = (0..n_trustees)
        .into_iter()
        .map(|i| {
            let kp = StrandSignatureSk::gen().unwrap();
            // let encryption_key = ChaCha20Poly1305::generate_key(&mut csprng);
            let encryption_key = strand::symm::gen_key();
            Trustee::new(
                i.to_string(),
                "foo".to_string(),
                kp,
                encryption_key,
                None,
                None,
            )
        })
        .collect();

    let trustee_pks: Vec<StrandSignaturePk> = trustees
        .iter()
        .map(|t| StrandSignaturePk::from_sk(&t.signing_key).unwrap())
        .collect();

    let cfg = Configuration::<C>::new(
        0,
        StrandSignaturePk::from_sk(&pm.signing_key).unwrap(),
        trustee_pks.clone(),
        threshold.len(),
        PhantomData,
    );

    let mut remote = VectorBoard::new(0);
    let message = Message::bootstrap_msg(&cfg, &pm).unwrap();
    remote.add(message);

    info!(
        "Num trustees = {:?}, threshold = {:?}",
        n_trustees, threshold
    );

    ReplContext {
        ctx,
        cfg,
        protocol_manager: pm,
        trustees,
        trustee_pks,
        remote,
        last_messages: vec![],
        last_actions: HashSet::from([]),
        log_reload: None,
        plaintexts: vec![],
        selected_trustees: selected,
    }
}

/// Sets or displays the current log level.
fn log<C: Ctx>(args: ArgMatches, context: &mut ReplContext<C>) -> Result<Option<String>> {
    let new_level;
    let l = args.get_one::<String>("level");
    if let Some(level) = l {
        let parsed = level.parse::<u8>()?;
        new_level = match parsed {
            0 => filter::LevelFilter::OFF,
            1 => filter::LevelFilter::ERROR,
            2 => filter::LevelFilter::WARN,
            3 => filter::LevelFilter::INFO,
            4 => filter::LevelFilter::DEBUG,
            5 => filter::LevelFilter::TRACE,
            _ => filter::LevelFilter::OFF,
        };

        context
            .log_reload
            .as_ref()
            .unwrap()
            .modify(|filter| *filter = new_level)
            .unwrap();
    } else {
        new_level = context
            .log_reload
            .as_ref()
            .unwrap()
            .clone_current()
            .unwrap();
    }

    Ok(Some(format!(
        "Log level is set to {}",
        new_level.to_string()
    )))
}

/// Quits
fn quit<T>(_args: ArgMatches, _context: &mut T) -> Result<Option<String>> {
    std::process::exit(0);
}

/// Displays the protocol status.
///
/// Besides general inspection, the protocol status can be used to detect
/// when the key generation is completed in order to post ballots, and
/// when shuffling and decryption is completed in order to check output
/// plaintexts.
fn status<C: Ctx>(_args: ArgMatches, context: &mut ReplContext<C>) -> Result<Option<String>> {
    let stmt_keys: Vec<Vec<StatementEntryIdentifier>> = context
        .trustees
        .iter()
        .map(|t| t.local_board.statements.keys().cloned().collect())
        .collect();

    let art_keys: Vec<Vec<ArtifactEntryIdentifier>> = context
        .trustees
        .iter()
        .map(|t| t.local_board.artifacts_memory.keys().cloned().collect())
        .collect();

    let mut messages = vec![];
    for m in &context.last_messages {
        messages.push(m.try_clone().unwrap());
    }
    // let messages = context.last_messages.clone();
    let actions = context.last_actions.clone();

    let status = Status::new(
        context.cfg.clone(),
        stmt_keys,
        art_keys,
        context.remote.clone(),
        messages,
        actions,
    );
    Ok(Some(status.to_string()))
}

/// Posts random ballots.
///
/// Generates random plaintexts.
/// Encrypts the plaintexts.
/// Posts the resulting ballots.
///
/// The last generated plaintexts used as input to encryption
/// can be shown with the plaintexts command. When the protocol
/// is complete, the plaintexts and decrypted commands can
/// show the correspondence.
fn ballots<C: Ctx>(args: ArgMatches, context: &mut ReplContext<C>) -> Result<Option<String>> {
    let ctx = context.ctx.clone();
    let ballot_no = args
        .get_one::<String>("count")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(10);
    let batch = args
        .get_one::<String>("batch")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(1);
    let dkgpk = context.trustees[0]._get_dkg_public_key_nohash().unwrap();

    let pk_bytes = dkgpk.strand_serialize().unwrap();
    let pk_h = strand::hash::hash_to_array(&pk_bytes).unwrap();

    let pk_element = dkgpk.pk;
    let pk = strand::elgamal::PublicKey::from_element(&pk_element, &ctx);

    let mut rng = ctx.get_rng();
    let ps: Vec<C::P> = (0..ballot_no)
        .map(|_| ctx.rnd_plaintext(&mut rng))
        .collect();
    let ballots: Vec<Ciphertext<C>> = ps
        .iter()
        .map(|p| {
            let encoded = ctx.encode(&p).unwrap();
            pk.encrypt(&encoded)
        })
        .collect();
    context.plaintexts = ps;

    let ballot_batch = Ballots::new(ballots);
    let message = Message::ballots_msg(
        &context.cfg,
        batch,
        &ballot_batch,
        context.selected_trustees,
        PublicKeyHash(pk_h),
        &context.protocol_manager,
    )
    .unwrap();

    context.remote.add(message);

    info!(
        "{}",
        status(ArgMatches::default(), context).unwrap().unwrap()
    );
    Ok(Some(format!(
        "Generated {} ballots with batch# {} ({:?})",
        ballot_no, batch, context.selected_trustees
    )))
}

/// Shows the last plaintexts generated during ballot posting.
fn plaintexts<C: Ctx>(_args: ArgMatches, context: &mut ReplContext<C>) -> Result<Option<String>> {
    let encoded: Vec<C::E> = context
        .plaintexts
        .iter()
        .map(|p| context.ctx.encode(p).unwrap())
        .collect();
    if encoded.len() > 0 {
        Ok(Some(format!("Plaintexts {:?}", encoded)))
    } else {
        Ok(Some(format!("No plaintexts found")))
    }
}
/// Shows and checks the validity of decryptions.
///
/// Validity is checked by comparing the decrypted
/// values with the plaintext values generated when
/// posting with the ballots command.
fn decrypted<C: Ctx>(_args: ArgMatches, context: &mut ReplContext<C>) -> Result<Option<String>> {
    // FIXME hardcoded batch 1, use command line argument
    let decryptor = context.selected_trustees[0] - 1;
    if let Some(plaintexts) = context.trustees[decryptor]._get_plaintexts_nohash(1, decryptor) {
        let decrypted: Vec<C::E> = plaintexts
            .0
             .0
            .iter()
            .map(|p| context.ctx.encode(p).unwrap())
            .collect();

        let set1: HashSet<C::P> = HashSet::from_iter(plaintexts.0 .0.clone());
        let set2 = HashSet::from_iter(context.plaintexts.iter().cloned());

        Ok(Some(format!(
            "Decrypted {:?}, matches={}",
            decrypted,
            set1 == set2
        )))
    } else {
        Ok(Some(format!("No decrypted plaintexts found")))
    }
}

/// Resets the protocol with given trustees and threshold.
///
/// All trustee and bulletin board information is reset.
fn reset<C: Ctx>(args: ArgMatches, context: &mut ReplContext<C>) -> Result<Option<String>> {
    let n_trustees = args
        .get_one::<String>("trustees")
        .unwrap()
        .parse::<u8>()
        .unwrap();
    let threshold_vec: Vec<_> = args
        .get_one::<String>("threshold")
        .unwrap()
        .split(',')
        .collect();

    let threshold: Vec<usize> = threshold_vec
        .iter()
        .map(|s| s.parse::<usize>())
        .collect::<std::result::Result<Vec<usize>, _>>()
        .unwrap();

    info!("Num trustees: {:?}", n_trustees);
    info!("Threshold: {:?}", threshold);

    let reset = mk_context(context.ctx.clone(), n_trustees, &threshold);

    context.remote = reset.remote;
    context.trustees = reset.trustees;
    context.trustee_pks = reset.trustee_pks;
    context.protocol_manager = reset.protocol_manager;
    context.last_actions = HashSet::from([]);
    context.last_messages = vec![];
    context.cfg = reset.cfg;
    context.selected_trustees = reset.selected_trustees;

    status(ArgMatches::default(), context)
}

/// Executes one step of the protocol.
///
/// If a trustee index is specified, the protocol will
/// only execute for that trustee. Otherwise it will
/// execute for all trustees.
fn step<C: Ctx>(args: ArgMatches, context: &mut ReplContext<C>) -> Result<Option<String>> {
    context.last_actions = HashSet::from([]);
    context.last_messages = vec![];

    let trustee = args.get_one::<String>("trustee");
    if let Some(value) = trustee {
        let t = value.parse::<u8>()?;
        let trustee_: Option<&mut Trustee<C>> = context.trustees.get_mut(t as usize);
        if let Some(trustee) = trustee_ {
            // let (messages, actions, _last_id) = trustee.step(context.remote.get(-1)).unwrap();
            let step_result = trustee.step(&context.remote.get(-1)).unwrap();
            send(&step_result.messages, &mut context.remote);
            context.last_messages = step_result.messages;
            context.last_actions = step_result.actions;
        } else {
            return Ok(Some("Invalid trustee index".to_string()));
        }
    } else {
        for t in context.trustees.iter_mut() {
            let position = context.cfg.get_trustee_position(&t.get_pk().unwrap());
            info!(
                "====================== Running trustee {} ======================",
                position.unwrap()
            );
            //let (mut messages, actions, _last_id) = t.step(context.remote.get(-1)).unwrap();
            let mut step_result = t.step(&context.remote.get(-1)).unwrap();
            send(&step_result.messages, &mut context.remote);
            context.last_messages.append(&mut step_result.messages);
            context.last_actions.extend(&step_result.actions);
        }
    }

    status(ArgMatches::default(), context)
}

/// Simulates posting to the bulletin board.
fn send(messages: &Vec<Message>, remote: &mut VectorBoard) {
    for m in messages.iter() {
        info!("Adding message {:?} to remote", m);
        remote.add(m.try_clone().unwrap());
    }
}
