// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Keep the original initializer's unrestricted filter contract in its own
//! process, since initialization installs both a global subscriber and logger.
#![cfg(feature = "log")]

use sequent_core::util::init_log::init_log;
use tracing::Level;
use tracing_subscriber::filter::LevelFilter;

#[test]
fn default_logging_keeps_enabled_events_and_supports_level_reload() {
    std::env::set_var("LOG_LEVEL", "info");
    let handle = init_log(true);
    assert!(tracing::enabled!(Level::INFO));
    assert!(!tracing::enabled!(Level::DEBUG));
    handle.reload(LevelFilter::DEBUG).unwrap();
    assert!(tracing::enabled!(Level::DEBUG));
    handle.reload(LevelFilter::ERROR).unwrap();
    assert!(!tracing::enabled!(Level::WARN));
    assert!(tracing::enabled!(Level::ERROR));
}
