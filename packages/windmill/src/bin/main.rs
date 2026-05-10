#![allow(non_upper_case_globals)]
#![recursion_limit = "256"]
//! Celery worker binary for Windmill: runs the Celery app as a queue consumer or in produce-only mode.
// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

extern crate lazy_static;
use lazy_static::lazy_static;

use anyhow::Context;
use anyhow::{anyhow, Result};
use celery::Celery;
use clap::Parser;
use dotenv::dotenv;
use sequent_core::util::init_log::init_log;
use std::collections::HashMap;
use tokio::runtime::Builder;
use tracing::{event, Level};
use windmill::services::celery_app::*;
use windmill::services::probe::{setup_probe, AppName};
use windmill::services::tasks_semaphore::init_semaphore;

/// Returns the AMQP queue name for `queue` prefixed with `ENV_SLUG`.
///
/// # Panics
///
/// Panics if `ENV_SLUG` is not set in the environment.
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

/// Celery options for the Windmill Celery worker process.
#[derive(Debug, Parser, Clone)]
#[command(name = "windmill", about = "Windmill task queue prosumer.")]
enum CeleryOpt {
    /// Consume tasks from one or more AMQP queues.
    Consume {
        /// Queue names to bind.
        #[arg(short, long, num_args(1..), default_values_t = vec![BEAT_QUEUE_NAME.clone()])]
        queues: Vec<String>,
        /// Maximum unacknowledged messages per consumer.
        #[arg(short, long, default_value = "100")]
        prefetch_count: u16,
        /// When true, acknowledgements are sent after the task body returns.
        #[arg(short, long)]
        acks_late: bool,
        /// Default retry cap Celery applies before marking a task failed.
        #[arg(short, long, default_value = "4")]
        task_max_retries: u32,
        /// Retries when establishing the broker connection before exiting.
        #[arg(short, long, default_value = "5")]
        broker_connection_max_retries: u32,
        /// Broker heartbeat interval in seconds.
        #[arg(short = 'H', long, default_value = "10")]
        heartbeat: u16,
        /// Tokio worker thread count for the runtime (defaults to logical CPUs).
        #[arg(short, long)]
        worker_threads: Option<usize>,
    },
    /// Connect to the broker, log readiness, and exit without consuming.
    Produce,
}

/// Finds duplicates in a vector of strings.
fn find_duplicates(input: Vec<&str>) -> Vec<&str> {
    let mut occurrences = HashMap::new();
    let mut duplicates = Vec::new();
    for &item in &input {
        let count: &mut i32 = occurrences.entry(item).or_insert(0);
        *count = (*count)
            .checked_add(1)
            .expect("occurrence counter overflow");
    }
    for (&item, &count) in &occurrences {
        if count > 1 {
            duplicates.push(item);
        }
    }
    duplicates
}

/// Resolves the async runtime worker thread count from `Consume` options or CPU count.
fn read_worker_threads(opt: &CeleryOpt) -> usize {
    match opt.clone() {
        CeleryOpt::Consume { worker_threads, .. } => worker_threads,
        CeleryOpt::Produce => None,
    }
    .unwrap_or(num_cpus::get())
}

/// Entry point: builds the multi-thread runtime and runs async worker.
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

/// Runs the Celery app.
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
