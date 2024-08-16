// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use strand::backend::ristretto::RistrettoCtx;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::reload::Handle;
use tracing_subscriber::{filter, reload};
use tracing_subscriber::{layer::SubscriberExt, registry::Registry};
use tracing_tree::HierarchicalLayer;
cfg_if::cfg_if! {
    if #[cfg(feature = "rug")] {
        use strand::backend::rug::RugCtx;
        use strand::backend::rug::P2048 as RUGP2048;
    }
}

#[test]
fn test_protocol_memory() {
    init_log(true);

    let ctx = RistrettoCtx;
    braid::test::protocol_test_memory::run(1000, 1, ctx);
    cfg_if::cfg_if! {
        if #[cfg(feature = "rug")] {
            let ctx = RugCtx::<RUGP2048>::default();
            braid::test::protocol_test_memory::run(100, 1, ctx);
        }
    }
}

#[tokio::test]
#[ignore]
async fn test_protocol_pgsql() {
    init_log(true);
    let ctx = RistrettoCtx;

    braid::test::protocol_test_pgsql::run(1000, 1, ctx).await;
    cfg_if::cfg_if! {
        if #[cfg(feature = "rug")] {
            let ctx = RugCtx::<RUGP2048>::default();
            braid::test::protocol_test_pgsql::run(100, 1, ctx).await;
        }
    }
}

#[tokio::test]
#[ignore]
async fn test_protocol_grpc() {
    init_log(true);
    let ctx = RistrettoCtx;

    braid::test::protocol_test_grpc::run(1000, 1, ctx).await;
    cfg_if::cfg_if! {
        if #[cfg(feature = "rug")] {
            let ctx = RugCtx::<RUGP2048>::default();
            braid::test::protocol_test_pgsql::run(100, 1, ctx).await;
        }
    }
}

pub fn init_log(set_global: bool) -> Handle<LevelFilter, Registry> {
    let layer = HierarchicalLayer::default()
        .with_writer(std::io::stdout)
        .with_indent_lines(true)
        .with_indent_amount(3)
        .with_thread_names(false)
        .with_thread_ids(false)
        .with_verbose_exit(false)
        .with_verbose_entry(false)
        .with_targets(false);

    let filter = filter::LevelFilter::INFO;
    let (filter, reload_handle) = reload::Layer::new(filter);
    let subscriber = Registry::default().with(filter).with(layer);

    if set_global {
        tracing::subscriber::set_global_default(subscriber).unwrap();
    }
    tracing_log::LogTracer::init().unwrap();
    reload_handle
}
