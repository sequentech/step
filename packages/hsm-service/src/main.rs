// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use hsm_service::audit::{self, AuditLog};
use hsm_service::config::{read_pin, Config};
use hsm_service::officials::StepUpVerifier;
use hsm_service::pkcs11::Pkcs11Provider;
use hsm_service::policy::Policy;
use hsm_service::registry::Registry;
use hsm_service::service::Service;
use hsm_service::{server, tls};

#[derive(Parser)]
#[command(about = "The platform's central HSM/KMS key service")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Serve the API (settings from HSM_SERVICE_* variables).
    Serve,
    /// Check an audit log's hash chain.
    VerifyAudit { path: PathBuf },
    /// Check a policy file.
    CheckPolicy { path: PathBuf },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    match Cli::parse().command {
        Command::VerifyAudit { path } => {
            let (seq, head) = audit::verify(&path)?;
            println!("audit log intact: {seq} records, head {head}");
            Ok(())
        }
        Command::CheckPolicy { path } => {
            let policy = Policy::parse(&std::fs::read_to_string(&path)?)?;
            println!(
                "policy valid: {} callers, {} key rules",
                policy.callers.len(),
                policy.keys.len()
            );
            Ok(())
        }
        Command::Serve => serve().await,
    }
}

async fn serve() -> Result<()> {
    let config = Config::from_env()?;
    let policy =
        Policy::parse(&std::fs::read_to_string(&config.policy).context("reading the policy")?)?;
    let pin = read_pin(&config.pkcs11.pin_file)?;
    let provider = Pkcs11Provider::open(
        config.pkcs11.backend,
        &config.pkcs11.module,
        &config.pkcs11.token_label,
        pin,
    )?;
    let info = hsm_service::provider::Provider::info(&provider);
    tracing::info!(
        backend = %info.backend, hardware = info.hardware, model = %info.token_model,
        deployment = ?config.deployment, "key provider ready"
    );
    std::fs::create_dir_all(&config.state_dir)?;
    let registry = Registry::open(&config.state_dir.join("keys.json"))?;
    let audit = AuditLog::open(&config.state_dir.join("audit.jsonl"))?;
    let officials = policy.officials.clone().map(StepUpVerifier::new);
    let service = Arc::new(Service::new(
        policy,
        Arc::new(provider),
        registry,
        audit,
        officials,
    ));
    let tls = tls::server_config(&config.tls_cert, &config.tls_key, &config.client_ca)?;
    let listener = tokio::net::TcpListener::bind(&config.listen).await?;
    tracing::info!(listen = %config.listen, "serving");
    server::serve(listener, tls, service).await
}
