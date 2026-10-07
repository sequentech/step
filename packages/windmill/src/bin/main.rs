#![allow(non_upper_case_globals)]
#![recursion_limit = "256"]
// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Context;
use anyhow::{anyhow, Result};
use celery::Celery;
use clap::Parser;
use dotenv::dotenv;
use sequent_core::util::init_log::init_log;
use std::collections::HashMap;
use std::str::FromStr;
use tokio::runtime::Builder;
use tracing::{event, Level};
use windmill::services::celery_app::{self as celery_cfg, Queue};
use windmill::services::probe::{setup_probe, AppName};
use windmill::services::task_queues::set_up_task_queue_database;
use windmill::services::tasks_semaphore::init_semaphore;

#[derive(Debug, Parser, Clone)]
#[command(name = "windmill", about = "Windmill task queue prosumer.")]
enum CeleryOpt {
    Consume {
        #[arg(short, long, num_args(1..), default_values_t = vec![Queue::Beat.queue_name().to_string()])]
        queues: Vec<String>,
        #[arg(short, long, default_value = "100")]
        prefetch_count: u16,
        #[arg(short, long, default_value_t = true)]
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
    /// Install PGMQ and create the queues in the task-queue database, as its owner.
    SetupQueueDatabase,
}

/// Resolve the queues to consume. A queue may also be given with the `ENV_SLUG_` prefix that
/// queue names had when environments shared a broker.
fn normalize_consumed_queues(slug: &str, inputs: &[String]) -> Result<Vec<String>> {
    let legacy_prefix = format!("{slug}_");
    inputs
        .iter()
        .map(|input| {
            Queue::from_str(input)
                .or_else(|_| {
                    input
                        .strip_prefix(&legacy_prefix)
                        .ok_or(strum::ParseError::VariantNotFound)
                        .and_then(Queue::from_str)
                })
                .map(|queue| queue.queue_name().to_string())
                .map_err(|_| anyhow!("Unknown queue {input}"))
        })
        .collect()
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
        CeleryOpt::Produce | CeleryOpt::SetupQueueDatabase => None,
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
    if let CeleryOpt::SetupQueueDatabase = opt {
        let installation = set_up_task_queue_database().await?;
        event!(
            Level::INFO,
            "Task-queue database set up with {} queues (PGMQ: {:?})",
            Queue::all_names().len(),
            installation
        );
        return Ok(());
    }
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
            let queues = normalize_consumed_queues(&slug, &queues_input)?;

            for (queue, purpose) in [
                (
                    Queue::ElectoralLogEvent,
                    "is read only by the electoral-log dispatcher",
                ),
                (
                    Queue::ElectoralLogDeadLetter,
                    "holds electoral-log events for inspection and replay",
                ),
            ] {
                let name = queue.queue_name();
                if queues.iter().any(|consumed| consumed == name) {
                    return Err(anyhow!(
                        "{name} {purpose}; a worker consuming it would discard its events"
                    ));
                }
            }
            let copies =
                windmill::services::electoral_log_checkpoint_copies::CheckpointCopyConfig::from_env()?;
            event!(
                Level::INFO,
                "Electoral-log checkpoint copies: {}, bucket {}, {} lock for {} days",
                copies.policy,
                copies.bucket,
                copies.lock_mode,
                copies.retention_days
            );
            if queues
                .iter()
                .any(|consumed| consumed == Queue::Beat.queue_name())
            {
                let retention = windmill::tasks::purge_queue_archives::retention()?;
                event!(
                    Level::INFO,
                    "Task-queue archives are kept for {} hours",
                    retention.as_secs() / 3600
                );
            }
            if queues
                .iter()
                .any(|consumed| consumed == Queue::ElectoralLogBeat.queue_name())
            {
                let limits = windmill::tasks::electoral_log::BatchLimits::from_env()?;
                event!(
                    Level::INFO,
                    "Electoral-log dispatcher batch limits: {} events, {} bytes",
                    limits.max_events,
                    limits.max_bytes
                );
            }

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
        CeleryOpt::SetupQueueDatabase => {}
    };
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn consumed_queues_accept_plain_and_legacy_prefixed_names() {
        let inputs = ["short_queue", "dev_beat", "dev_tally_queue"].map(String::from);
        assert_eq!(
            normalize_consumed_queues("dev", &inputs).unwrap(),
            ["short_queue", "beat", "tally_queue"]
        );
        for unknown in ["prod_short_queue", "dev_", "unknown_queue", ""] {
            assert!(
                normalize_consumed_queues("dev", &[unknown.to_string()]).is_err(),
                "{unknown}"
            );
        }
    }
}
