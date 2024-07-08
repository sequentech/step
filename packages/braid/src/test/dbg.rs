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
use strand::serialization::StrandSerialize;
use strand::signature::{StrandSignaturePk, StrandSignatureSk};

use crate::protocol::action::Action;
use crate::protocol::board::local::LocalBoard;
use board_messages::braid::artifact::Ballots;
use board_messages::braid::artifact::Configuration;
use board_messages::braid::message::Message;
use board_messages::braid::newtypes::PublicKeyHash;
use board_messages::braid::newtypes::NULL_TRUSTEE;
use board_messages::braid::protocol_manager::ProtocolManager;

use crate::protocol::trustee::Trustee;
use crate::test::vector_board::VectorBoard;
use board_messages::braid::newtypes::MAX_TRUSTEES;

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
            Command::new("reset")
                .arg(Arg::new("trustees").required(true).help("The total number of trustees"))
                .arg(Arg::new("threshold").required(true).help("The trustees that will participate in the protocol (eg '1,2')"))
                .about("Reset the run"),
            reset,
        )
        .with_command(
            Command::new("step")
                .arg(Arg::new("trustee").required(false))
                .about("Execute one step of the protocol"),
            step,
        )
        .with_command(
            Command::new("status").about("Shows remote board and localboard"),
            status,
        )
        .with_command(
            Command::new("plaintexts").about("Shows the last encrypted plaintexts"),
            plaintexts,
        )
        .with_command(
            Command::new("decrypted").about("Shows the last decrypted plaintexts and whether they match the encrypted plaintexts"),
            decrypted,
        )
        .with_command(
            Command::new("log")
                .arg(Arg::new("level").required(false))
                .about("Set log level (0-6)"),
            log,
        )
        .with_command(
            Command::new("ballots")
                .arg(Arg::new("batch").required(true))
                .arg(Arg::new("count").required(false))
                .about("Post a ballot batch"),
            ballots,
        )
        .with_command(Command::new("quit").about("quit"), quit);
    repl.run()
}

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

struct Status<C: Ctx> {
    cfg: Configuration<C>,
    locals: Vec<LocalBoard<C>>,
    remote: VectorBoard,
    last_messages: Vec<Message>,
    last_actions: HashSet<Action>,
}
impl<C: Ctx> Status<C> {
    fn new(
        cfg: Configuration<C>,
        locals: Vec<LocalBoard<C>>,
        remote: VectorBoard,
        last_messages: Vec<Message>,
        last_actions: HashSet<Action>,
    ) -> Status<C> {
        Status {
            cfg,
            locals,
            remote,
            last_messages,
            last_actions,
        }
    }
    fn to_string(&self) -> String {
        let mut boards = vec![];

        boards.push("Trustees".to_string());
        for (i, b) in self.locals.iter().enumerate() {
            let mut ascii_table = AsciiTable::default();
            ascii_table.set_max_width(205);
            ascii_table
                .column(0)
                .set_header(format!("trustee {}", i.to_string()))
                .set_align(Align::Left);

            let data1: Vec<String> = b
                .statements
                .keys()
                .map(|k| format!("{}-{}", k.kind.to_string(), k.signer_position))
                .collect();
            let data2: Vec<String> = b
                .artifacts
                .keys()
                .map(|k| {
                    format!(
                        "{}-{}",
                        k.statement_entry.kind, k.statement_entry.signer_position
                    )
                })
                .collect();
            let mut data3 = vec![format!("{:?}", b.configuration)];

            data3.insert(0, "cfg:".to_string());
            let data: Vec<Vec<String>> = vec![
                vec!["stm:".to_string(), data1.join(" ")],
                vec!["art:".to_string(), data2.join(" ")],
                data3,
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
            boards.push("* None *".to_string());
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
        for (m, id) in self.remote.messages.iter() {
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
        boards.push(format!("{:?}", self.last_actions));

        boards.join("\r\n")
    }
}

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
            Trustee::new(i.to_string(), kp, encryption_key)
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

fn quit<T>(_args: ArgMatches, _context: &mut T) -> Result<Option<String>> {
    std::process::exit(0);
}

fn status<C: Ctx>(_args: ArgMatches, context: &mut ReplContext<C>) -> Result<Option<String>> {
    let locals: Vec<LocalBoard<C>> = context
        .trustees
        .iter()
        .map(|t| t.copy_local_board())
        .collect();
    let mut messages = vec![];
    for m in &context.last_messages {
        messages.push(m.try_clone().unwrap());
    }
    // let messages = context.last_messages.clone();
    let actions = context.last_actions.clone();

    let status = Status::new(
        context.cfg.clone(),
        locals,
        context.remote.clone(),
        messages,
        actions,
    );
    Ok(Some(status.to_string()))
}

fn ballots<C: Ctx>(args: ArgMatches, context: &mut ReplContext<C>) -> Result<Option<String>> {
    let ctx = context.ctx.clone();
    let batch = args
        .get_one::<String>("batch")
        .unwrap()
        .parse::<usize>()
        .unwrap();
    let ballot_no = args
        .get_one::<String>("count")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(10);
    let dkgpk = context.trustees[0].get_dkg_public_key_nohash().unwrap();

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

    info!("selected_trustees {:?}", context.selected_trustees);
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

fn plaintexts<C: Ctx>(_args: ArgMatches, context: &mut ReplContext<C>) -> Result<Option<String>> {
    let encoded: Vec<C::E> = context
        .plaintexts
        .iter()
        .map(|p| context.ctx.encode(p).unwrap())
        .collect();
    Ok(Some(format!("Plaintexts {:?}", encoded)))
}
fn decrypted<C: Ctx>(_args: ArgMatches, context: &mut ReplContext<C>) -> Result<Option<String>> {
    // FIXME hardcoded batch 1, use command line argument
    let decryptor = context.selected_trustees[0] - 1;
    if let Some(plaintexts) = context.trustees[decryptor].get_plaintexts_nohash(1, decryptor) {
        let decrypted: Vec<C::E> = plaintexts
            .0
             .0
            .iter()
            .map(|p| context.ctx.encode(p).unwrap())
            .collect();

        let set1: HashSet<C::P> = HashSet::from_iter(plaintexts.0 .0);
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

    info!("Threshold {:?}", threshold);

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

fn step<C: Ctx>(args: ArgMatches, context: &mut ReplContext<C>) -> Result<Option<String>> {
    context.last_actions = HashSet::from([]);
    context.last_messages = vec![];

    let trustee = args.get_one::<String>("trustee");
    if let Some(value) = trustee {
        let t = value.parse::<u8>()?;
        let trustee_: Option<&mut Trustee<C>> = context.trustees.get_mut(t as usize);
        if let Some(trustee) = trustee_ {
            let (messages, actions, _last_id) = trustee.step(context.remote.get(-1)).unwrap();
            send(&messages, &mut context.remote);
            context.last_messages = messages;
            context.last_actions = actions;
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
            let (mut messages, actions, _last_id) = t.step(context.remote.get(-1)).unwrap();
            send(&messages, &mut context.remote);
            context.last_messages.append(&mut messages);
            context.last_actions.extend(&actions);
        }
    }

    status(ArgMatches::default(), context)
}

fn send(messages: &Vec<Message>, remote: &mut VectorBoard) {
    for m in messages.iter() {
        info!("Adding message {:?} to remote", m);
        remote.add(m.try_clone().unwrap());
    }
}
