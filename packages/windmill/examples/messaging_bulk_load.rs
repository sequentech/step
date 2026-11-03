// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! A saturating bulk send for the OTP capacity bench
//! (`step-dev bench messaging-otp`). Test tooling, never run against real
//! voters: notices to synthetic recipients through one account, sent by
//! windmill's own dispatch path (the ledger, the shared bucket above the
//! OTP reserve, the provider adapter), from concurrent workers. Point the
//! deployment's endpoint variable (`MESSAGING_META_GRAPH_URL` or
//! `MESSAGING_INFOBIP_URL`) at the loopback provider first.
//!
//! ```text
//! cargo run -p windmill --example messaging_bulk_load -- \
//!     --tenant-id <uuid> --account-id <uuid> --run <id> \
//!     [--workers 16] [--messages 100000] [--seconds 60]
//! ```
//!
//! Ledger rows get the logical key `bench-bulk:<run>:<n>`, so the bench can
//! delete them afterwards. Prints one JSON line per second with the counts
//! so far, and a last one with `"done": true`.

use anyhow::{anyhow, bail, Result};
use sequent_core::types::messaging::{
    MessageAttemptState, MessageChannel, MessageContent, MessagePurpose,
};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use uuid::Uuid;
use windmill::postgres::messaging::get_messaging_account;
use windmill::services::database::get_hasura_pool;
use windmill::services::messaging::dispatch::{
    deliver, Delivery, Dispatcher, ProviderTemplate, Recipient,
};

#[derive(Default)]
struct Counts {
    sent: AtomicU64,
    accepted: AtomicU64,
    failed: AtomicU64,
    other: AtomicU64,
    errors: AtomicU64,
}

impl Counts {
    fn line(&self, started: Instant, done: bool) -> String {
        format!(
            "{{\"seconds\":{:.3},\"sent\":{},\"accepted\":{},\"failed\":{},\"other\":{},\"errors\":{},\"done\":{done}}}",
            started.elapsed().as_secs_f64(),
            self.sent.load(Ordering::SeqCst),
            self.accepted.load(Ordering::SeqCst),
            self.failed.load(Ordering::SeqCst),
            self.other.load(Ordering::SeqCst),
            self.errors.load(Ordering::SeqCst),
        )
    }
}

struct Options {
    tenant_id: Uuid,
    account_id: Uuid,
    run: String,
    workers: usize,
    messages: u64,
    seconds: u64,
}

fn options() -> Result<Options> {
    let mut tenant_id = None;
    let mut account_id = None;
    let mut run = None;
    let mut workers = 16;
    let mut messages = 100_000;
    let mut seconds = 60;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        let mut value = || args.next().ok_or_else(|| anyhow!("{arg} needs a value"));
        match arg.as_str() {
            "--tenant-id" => tenant_id = Some(Uuid::parse_str(&value()?)?),
            "--account-id" => account_id = Some(Uuid::parse_str(&value()?)?),
            "--run" => run = Some(value()?),
            "--workers" => workers = value()?.parse()?,
            "--messages" => messages = value()?.parse()?,
            "--seconds" => seconds = value()?.parse()?,
            other => bail!("unknown option {other}"),
        }
    }
    let run = run.ok_or_else(|| anyhow!("--run is required"))?;
    if !run.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        bail!("--run: letters, digits and '-' only");
    }
    Ok(Options {
        tenant_id: tenant_id.ok_or_else(|| anyhow!("--tenant-id is required"))?,
        account_id: account_id.ok_or_else(|| anyhow!("--account-id is required"))?,
        run,
        workers: workers.max(1),
        messages,
        seconds,
    })
}

/// A synthetic destination of `channel`: never a real voter's.
fn destination(channel: MessageChannel, n: u64) -> Result<String> {
    Ok(match channel {
        MessageChannel::WHATSAPP | MessageChannel::VIBER | MessageChannel::SMS => {
            format!("+63917{:07}", n % 10_000_000)
        }
        MessageChannel::EMAIL => format!("bench-{n}@example.org"),
        MessageChannel::MESSENGER => {
            bail!("Messenger needs an open conversation per recipient: bench another channel")
        }
    })
}

#[tokio::main]
async fn main() -> Result<()> {
    let options = Arc::new(options()?);
    let pool = get_hasura_pool().await;
    let channel = {
        let mut client = pool.get().await?;
        let tx = client.transaction().await?;
        let account = get_messaging_account(&tx, &options.tenant_id, &options.account_id)
            .await?
            .ok_or_else(|| anyhow!("no such account in the tenant"))?;
        tx.commit().await?;
        account.channel
    };
    destination(channel, 0)?;
    let dispatcher = Dispatcher::global().await?;
    let counts = Arc::new(Counts::default());
    let next = Arc::new(AtomicU64::new(0));
    let started = Instant::now();
    let until = started + Duration::from_secs(options.seconds);
    let mut workers = Vec::new();
    for _ in 0..options.workers {
        let pool = pool.clone();
        let options = options.clone();
        let counts = counts.clone();
        let next = next.clone();
        workers.push(tokio::spawn(async move {
            let mut client = match pool.get().await {
                Ok(client) => client,
                Err(error) => {
                    eprintln!("no database connection: {error}");
                    counts.errors.fetch_add(1, Ordering::SeqCst);
                    return;
                }
            };
            loop {
                let n = next.fetch_add(1, Ordering::SeqCst);
                if n >= options.messages || Instant::now() >= until {
                    return;
                }
                let delivery = Delivery {
                    tenant_id: options.tenant_id,
                    election_event_id: None,
                    purpose: MessagePurpose::NOTICE,
                    recipient: Recipient {
                        voter_id: None,
                        channel,
                        destination: destination(channel, n).unwrap_or_default(),
                        election_ids: vec![],
                        messenger_page: None,
                    },
                    content: Some(MessageContent {
                        subject: Some("Bench".to_string()),
                        text: "Bench notice".to_string(),
                        html: None,
                        template_parameters: vec![],
                        code: None,
                    }),
                    language: Some("en".to_string()),
                    template_alias: None,
                    logical_key: format!("bench-bulk:{}:{n}", options.run),
                    expires_at: None,
                    account_id: Some(options.account_id),
                    template_key: None,
                    provider_template: Some(ProviderTemplate {
                        name: "bench_notice".to_string(),
                        language: Some("en".to_string()),
                    }),
                };
                counts.sent.fetch_add(1, Ordering::SeqCst);
                match deliver(&mut client, dispatcher, &delivery).await {
                    Ok(result) => match result.state {
                        MessageAttemptState::ACCEPTED | MessageAttemptState::DELIVERED => {
                            counts.accepted.fetch_add(1, Ordering::SeqCst)
                        }
                        MessageAttemptState::FAILED => counts.failed.fetch_add(1, Ordering::SeqCst),
                        _ => counts.other.fetch_add(1, Ordering::SeqCst),
                    },
                    Err(error) => {
                        eprintln!("send failed: {error:#}");
                        counts.errors.fetch_add(1, Ordering::SeqCst)
                    }
                };
            }
        }));
    }
    let reporter = {
        let counts = counts.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(1)).await;
                println!("{}", counts.line(started, false));
            }
        })
    };
    for worker in workers {
        worker.await?;
    }
    reporter.abort();
    println!("{}", counts.line(started, true));
    Ok(())
}
