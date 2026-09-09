#![allow(non_upper_case_globals)]
#![recursion_limit = "256"]
// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Context;
use anyhow::{anyhow, Result};
use clap::Parser;
use dotenv::dotenv;
use sequent_core::util::init_log::init_log;
use std::collections::HashMap;
use std::sync::LazyLock;
use tokio::runtime::Builder;
use tracing::{event, Level};
use windmill::services::celery_app::{self as celery_cfg, Queue};
use windmill::services::probe::{setup_probe, AppName};
use windmill::services::tasks_semaphore::init_semaphore;

fn get_queue_name(queue: Queue) -> String {
    let slug = std::env::var("ENV_SLUG")
        .with_context(|| "missing env var ENV_SLUG")
        .unwrap();
    queue.queue_name(&slug)
}

static BEAT_QUEUE_NAME: LazyLock<String> = LazyLock::new(|| get_queue_name(Queue::Beat));

#[derive(Debug, Parser, Clone)]
#[command(name = "windmill", about = "Windmill task queue prosumer.")]
enum CeleryOpt {
    Consume {
        #[arg(short, long, num_args(1..), default_values_t = vec![BEAT_QUEUE_NAME.clone()])]
        queues: Vec<String>,
        #[arg(short, long, default_value = "100")]
        prefetch_count: u16,
        #[arg(short, long)]
        acks_late: bool,
        #[arg(short, long, default_value = "4")]
        task_max_retries: u32,
        #[arg(short, long, default_value = "5")]
        broker_connection_max_retries: u32,
        #[arg(short = 'H', long, default_value = "10")]
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

    let worker_threads = read_worker_threads(&opt);
    celery_cfg::set_worker_threads(worker_threads);

    // 1) Build a custom runtime
    let rt = Builder::new_multi_thread()
        .enable_all()
        .worker_threads(worker_threads)
        .thread_stack_size(8 * 1024 * 1024)
        .build()?;

    // 2) Run your async code on it
    rt.block_on(async_main(opt))?;

    Ok(())
}

async fn async_main(opt: CeleryOpt) -> Result<()> {
    init_log(true);
    setup_probe(AppName::WINDMILL).await;

    let cpus = celery_cfg::get_worker_threads();
    init_semaphore(cpus).with_context(|| "failed to init semaphore")?;
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
            celery_cfg::set_config(celery_cfg::CeleryConfig {
                prefetch_count,
                acks_late,
                task_max_retries,
                broker_connection_max_retries,
                heartbeat_secs: heartbeat,
            });

            let celery_app = celery_cfg::get_celery_app().await;
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
            celery_cfg::set_queues(queues.clone());
            celery_cfg::set_is_app_active(true);
            celery_app.consume_from(&vec_str[..]).await?;
            celery_cfg::set_is_app_active(false);
            celery_app.close().await?;
        }
        CeleryOpt::Produce => {
            let celery_app = celery_cfg::get_celery_app().await;
            event!(Level::INFO, "No new tasks to produce");
            celery_app.close().await?;
        }
    };
    Ok(())
}
