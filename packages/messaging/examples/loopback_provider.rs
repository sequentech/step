// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The loopback provider as a process of its own, for the OTP capacity
//! bench (`step-dev bench messaging-otp`). Test code only: it accepts every
//! send after a fixed latency, with the answer WhatsApp, Messenger and
//! Infobip each read as accepted. Point a deployment's endpoint variables
//! (`MESSAGING_META_GRAPH_URL`, `MESSAGING_INFOBIP_URL`) at it.
//!
//! ```text
//! cargo run -p messaging --features test-support --example loopback_provider -- \
//!     [--port 0] [--latency-ms 150]
//! ```
//!
//! Prints `listening <base url>` once it accepts connections, then one
//! JSON line `{"served":…,"peak_in_flight":…}` per second.

#[cfg(feature = "test-support")]
#[tokio::main]
async fn main() {
    use messaging::test_server::{accepted_by_any_provider, Reply, ServerOptions, TestServer};
    use std::io::Write;
    use std::time::Duration;

    let mut port: u16 = 0;
    let mut latency_ms: u64 = 150;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        let mut value = || args.next().expect("a value after the option");
        match arg.as_str() {
            "--port" => port = value().parse().expect("--port: a number"),
            "--latency-ms" => latency_ms = value().parse().expect("--latency-ms: a number"),
            "-h" | "--help" => {
                println!("loopback_provider [--port 0] [--latency-ms 150]");
                return;
            }
            other => panic!("unknown option {other}"),
        }
    }
    let server = TestServer::bind(
        ([127, 0, 0, 1], port).into(),
        vec![],
        ServerOptions {
            latency: Duration::from_millis(latency_ms),
            then: Some(Reply::Json(200, accepted_by_any_provider("loopback"))),
            concurrent: true,
            count_only: true,
        },
    )
    .await;
    println!("listening {}", server.base_url);
    let _ = std::io::stdout().flush();
    loop {
        tokio::time::sleep(Duration::from_secs(1)).await;
        println!(
            "{{\"served\":{},\"peak_in_flight\":{}}}",
            server.served(),
            server.peak_in_flight()
        );
        let _ = std::io::stdout().flush();
    }
}

#[cfg(not(feature = "test-support"))]
fn main() {
    eprintln!("build with --features test-support");
    std::process::exit(2);
}
