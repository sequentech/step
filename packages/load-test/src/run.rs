// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! End-to-end wiring: admin login once, then one tokio task per tenant
//! (each internally running its election events concurrently), each doing
//! Phase 1 provisioning followed by Phase 2's rate-limited voting, folded
//! into a `RunReport`.

use std::sync::Arc;
use std::time::Instant;

use anyhow::{Context, Result};
use tokio::sync::Semaphore;
use tokio::task::JoinSet;

use crate::concurrency::run_rate_limited;
use crate::config::{ElectionEventLayer, Layers, TenantLayer};
use crate::hasura::HasuraClient;
use crate::provision;
use crate::report::{ElectionEventReport, RunReport};
use crate::vote;

pub struct AdminAuth {
    /// The tenant whose realm the admin identity authenticates against —
    /// an existing tenant with cross-tenant permission to create the
    /// tenants `layers.yaml` describes, not one of those tenants itself.
    pub tenant_id: String,
    /// A confidential client in that tenant's realm whose *service
    /// account* (not a human user) carries the `admin-user` Hasura role —
    /// `TENANT_CREATE` and friends aren't necessarily held by any
    /// password-grant-capable user, but every confidential client gets a
    /// service account for free, authenticated via
    /// `grant_type=client_credentials`.
    pub keycloak_client_id: String,
    pub keycloak_client_secret: String,
}

pub struct RunOptions {
    pub endpoint_url: String,
    pub keycloak_url: String,
    pub admin: AdminAuth,
    /// Caps how many tenants are provisioned and voted concurrently.
    /// Defaults to all of them at once. Election events *within* an
    /// already-provisioning tenant are not further throttled by this —
    /// see the module doc comment.
    pub max_concurrent_tenants: Option<usize>,
}

pub async fn run(
    layers: Layers,
    template: serde_json::Value,
    options: RunOptions,
) -> Result<RunReport> {
    let http = reqwest::Client::new();

    let admin_realm = format!("tenant-{}", options.admin.tenant_id);
    let admin_token = crate::auth::login_client_credentials(
        &http,
        &options.keycloak_url,
        &admin_realm,
        &options.admin.keycloak_client_id,
        &options.admin.keycloak_client_secret,
    )
    .await
    .map_err(|err| anyhow::anyhow!("admin login failed: {err}"))?;

    let template_bytes =
        serde_json::to_vec(&template).context("failed to serialize the election-event template")?;

    let tenant_count = layers.tenants.len().max(1);
    let semaphore = Arc::new(Semaphore::new(
        options.max_concurrent_tenants.unwrap_or(tenant_count),
    ));

    let mut tenant_tasks = JoinSet::new();
    for tenant_layer in layers.tenants {
        let semaphore = semaphore.clone();
        let http = http.clone();
        let endpoint_url = options.endpoint_url.clone();
        let keycloak_url = options.keycloak_url.clone();
        let admin_bearer_token = admin_token.access_token.clone();
        let template_bytes = template_bytes.clone();

        tenant_tasks.spawn(async move {
            let _permit = semaphore
                .acquire_owned()
                .await
                .expect("semaphore is never closed");
            run_tenant(
                http,
                endpoint_url,
                admin_bearer_token,
                keycloak_url,
                tenant_layer,
                template_bytes,
            )
            .await
        });
    }

    let mut run_report = RunReport::default();
    while let Some(joined) = tenant_tasks.join_next().await {
        match joined {
            Ok(results) => {
                for result in results {
                    match result {
                        Ok(report) => run_report.election_events.push(report),
                        Err(err) => run_report.provisioning_failures.push(format!("{err:#}")),
                    }
                }
            }
            Err(join_err) => run_report
                .provisioning_failures
                .push(format!("tenant task panicked: {join_err}")),
        }
    }

    Ok(run_report)
}

