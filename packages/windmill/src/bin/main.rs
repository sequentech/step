#![allow(non_upper_case_globals)]
#![feature(result_flattening)]
#![recursion_limit = "256"]
// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

extern crate lazy_static;

use std::collections::HashMap;

use anyhow::{anyhow, Result};
use sequent_core::util::init_log::init_log;

use dotenv::dotenv;
use structopt::StructOpt;
use tokio::runtime::Builder;
use tracing::{event, Level};
use windmill::services::celery_app::*;
use windmill::services::probe::{setup_probe, AppName};
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
            "short_queue", "reports_queue", "tally_queue", "beat", "communication_queue", "import_export_queue"
        ], default_value = "beat")]
        queues: Vec<String>,
        #[structopt(short, long, default_value = "100")]
        prefetch_count: u16,
        #[structopt(short, long)]
        acks_late: bool,
        #[structopt(short, long, default_value = "4")]
        task_max_retries: u32,
        #[structopt(short, long, default_value = "5")]
        broker_connection_max_retries: u32,
        #[structopt(short, long, default_value = "10")]
        heartbeat: u16,
    },
    Produce,
}

fn find_duplicates(input: Vec<&str>) -> Vec<&str> {
    let mut occurrences = HashMap::new();
    let mut duplicates = Vec::new();

    for &item in &input {
        let count = occurrences.entry(item).or_insert(0);
        *count += 1;
    }

    for (&item, &count) in &occurrences {
        if count > 1 {
            duplicates.push(item);
        }
    }

    duplicates
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cpus = num_cpus::get();

    // 1) Build a custom runtime
    let rt = Builder::new_multi_thread()
        .enable_all()
        .worker_threads(cpus)
        .thread_stack_size(8 * 1024 * 1024)
        .build()?;

    // 2) Run your async code on it
    rt.block_on(async_main())?;

    Ok(())
}

async fn async_main() -> Result<()> {
    dotenv().ok();
    init_log(true);

    setup_probe(AppName::WINDMILL).await;

    let opt = CeleryOpt::from_args();

    match opt {
        CeleryOpt::Consume {
            queues,
            prefetch_count,
            acks_late,
            task_max_retries,
            broker_connection_max_retries,
            heartbeat,
        } => {
            set_prefetch_count(prefetch_count);
            set_acks_late(acks_late);
            set_task_max_retries(task_max_retries);
            set_broker_connection_max_retries(broker_connection_max_retries);
            set_heartbeat(heartbeat);
            let celery_app = get_celery_app().await;
            celery_app.display_pretty().await;

            let vec_str: Vec<&str> = queues.iter().map(AsRef::as_ref).collect();

            let duplicates = find_duplicates(vec_str.clone());
            if duplicates.len() > 0 {
                return Err(anyhow!("Found duplicate queues: {:?}", duplicates));
            }

            set_is_app_active(true);
            celery_app.consume_from(&vec_str[..]).await?;
            set_is_app_active(false);
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
