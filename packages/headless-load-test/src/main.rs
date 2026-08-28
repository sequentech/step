// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

mod auth;
mod concurrency;
mod config;
mod hasura;
mod provision;
mod report;
mod run;
mod types;
mod vote;

use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

#[derive(Parser)]
#[command(
    name = "headless-load-test",
    version,
    about = "Provisions election events across tenants and casts votes against them directly over the network"
)]
struct Cli {
    /// Path to the layers.yaml file describing tenants, election events, and
    /// vote load per event
    #[arg(long)]
    layers_file: PathBuf,

    /// Path to the election-event.json template imported into every
    /// synthetic election event
    #[arg(long)]
    election_event_template: PathBuf,

    /// Hasura GraphQL endpoint on the target environment
    #[arg(long, env = "HEADLESS_LOAD_TEST_ENDPOINT_URL")]
    endpoint_url: String,

    /// Base Keycloak URL. Per-tenant, per-event realms
    /// (tenant-{t}-event-{e}) are resolved under this for both the admin
    /// login and every voter login
    #[arg(long, env = "HEADLESS_LOAD_TEST_KEYCLOAK_URL")]
    keycloak_url: String,

    /// Existing tenant whose realm the admin identity authenticates
    /// against — this identity needs cross-tenant permission to create
    /// tenants, import, publish, and open voting. Not one of the tenants
    /// being created
    #[arg(long, env = "HEADLESS_LOAD_TEST_ADMIN_TENANT_ID")]
    admin_tenant_id: String,

    /// A confidential OIDC client in the admin tenant's realm whose
    /// service account carries the `admin-user` Hasura role. Authenticates
    /// via client_credentials, not a human login — see the Architecture doc
    #[arg(long, env = "HEADLESS_LOAD_TEST_ADMIN_KEYCLOAK_CLIENT_ID")]
    admin_keycloak_client_id: String,

    /// That client's secret. Prefer the environment variable over the flag
    /// — it keeps the secret out of shell history and `ps`
    #[arg(long, env = "HEADLESS_LOAD_TEST_ADMIN_KEYCLOAK_CLIENT_SECRET")]
    admin_keycloak_client_secret: String,

    /// A human username to log the admin identity in with
    /// `grant_type=password` instead of `client_credentials`. Set together
    /// with --admin-keycloak-password. Needed on target realms where a
    /// privileged action requires step-up ("gold" ACR/LoA) — a service
    /// account's client_credentials grant never goes through an
    /// interactive authentication flow, so it can never satisfy that,
    /// regardless of which roles it holds
    #[arg(
        long,
        env = "HEADLESS_LOAD_TEST_ADMIN_KEYCLOAK_USERNAME",
        requires = "admin_keycloak_password"
    )]
    admin_keycloak_username: Option<String>,

    /// Password for --admin-keycloak-username's password-grant login
    #[arg(
        long,
        env = "HEADLESS_LOAD_TEST_ADMIN_KEYCLOAK_PASSWORD",
        requires = "admin_keycloak_username"
    )]
    admin_keycloak_password: Option<String>,

    /// Caps how many tenants are provisioned and run concurrently. Default:
    /// all tenants in layers.yaml at once
    #[arg(long)]
    max_concurrent_tenants: Option<usize>,

    /// An existing tenant to provision every election event into, instead of
    /// creating a fresh tenant per `tenants[].slug`. When set, every
    /// `tenants[]` entry in layers.yaml imports into this same tenant and
    /// `slug` is used only as a report label. Omit to create a new tenant
    /// per entry (the default)
    #[arg(long, env = "HEADLESS_LOAD_TEST_TARGET_TENANT_ID")]
    target_tenant_id: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let layers = config::load_layers(&cli.layers_file)?;
    let template = config::load_election_event_template(&cli.election_event_template)?;

    println!(
        "Loaded {} tenant(s) from {}",
        layers.tenants.len(),
        cli.layers_file.display()
    );
    println!(
        "Loaded election event template from {}",
        cli.election_event_template.display()
    );

    let options = run::RunOptions {
        endpoint_url: cli.endpoint_url,
        keycloak_url: cli.keycloak_url,
        admin: run::AdminAuth {
            tenant_id: cli.admin_tenant_id,
            keycloak_client_id: cli.admin_keycloak_client_id,
            keycloak_client_secret: cli.admin_keycloak_client_secret,
            username: cli.admin_keycloak_username,
            password: cli.admin_keycloak_password,
        },
        max_concurrent_tenants: cli.max_concurrent_tenants,
        target_tenant_id: cli.target_tenant_id,
    };

    let report = run::run(layers, template, options).await?;

    println!("{report}");

    std::process::exit(report.exit_code());
}
