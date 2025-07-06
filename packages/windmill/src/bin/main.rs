#![allow(non_upper_case_globals)]
#![feature(result_flattening)]
#![recursion_limit = "256"]
// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

extern crate lazy_static;
use lazy_static::lazy_static;

use anyhow::{anyhow, Result};
use celery::Celery;
use dotenv::dotenv;
use sequent_core::util::init_log::init_log;
use std::collections::HashMap;
use structopt::StructOpt;
use tokio::runtime::Builder;
use tracing::{event, Level};
use windmill::services::celery_app::*;
use windmill::services::probe::{setup_probe, AppName};
use windmill::services::tasks_semaphore::init_semaphore;

lazy_static! {
    static ref BEAT_QUEUE_NAME: String = Queue::Beat.queue_name();
    static ref SHORT_QUEUE_NAME: String = Queue::Short.queue_name();
    static ref ELECTORAL_LOG_BEAT_QUEUE_NAME: String = Queue::ElectoralLogBeat.queue_name();
    static ref COMMUNICATION_QUEUE_NAME: String = Queue::Communication.queue_name();
    static ref TALLY_QUEUE_NAME: String = Queue::Tally.queue_name();
    static ref REPORTS_QUEUE_NAME: String = Queue::Reports.queue_name();
    static ref IMPORT_EXPORT_QUEUE_NAME: String = Queue::ImportExport.queue_name();
    static ref ELECTORAL_LOG_BATCH_QUEUE_NAME: String = Queue::ElectoralLogBatch.queue_name();
}

#[derive(Debug, StructOpt, Clone)]
#[structopt(
    name = "windmill",
    about = "Windmill task queue prosumer.",
    setting = structopt::clap::AppSettings::ColoredHelp,
)]
enum CeleryOpt {
    Consume {
        #[structopt(short, long, possible_values = &[
            &*SHORT_QUEUE_NAME,
            &*BEAT_QUEUE_NAME,
            &*ELECTORAL_LOG_BEAT_QUEUE_NAME,
            &*COMMUNICATION_QUEUE_NAME,
            &*TALLY_QUEUE_NAME,
            &*REPORTS_QUEUE_NAME,
            &*IMPORT_EXPORT_QUEUE_NAME,
            &*ELECTORAL_LOG_BATCH_QUEUE_NAME,
        ], default_value = &*BEAT_QUEUE_NAME)]
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
        #[structopt(short, long)]
        worker_threads: Option<usize>,
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

fn read_worker_threads(opt: &CeleryOpt) -> usize {
    match opt.clone() {
        CeleryOpt::Consume { worker_threads, .. } => worker_threads,
        CeleryOpt::Produce => None,
    }
    .unwrap_or(num_cpus::get())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    let opt = CeleryOpt::from_args();

    let cpus = read_worker_threads(&opt);
    set_worker_threads(cpus);

    // 1) Build a custom runtime
    let rt = Builder::new_multi_thread()
        .enable_all()
        .worker_threads(cpus)
        .thread_stack_size(8 * 1024 * 1024)
        .build()?;

    // 2) Run your async code on it
    rt.block_on(async_main(opt))?;

    Ok(())
}

async fn async_main(opt: CeleryOpt) -> Result<()> {
    init_log(true);
    setup_probe(AppName::WINDMILL).await;

    let cpus = get_worker_threads();
    init_semaphore(cpus);

    match opt.clone() {
        CeleryOpt::Consume {
            queues,
            prefetch_count,
            acks_late,
            task_max_retries,
            broker_connection_max_retries,
            heartbeat,
            ..
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
            if !duplicates.is_empty() {
                return Err(anyhow!("Found duplicate queues: {:?}", duplicates));
            }
            set_queues(queues.clone());
            set_is_app_active(true);
            celery_app.consume_from(&vec_str[..]).await?;
            set_is_app_active(false);
            celery_app.close().await?;
        }
        CeleryOpt::Produce => {
            let celery_app = get_celery_app().await;
            event!(Level::INFO, "No new tasks to produce");
            celery_app.close().await?;
        }
    };
    Ok(())
}
