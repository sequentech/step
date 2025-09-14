#![allow(non_upper_case_globals)]
#![feature(result_flattening)]
#![recursion_limit = "256"]
// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

extern crate lazy_static;
use lazy_static::lazy_static;

use anyhow::Context;
use anyhow::{anyhow, Result};
use celery::Celery;
use dotenv::dotenv;
use sequent_core::util::init_log::init_log;
use std::collections::HashMap;
use clap::Parser;
use tokio::runtime::Builder;
use tracing::{event, Level};
use windmill::services::celery_app::*;
use windmill::services::probe::{setup_probe, AppName};
use windmill::services::tasks_semaphore::init_semaphore;

fn get_queue_name(queue: Queue) -> String {
    let slug = std::env::var("ENV_SLUG")
        .with_context(|| "missing env var ENV_SLUG")
        .unwrap();
    queue.queue_name(&slug)
}

lazy_static! {
    static ref BEAT_QUEUE_NAME: String = get_queue_name(Queue::Beat);
    static ref SHORT_QUEUE_NAME: String = get_queue_name(Queue::Short);
    static ref ELECTORAL_LOG_BEAT_QUEUE_NAME: String = get_queue_name(Queue::ElectoralLogBeat);
    static ref COMMUNICATION_QUEUE_NAME: String = get_queue_name(Queue::Communication);
    static ref TALLY_QUEUE_NAME: String = get_queue_name(Queue::Tally);
    static ref REPORTS_QUEUE_NAME: String = get_queue_name(Queue::Reports);
    static ref IMPORT_EXPORT_QUEUE_NAME: String = get_queue_name(Queue::ImportExport);
    static ref ELECTORAL_LOG_BATCH_QUEUE_NAME: String = get_queue_name(Queue::ElectoralLogBatch);
}

#[derive(Debug, Parser, Clone)]
#[command(
    name = "windmill",
    about = "Windmill task queue prosumer."
)]
enum CeleryOpt {
    Consume {
        #[arg(short, long, default_values_t = vec![BEAT_QUEUE_NAME.clone()])]
        queues: Vec<String>,
        #[arg(short, long, default_value = "100")]
        prefetch_count: u16,
        #[arg(short, long)]
        acks_late: bool,
        #[arg(short, long, default_value = "4")]
        task_max_retries: u32,
        #[arg(short, long, default_value = "5")]
        broker_connection_max_retries: u32,
        #[arg(short, long, default_value = "10")]
        heartbeat: u16,
        #[arg(short, long)]
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

    let opt = CeleryOpt::parse();

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
    let slug = std::env::var("ENV_SLUG").with_context(|| "missing env var ENV_SLUG")?;

    match opt.clone() {
        CeleryOpt::Consume {
            queues: queues_input,
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
            let queues: Vec<String> = queues_input
                .iter()
                .map(|queue_name| {
                    if queue_name.starts_with(&slug) {
                        queue_name.clone()
                    } else {
                        format!("{}_{}", slug, queue_name)
                    }
                })
                .collect();

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
