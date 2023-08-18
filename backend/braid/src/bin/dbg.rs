cfg_if::cfg_if! {
if #[cfg(feature = "dbg")] {


use ascii_table::{Align, AsciiTable};

use chacha20poly1305::{
    aead::{Aead, AeadCore, KeyInit},
    consts::{U12, U32},
    ChaCha20Poly1305, Nonce,
};
use log::{info, warn};
use rand::rngs::OsRng;
use reedline_repl_rs::clap::{Arg, ArgMatches, Command};
use reedline_repl_rs::{Repl, Result};
use std::collections::HashSet;
use std::iter::FromIterator;
use std::marker::PhantomData;

use tracing_attributes::instrument;
use tracing_subscriber::reload::Handle;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::{filter, reload};
use tracing_subscriber::{layer::SubscriberExt, registry::Registry};
use tracing_tree::HierarchicalLayer;

use strand::backend::ristretto::RistrettoCtx;
use strand::elgamal::{Ciphertext, PrivateKey};
use strand::serialization::StrandSerialize;
use strand::signature::{StrandSignature, StrandSignaturePk, StrandSignatureSk};

use braid::protocol2::artifact::{Configuration};
use braid::protocol2::board::local::LocalBoard;
use braid::test::vector_board::VectorBoard;
use braid::protocol2::message::Message;
use braid::protocol2::predicate::PublicKeyHash;
use braid::protocol2::trustee::ProtocolManager;
use braid::protocol2::trustee::Trustee;
use strand::context::Ctx;
use strand::context::Exponent;
use braid::protocol2::action::Action;

struct ReplContext<C: Ctx> {
    pub ctx: C,
    pub cfg: Configuration<C>,
    pub session_id: u128,
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
                        k.artifact_type, k.statement_entry.signer_position
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
            let sender = self.cfg.get_trustee_position(&m.signer_key).unwrap();
            data.push(vec![
                format!("{:?}", m.statement.get_kind()),
                format!("{}", sender),
                m.artifact.is_some().to_string(),
            ])
        }
        boards.push("Last step messages".to_string());
        boards.push(ascii_table.format(data));

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
            let sender = self.cfg.get_trustee_position(&m.signer_key).unwrap();
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

fn main() {
    let log_reload = init_log();
    let ctx = RistrettoCtx;
    dbg(ctx, log_reload).unwrap();
}

fn init_log() -> Handle<LevelFilter, Registry> {
    let layer = HierarchicalLayer::default()
        .with_writer(std::io::stdout)
        .with_indent_lines(true)
        .with_indent_amount(3)
        .with_thread_names(false)
        .with_thread_ids(false)
        .with_verbose_exit(false)
        .with_verbose_entry(false)
        .with_targets(false);

    let filter = filter::LevelFilter::INFO;
    let (filter, reload_handle) = reload::Layer::new(filter);
    let subscriber = Registry::default().with(filter).with(layer);

    tracing::subscriber::set_global_default(subscriber).unwrap();
    tracing_log::LogTracer::init().unwrap();
    reload_handle
}

fn mk_context<C: Ctx>(ctx: C, n_trustees: u8, threshold: &[usize]) -> ReplContext<C> {
    let mut csprng = strand::rnd::StrandRng;
    let session_id = 0;

    let mut selected = [1001usize; 12];
    selected[0..threshold.len()].copy_from_slice(&threshold);


    let pmkey: StrandSignatureSk = StrandSignatureSk::new(&mut csprng);
    let pm: ProtocolManager<C> = ProtocolManager {
        signing_key: pmkey,
        phantom: PhantomData,
    };

    let trustees: Vec<Trustee<C>> = (0..n_trustees)
        .into_iter()
        .map(|_| {
            let kp = StrandSignatureSk::new(&mut csprng);
            let encryption_key = ChaCha20Poly1305::generate_key(&mut chacha20poly1305::aead::OsRng);
            Trustee::new(kp, encryption_key)
        })
        .collect();

    let trustee_pks: Vec<StrandSignaturePk> = trustees
        .iter()
        .map(|t| StrandSignaturePk::from(&t.signing_key))
        .collect();

    let cfg = Configuration::<C>::new(
        0,
        StrandSignaturePk::from(&pm.signing_key),
        trustee_pks.clone(),
        threshold.len(),
        PhantomData,
    );

    let mut remote = VectorBoard::new(session_id);
    let message = Message::bootstrap_msg(&cfg, &pm).unwrap();
    remote.add(message);

    ReplContext {
        ctx,
        cfg,
        session_id,
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
    let mut new_level = filter::LevelFilter::INFO;
    let l = args.value_of("level");
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
    let messages = context.last_messages.clone();
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
    let batch = args.value_of("batch").unwrap().parse::<usize>().unwrap();
    let ballot_no_str = args.value_of("count").unwrap_or("10");
    let ballot_no = ballot_no_str.parse::<usize>().unwrap();

    let dkgpk = context.trustees[0].get_dkg_public_key_nohash().unwrap();

    let pk_bytes = dkgpk.strand_serialize().unwrap();
    let pk_h = strand::util::hash_array(&pk_bytes);

    let pk_element = dkgpk.pk;
    let pk = strand::elgamal::PublicKey::from_element(&pk_element, &ctx);

    let ps: Vec<C::P> = (0..ballot_no).map(|i| ctx.rnd_plaintext()).collect();
    let ballots: Vec<Ciphertext<C>> = ps
        .iter()
        .map(|p| {
            let encoded = ctx.encode(&p).unwrap();
            pk.encrypt(&encoded)
        })
        .collect();
    context.plaintexts = ps;

    info!("selected_trustees {:?}", context.selected_trustees);
    let ballot_batch = braid::protocol2::artifact::Ballots::new(ballots, context.selected_trustees, &context.cfg);
    let message = braid::protocol2::message::Message::ballots_msg(
        &context.cfg,
        batch,
        &ballot_batch,
        PublicKeyHash(pk_h),
        &context.protocol_manager,
    ).unwrap();

    context.remote.add(message);


    info!("{}", status(ArgMatches::default(), context).unwrap().unwrap());
    Ok(Some(format!("Generated {} ballots with batch# {} ({:?})", ballot_no, batch, context.selected_trustees)))
}

fn plaintexts<C: Ctx>(args: ArgMatches, context: &mut ReplContext<C>) -> Result<Option<String>> {
    let encoded: Vec<C::E> = context
        .plaintexts
        .iter()
        .map(|p| context.ctx.encode(p).unwrap())
        .collect();
    Ok(Some(format!("Plaintexts {:?}", encoded)))
}
fn decrypted<C: Ctx>(args: ArgMatches, context: &mut ReplContext<C>) -> Result<Option<String>> {
    if let Some(plaintexts) = context.trustees[0].get_plaintexts_nohash(1) {
        let decrypted: Vec<C::E> = plaintexts
            .0.0
            .iter()
            .map(|p| context.ctx.encode(p).unwrap())
            .collect();

        let set1: HashSet<C::P> = HashSet::from_iter(plaintexts.0.0);
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
    let n_trustees = args.value_of("trustees").unwrap().parse::<u8>().unwrap();
    let threshold = args
        .value_of("threshold")
        .unwrap()
        .parse::<usize>()
        .unwrap();

    let t = [1, 2, 3, 4, 5, 6, 7, 8];
    let reset = mk_context(context.ctx.clone(), n_trustees, &t[0..threshold]);

    if let Some(plaintexts) = context.trustees[0].get_plaintexts_nohash(1) {
        info!("Plaintexts {:?}", plaintexts);
    } else {
        info!("No plaintexts found");
    }

    context.remote = reset.remote;
    context.trustees = reset.trustees;
    context.trustee_pks = reset.trustee_pks;
    context.protocol_manager = reset.protocol_manager;
    context.last_actions = HashSet::from([]);
    context.last_messages = vec![];
    context.cfg = reset.cfg;

    status(ArgMatches::default(), context)
}

fn step<C: Ctx>(args: ArgMatches, context: &mut ReplContext<C>) -> Result<Option<String>> {
    context.last_actions = HashSet::from([]);
    context.last_messages = vec![];

    let trustee = args.value_of("trustee");
    if let Some(value) = trustee {
        let t = value.parse::<u8>()?;
        let trustee_: Option<&mut Trustee<C>> = context.trustees.get_mut(t as usize);
        if let Some(trustee) = trustee_ {
            let (messages, actions) = trustee.step(context.remote.get(0)).unwrap();
            send(&messages, &mut context.remote);
            context.last_messages = messages;
            context.last_actions = actions;
        } else {
            return Ok(Some("Invalid trustee index".to_string()));
        }
    } else {
        for t in context.trustees.iter_mut() {
            let position = context.cfg.get_trustee_position(&t.get_pk());
            info!(
                "====================== Running trustee {} ======================",
                position.unwrap()
            );
            let (mut messages, actions) = t.step(context.remote.get(0)).unwrap();
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
        remote.add(m.clone());
    }
}

#[instrument(skip(log_reload))]
fn dbg<C: Ctx>(ctx: C, log_reload: Handle<LevelFilter, Registry>) -> Result<()> {
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
                .arg(Arg::new("trustees").required(true))
                .arg(Arg::new("threshold").required(true))
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
}

else {
    fn main() {
        println!("Requires the 'dbg' feature");
    }
}
}
