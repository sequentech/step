// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;

use braid::protocol::board::grpc_m::GrpcB3Index;
use clap::Parser;
use log::warn;
use std::io::Write;
use std::collections::HashSet;
use std::fs;
use std::hash::Hasher;
use std::path::PathBuf;

use tokio::time::{sleep, Duration};
use tracing::instrument;
use tracing::{error, info};

use rustc_hash::FxHasher;

use tokio::sync::mpsc::{Sender, Receiver};

use braid::protocol::session::session_m::{SessionFactory, SessionSet, SessionSetMessage};
use braid::protocol::trustee2::TrusteeConfig;

cfg_if::cfg_if! {
    if #[cfg(feature = "jemalloc")] {
        use tikv_jemalloc_ctl::{stats, epoch};

        #[global_allocator]
        static ALLOC: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;
    }
}

#[derive(Parser)]
struct Cli {
    #[arg(short, long)]
    server_url: String,

    #[arg(short, long)]
    trustee_config: PathBuf,

    #[arg(short, long, default_value_t = false)]
    no_cache: bool,

    #[arg(short, long, default_value_t = 1)]
    tokio_workers: usize,

    #[arg(short, long, default_value_t = 1)]
    session_workers: usize,
}

fn main() -> Result<()> {

    let args = Cli::parse();

    // let runtime = tokio::runtime::Builder::new_current_thread()
    let runtime = tokio::runtime::Builder::new_multi_thread()
    .worker_threads(args.tokio_workers)
    .max_blocking_threads(10)
    .enable_all()
    .build()
    .unwrap();
    
    runtime.block_on(async { 
        run(&args).await
    })
}

/*
Entry point for a braid mixnet trustee.

Example run command

cargo run --release --bin main_m -- --server-url --server-url http://127.0.0.1:50051 --trustee-config trustee.toml

A mixnet trustee will periodically:

    1) Poll the board index for active protocol boards
    2) For each protocol board
        a) Poll the protocol board for new messages
        b) Update the local store with new messages
        c) Execute the protocol with the existing messages in the local store
*/
#[instrument(skip_all)]
async fn run(args: &Cli) -> Result<()> {
    braid::util::init_log(true);

    let default_panic = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            default_panic(info);
            std::process::exit(1);
    }));

    cfg_if::cfg_if! {
        if #[cfg(feature = "jemalloc")] {
            let e = epoch::mib().unwrap();
            let allocated = stats::allocated::mib().unwrap();
            let resident = stats::resident::mib().unwrap();
        }
    }

    let contents = fs::read_to_string(args.trustee_config.clone())
        .expect("Should have been able to read the trustee configuration file");

    info!("{}", strand::info_string());

    let tc: TrusteeConfig = toml::from_str(&contents).unwrap();

    let ignored_boards = get_ignored_boards();
    info!("ignored boards {:?}", ignored_boards);

    let store_root = std::env::current_dir().unwrap().join("message_store");
    braid::util::ensure_directory(store_root.clone())?;

    let store_root = std::env::current_dir().unwrap().join("message_store");
    
    let trustee_name = std::env::var("TRUSTEE_NAME")
        .unwrap_or(args.trustee_config.clone().into_os_string().into_string().unwrap());

    let factory = SessionFactory::new(&trustee_name, tc, store_root, args.no_cache)?;
    let mut master = SessionMaster::new(&args.server_url, factory, args.session_workers)?;

    loop {
        
        let b3index = GrpcB3Index::new(&args.server_url);
        let boards_result = b3index.get_boards().await;

        let Ok(mut boards) = boards_result else {
            error!(
                "Error listing board names: '{}' ({})",
                boards_result.err().unwrap(),
                args.server_url
            );
            sleep(Duration::from_millis(1000)).await;
            continue;
        };

        boards.retain(|b| !ignored_boards.contains(b));
        
        master.refresh_sets(boards).await?;

        cfg_if::cfg_if! {
            if #[cfg(feature = "jemalloc")] {
                // Many statistics are cached and only updated
                // when the epoch is advanced:
                let e_ = e.advance();
                let alloc = allocated.read();
                let res = resident.read();
                let mb = 1024 * 1024;

                if let(Ok(_), Ok(alloc), Ok(res)) = (e_, alloc, res) {
                    info!("{} MB allocated / {} MB resident ({} boards)", (alloc / mb), (res / mb), boards.len());
                }
            }
        }

        let _ = std::io::stdout().flush();
        sleep(Duration::from_millis(5000)).await;
    }
}

struct SessionSetHandle {
    boards: Vec<String>,
    sender: Sender<SessionSetMessage>,
}
impl SessionSetHandle {
    fn new(sender: Sender<SessionSetMessage>) -> Self {
        SessionSetHandle {
            boards: vec![],
            sender
        }
    }
}

struct SessionMaster {
    session_sets: Vec<SessionSetHandle>,
    b3_url: String,
    session_factory: SessionFactory,
}
impl SessionMaster {
    fn new(b3_url: &str, session_factory: SessionFactory, size: usize) -> Result<Self> {
        
        let mut session_sets = vec![];
        let mut runners = vec![];
        for i in 0..size {
            let (s,r): (Sender<SessionSetMessage>, Receiver<SessionSetMessage>)
                    = tokio::sync::mpsc::channel(1);
            let session_set = SessionSet::new(&i.to_string(), &session_factory, &b3_url, r)?;
            runners.push(session_set);
            
            let handle = SessionSetHandle::new(s);
            session_sets.push(handle);
        }

        info!("* Starting {} session sets..", runners.len());
        runners.into_iter().for_each(|r| {r.run();});
        
        Ok(SessionMaster {
            b3_url: b3_url.to_string(),
            session_factory,
            session_sets,
        })
    }

    fn hash(&self, board: &str) -> usize {
        let mut hasher = FxHasher::default();
        hasher.write(board.as_bytes());
        let ret = hasher.finish() % self.session_sets.len() as u64;

        ret as usize
    }

    async fn refresh_sets(&mut self, boards: Vec<String>) -> Result<()> {        
        
        // info!("Refreshing {} sets with {} boards", self.session_sets.len(), boards.len());
        
        for board in boards {
            let index = self.hash(&board);
            // Assign boards to session sets
            self.session_sets[index].boards.push(board);
        }
        
        for (i, h) in self.session_sets.iter_mut().enumerate() {
            let boards = std::mem::replace(&mut h.boards, vec![]);
            

            if h.sender.is_closed() {
                warn!("Sender was closed, rebuilding set..");
                let (s,r): (Sender<SessionSetMessage>, Receiver<SessionSetMessage>)
                = tokio::sync::mpsc::channel(1);
                let session_set = SessionSet::new(&format!("rebuilt {}", i), &self.session_factory, &self.b3_url, r)?;
                h.sender = s;
                
                session_set.run();
            }
            // The only error we care about is checked above with sender.is_closed
            let _ = h.sender.try_send(SessionSetMessage::REFRESH(boards));
        }

        Ok(())
    }
}

fn get_ignored_boards() -> HashSet<String> {
    let boards_str: String = std::env::var("IGNORE_BOARDS").unwrap_or_else(|_| "".into());
    HashSet::from_iter(boards_str.split(',').map(|s| s.to_string()))
}