/// Creates one tenant, then runs every one of its election events
/// concurrently against it. A tenant-creation failure fails every election
/// event configured for it, since none of them have anywhere to import
/// into.
async fn run_tenant(
    http: reqwest::Client,
    endpoint_url: String,
    admin_bearer_token: String,
    keycloak_url: String,
    tenant_layer: TenantLayer,
    template_bytes: Vec<u8>,
) -> Vec<Result<ElectionEventReport>> {
    let admin_client = HasuraClient::new(http.clone(), endpoint_url.clone(), admin_bearer_token);

    let created_tenant = match provision::create_tenant(&admin_client, &tenant_layer.slug).await {
        Ok(tenant) => tenant,
        Err(err) => {
            return tenant_layer
                .election_events
                .iter()
                .map(|_| {
                    Err(anyhow::anyhow!(
                        "tenant `{}` creation failed: {err:#} (tenant creation \
                         isn't idempotent — layers.yaml slugs must be unique \
                         per run)",
                        tenant_layer.slug
                    ))
                })
                .collect();
        }
    };

    let mut event_tasks = JoinSet::new();
    for event_layer in tenant_layer.election_events {
        let admin_client = admin_client.clone();
        let http = http.clone();
        let endpoint_url = endpoint_url.clone();
        let keycloak_url = keycloak_url.clone();
        let tenant_id = created_tenant.id.clone();
        let tenant_slug = created_tenant.slug.clone();
        let template_bytes = template_bytes.clone();

        event_tasks.spawn(async move {
            run_election_event(
                &admin_client,
                &http,
                &endpoint_url,
                &keycloak_url,
                &tenant_id,
                &tenant_slug,
                event_layer,
                &template_bytes,
            )
            .await
        });
    }

    let mut results = Vec::new();
    while let Some(joined) = event_tasks.join_next().await {
        match joined {
            Ok(result) => results.push(result),
            Err(join_err) => results.push(Err(anyhow::anyhow!(
                "election event task panicked: {join_err}"
            ))),
        }
    }
    results
}

/// Runs Phase 1 for one election event, then Phase 2 for the voters it
/// provisioned.
#[allow(clippy::too_many_arguments)]
async fn run_election_event(
    admin_client: &HasuraClient,
    http: &reqwest::Client,
    endpoint_url: &str,
    keycloak_url: &str,
    tenant_id: &str,
    tenant_slug: &str,
    event_layer: ElectionEventLayer,
    template_bytes: &[u8],
) -> Result<ElectionEventReport> {
    let provisioned = provision::provision_election_event(
        admin_client,
        http,
        tenant_id,
        template_bytes,
        event_layer.voters,
    )
    .await
    .with_context(|| format!("provisioning failed for tenant `{tenant_slug}`"))?;

    let election_id = provisioned.election_ids.first().cloned().ok_or_else(|| {
        anyhow::anyhow!(
            "election event {} has no elections to vote in",
            provisioned.election_event_id
        )
    })?;

    let mut report = ElectionEventReport::new(
        tenant_slug.to_string(),
        provisioned.election_event_id.clone(),
    );

    let http_owned = http.clone();
    let keycloak_url_owned = keycloak_url.to_string();
    let endpoint_url_owned = endpoint_url.to_string();
    let tenant_id_owned = tenant_id.to_string();
    let election_event_id_owned = provisioned.election_event_id.clone();
    let election_id_owned = election_id.clone();

    let outcomes = run_rate_limited(
        provisioned.voters,
        event_layer.votes_per_second,
        event_layer.duration,
        move |voter| {
            let http = http_owned.clone();
            let keycloak_url = keycloak_url_owned.clone();
            let endpoint_url = endpoint_url_owned.clone();
            let tenant_id = tenant_id_owned.clone();
            let election_event_id = election_event_id_owned.clone();
            let election_id = election_id_owned.clone();
            async move {
                let start = Instant::now();
                let outcome = vote::cast_one_vote(
                    &http,
                    &keycloak_url,
                    &endpoint_url,
                    &tenant_id,
                    &election_event_id,
                    &election_id,
                    &voter.username,
                    &voter.password,
                )
                .await;
                let elapsed = start.elapsed();
                (voter, (outcome, elapsed))
            }
        },
    )
    .await;

    for (outcome, elapsed) in outcomes {
        report.record(&outcome, elapsed);
    }

    Ok(report)
}
