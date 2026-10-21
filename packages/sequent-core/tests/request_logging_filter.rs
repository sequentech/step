// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Observe emitted logs in a child process that owns its global subscriber.
#![cfg(feature = "log")]

use sequent_core::util::init_log::init_log_with_filter;
use std::process::Command;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use tracing::Level;
use tracing_subscriber::filter::LevelFilter;

const CHILD_MODE: &str = "CORE_LOG_FILTER_TEST_CHILD";

#[test]
fn request_filter_follows_runtime_levels_and_suppresses_events() {
    if std::env::var_os(CHILD_MODE).is_some() {
        exercise_filter();
        return;
    }
    let output = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "request_filter_follows_runtime_levels_and_suppresses_events",
            "--nocapture",
        ])
        .env(CHILD_MODE, "1")
        .env("LOG_LEVEL", "info")
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        output.status.success(),
        "{stdout}\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    for visible in [
        "FILTER_INFO_VISIBLE",
        "FILTER_REQUEST_RESTORED",
        "FILTER_DEBUG_VISIBLE",
        "FILTER_WARN_VISIBLE",
    ] {
        assert!(stdout.contains(visible), "missing {visible}: {stdout}");
    }
    for hidden in [
        "FILTER_DEBUG_HIDDEN",
        "FILTER_REQUEST_HIDDEN",
        "FILTER_INFO_HIDDEN_AFTER_RELOAD",
    ] {
        assert!(!stdout.contains(hidden), "emitted {hidden}: {stdout}");
    }
}

fn exercise_filter() {
    let allow = Arc::new(AtomicBool::new(true));
    let observed = Arc::new(Mutex::new(Vec::new()));
    let filter_allow = Arc::clone(&allow);
    let filter_observed = Arc::clone(&observed);
    let handle = init_log_with_filter(true, move |metadata, level| {
        if metadata.target() == "custody::fixture" {
            filter_observed
                .lock()
                .unwrap()
                .push((level, *metadata.level()));
            filter_allow.load(Ordering::SeqCst)
        } else {
            true
        }
    });
    tracing::info!(target: "custody::fixture", "FILTER_INFO_VISIBLE");
    tracing::debug!(target: "custody::fixture", "FILTER_DEBUG_HIDDEN");
    allow.store(false, Ordering::SeqCst);
    tracing::info!(target: "custody::fixture", "FILTER_REQUEST_HIDDEN");
    allow.store(true, Ordering::SeqCst);
    tracing::info!(target: "custody::fixture", "FILTER_REQUEST_RESTORED");
    handle.reload(LevelFilter::DEBUG).unwrap();
    tracing::debug!(target: "custody::fixture", "FILTER_DEBUG_VISIBLE");
    handle.reload(LevelFilter::WARN).unwrap();
    tracing::info!(target: "custody::fixture", "FILTER_INFO_HIDDEN_AFTER_RELOAD");
    tracing::warn!(target: "custody::fixture", "FILTER_WARN_VISIBLE");
    for pair in [
        (LevelFilter::INFO, Level::INFO),
        (LevelFilter::DEBUG, Level::DEBUG),
        (LevelFilter::WARN, Level::WARN),
    ] {
        assert!(observed.lock().unwrap().contains(&pair));
    }
}
