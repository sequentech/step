#![allow(non_upper_case_globals)]
#![feature(result_flattening)]
#![recursion_limit = "256"]

use anyhow::Result;
use tracing_flame::FlameLayer;
use windmill::{services::{celery_app::get_worker_threads, tasks_semaphore::init_semaphore}, tasks::execute_tally_session::transactions_wrapper};
use windmill::services::probe::{setup_probe, AppName};

use std::{str::FromStr, time::{Instant, UNIX_EPOCH}};
use tracing::{event, info, instrument, warn, Level};
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::reload::Handle;
use tracing_subscriber::{filter, reload};
use tracing_subscriber::{layer::SubscriberExt, registry::Registry};
use tracing_tree::HierarchicalLayer;

#[tokio::main]
async fn main() -> Result<()> {
    let tenant_id = "90505c8a-23a9-4cdf-a26b-4e19f6a097d5".to_string();
    let election_event_id = "8d112040-cc69-49ae-a09b-98ac7240c603".to_string();
    let tally_type = "ELECTORAL_RESULTS".to_string();
    let election_ids = ["ace383b4-6823-4257-a1b7-763742867a5d"].iter().map(|x| x.to_string()).collect();

    let tally_session_id = std::env::var("TALLY_ID").unwrap();

    // DHAT

    #[global_allocator]
    static ALLOC: dhat::Alloc = dhat::Alloc;

    let _profiler = dhat::Profiler::new_heap();

    // TRACING START
    
    let layer = HierarchicalLayer::default()
    .with_writer(std::io::stdout)
    .with_ansi(false)
    .with_indent_lines(true)
    .with_indent_amount(3)
    .with_thread_names(false)
    .with_thread_ids(true)
    .with_verbose_exit(true)
    .with_verbose_entry(false)
    .with_targets(false);


    let current_time = std::time::SystemTime::now().duration_since(UNIX_EPOCH)
        .expect("Time went backwards").as_secs();
    let filename = format!("/workspaces/step/tracing-tally-{tally_session_id}.{current_time:?}.folded");
    let (flame_layer, flame_guard) = FlameLayer::with_file(filename).unwrap();

    let level_str = std::env::var("LOG_LEVEL").unwrap_or("info".to_string());
    let level = Level::from_str(&level_str).unwrap();
    let filter = filter::LevelFilter::from_level(level);
    let (filter, reload_handle) = reload::Layer::new(filter);


    let subscriber = Registry::default().with(filter).with(layer).with(flame_layer);
    tracing::subscriber::set_global_default(subscriber).unwrap();

    // TRACING END

    setup_probe(AppName::WINDMILL).await;

    let cpus = get_worker_threads();
    init_semaphore(cpus)?;

    let current = Instant::now();
    info!("Start time: {current:?}");

    transactions_wrapper(tenant_id, election_event_id, tally_session_id, Some(tally_type), Some(election_ids)).await?;

    let current = Instant::now();
    info!("Finish time: {current:?}");

    flame_guard.flush()?;

    Ok(())
}
