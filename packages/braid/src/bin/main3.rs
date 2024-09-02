// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;

use braid::protocol::board::grpc2::GrpcB3Index;
use clap::Parser;
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

use tokio::time::{sleep, Duration};
use tracing::instrument;
use tracing::{error, info};

use hashring::HashRing;

use tokio::sync::mpsc::{Sender, Receiver};

use braid::protocol::session2::{SessionFactory, SessionSet, SessionSetMessage};
use braid::protocol::trustee::TrusteeConfig;

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

    #[arg(long, default_value_t = false)]
    strict: bool,

    #[arg(long, default_value_t = 300)]
    session_reset_period: u64,
}

fn get_ignored_boards() -> HashSet<String> {
    let boards_str: String = std::env::var("IGNORE_BOARDS").unwrap_or_else(|_| "".into());
    HashSet::from_iter(boards_str.split(',').map(|s| s.to_string()))
}

/*
Entry point for a braid mixnet trustee.

Example run command

cargo run --release --bin main  -- --server-url http://immudb:3322 --board-index defaultboardindex--trustee-config trustee.toml

A mixnet trustee will periodically:

    1) Poll the board index for active protocol boards
    2) For each protocol board
        a) Poll the protocol board for new messages
        b) Update the local store with new messages
        c) Execute the protocol with the existing messages in the local store

The process will loop indefinitely unless an error is encountered and the 'strict'
command line option is set to true.
*/
#[tokio::main]
#[instrument]
async fn main() -> Result<()> {
    braid::util::init_log(true);

    cfg_if::cfg_if! {
        if #[cfg(feature = "jemalloc")] {
            let e = epoch::mib().unwrap();
            let allocated = stats::allocated::mib().unwrap();
            let resident = stats::resident::mib().unwrap();
        }
    }

    let args = Cli::parse();

    let contents = fs::read_to_string(args.trustee_config)
        .expect("Should have been able to read the trustee configuration file");

    info!("{}", strand::info_string());

    let tc: TrusteeConfig = toml::from_str(&contents).unwrap();

    let ignored_boards = get_ignored_boards();
    info!("ignored boards {:?}", ignored_boards);

    let store_root = std::env::current_dir().unwrap().join("message_store");
    braid::util::ensure_directory(store_root.clone())?;

    let store_root = std::env::current_dir().unwrap().join("message_store");
    
    let trustee_name = std::env::var("TRUSTEE_NAME").unwrap_or_else(|_| "Self".to_string());

    let factory = SessionFactory::new(&trustee_name, tc, store_root)?;
    let mut ring = SessionSetRing::new(&args.server_url, factory, 10)?;

    
    let mut loop_count: u64 = 0;
    loop {
        info!("{} >", loop_count);

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
        
        ring.refresh(boards).await?;

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

        loop_count = (loop_count + 1) % u64::MAX;
        println!("");
        sleep(Duration::from_millis(1000)).await;
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

struct SessionSetRing {
    session_sets: Vec<SessionSetHandle>,
    b3_url: String,
    session_factory: SessionFactory,
    ring: HashRing<usize>
}
impl SessionSetRing {
    fn new(b3_url: &str, session_factory: SessionFactory, size: usize) -> Result<Self> {
        
        let mut ring = HashRing::new();
        let mut session_sets = vec![];
        let mut runners = vec![];
        for i in 0..size {
            let (s,r): (Sender<SessionSetMessage>, Receiver<SessionSetMessage>)
                    = tokio::sync::mpsc::channel(10);
            let session_set = SessionSet::new(&session_factory, &b3_url, r)?;
            runners.push(session_set);
            
            let handle = SessionSetHandle::new(s);
            session_sets.push(handle);

            ring.add(i);
        }

        info!("* Starting {} session sets..", runners.len());
        runners.into_iter().for_each(|r| {r.run();});
        
        Ok(SessionSetRing {
            b3_url: b3_url.to_string(),
            session_factory,
            ring,
            session_sets,
        })
    }

    async fn refresh(&mut self, boards: Vec<String>) -> Result<()> {        
        
        for board in boards {

            let index = self.ring.get(&board);
            let Some(index) = index else {
                // This is impossible
                error!("No session set index was returned for board '{}'", board);
                continue;
            };
            
            self.session_sets[*index].boards.push(board);
        }

        for (i, h) in self.session_sets.iter_mut().enumerate() {
            let boards = std::mem::replace(&mut h.boards, vec![]);
            info!("Refresing set {} with {:?}", i, boards);

            if h.sender.is_closed() {
                let (s,r): (Sender<SessionSetMessage>, Receiver<SessionSetMessage>)
                = tokio::sync::mpsc::channel(10);
                let session_set = SessionSet::new(&self.session_factory, &self.b3_url, r)?;
                h.sender = s;
                
                session_set.run();
            }
            h.sender.send(SessionSetMessage::REFRESH(boards)).await?;
        }

        Ok(())
    }
}