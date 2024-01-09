#![allow(non_upper_case_globals)]
#![feature(result_flattening)]
#![recursion_limit = "256"]
// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

extern crate lazy_static;

use anyhow::Result;
use sequent_core::util::init_log::init_log;

use dotenv::dotenv;
use sequent_core::services::probe::ProbeHandler;
use structopt::StructOpt;
use tracing::{event, Level};
use windmill::services::celery_app::*;
use windmill::services::database::*;
extern crate chrono;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "windmill",
    about = "Run a Rust Celery producer or consumer.",
    setting = structopt::clap::AppSettings::ColoredHelp,
)]
enum CeleryOpt {
    Consume {
        #[structopt(short, long, possible_values = &[
            "short_queue", "reports_queue", "tally_queue", "beat", "communication_queue"
        ], default_value = "beat")]
        queues: Vec<String>,
        #[structopt(short, long, default_value = "100")]
        prefetch_count: u16,
        #[structopt(short, long)]
        acks_late: bool,
    },
    Produce,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    init_log(true);

    setup_probe();

    let opt = CeleryOpt::from_args();

    match opt {
        CeleryOpt::Consume {
            queues,
            prefetch_count,
            acks_late,
        } => {
            set_prefetch_count(prefetch_count);
            set_acks_late(acks_late);
            let celery_app = get_celery_app().await;
            celery_app.display_pretty().await;

            let vec_str: Vec<&str> = queues.iter().map(AsRef::as_ref).collect();

            celery_app.consume_from(&vec_str[..]).await?;
            celery_app.close().await?;
        }
        CeleryOpt::Produce => {
            let celery_app = get_celery_app().await;
            event!(Level::INFO, "Task is empty, not adding any new tasks");
            celery_app.close().await?;
        }
    };

    Ok(())
}

fn setup_probe() {
    let addr_s = std::env::var("WINDMILL_PROBE_ADDR").unwrap_or("0.0.0.0:3030".to_string());
    let live_path = std::env::var("WINDMILL_PROBE_LIVE_PATH").unwrap_or("live".to_string());
    let ready_path = std::env::var("WINDMILL_PROBE_READY_PATH").unwrap_or("ready".to_string());

    let addr: Result<std::net::SocketAddr, _> = addr_s.parse();

    if let Ok(addr) = addr {
        let mut ph = ProbeHandler::new(&live_path, &ready_path, addr);
        let f = ph.future();
        ph.set_live(move || true);
        tokio::spawn(f);
    } else {
        tracing::warn!("Could not parse address for probe '{}'", addr_s);
    }
}
