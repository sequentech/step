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
    /// A confidential client in that tenant's realm. Its *service
    /// account* carries the `admin-user` Hasura role for the default
    /// `client_credentials` login — see `resolve_admin_login_mode`.
    pub keycloak_client_id: String,
    pub keycloak_client_secret: String,
    /// A human username to log in with `grant_type=password` instead of
    /// `client_credentials` — set together with `password`. Needed on
    /// realms where a privileged action requires step-up ("gold" ACR): a
    /// service account's client_credentials grant never runs an
    /// interactive authentication flow, so it can never reach that,
    /// regardless of which roles it holds.
    pub username: Option<String>,
    pub password: Option<String>,
}

/// Which OAuth grant the admin identity logs in with. Password grant when
/// both `username` and `password` are configured; otherwise the client's
/// own service account via `client_credentials` — the original default,
/// still right for realms with no step-up requirement.
#[derive(Debug, PartialEq, Eq)]
enum AdminLoginMode {
    Password,
    ClientCredentials,
}

fn resolve_admin_login_mode(admin: &AdminAuth) -> AdminLoginMode {
    match (&admin.username, &admin.password) {
        (Some(_), Some(_)) => AdminLoginMode::Password,
        _ => AdminLoginMode::ClientCredentials,
    }
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
    /// An existing tenant to provision every election event into, skipping
    /// tenant creation entirely. When set, every `tenants[]` entry in
    /// layers.yaml imports into this same tenant — see
    /// `resolved_tenant`.
    pub target_tenant_id: Option<String>,
}

pub async fn run(
    layers: Layers,
    template: serde_json::Value,
    options: RunOptions,
) -> Result<RunReport> {
    let http = reqwest::Client::new();

    let admin_realm = format!("tenant-{}", options.admin.tenant_id);
    let admin_token = match resolve_admin_login_mode(&options.admin) {
        AdminLoginMode::Password => {
            // Presence checked by resolve_admin_login_mode.
            let username = options.admin.username.as_deref().unwrap();
            let password = options.admin.password.as_deref().unwrap();
            crate::auth::login(
                &http,
                &options.keycloak_url,
                &admin_realm,
                &options.admin.keycloak_client_id,
                Some(&options.admin.keycloak_client_secret),
                username,
                password,
            )
            .await
        }
        AdminLoginMode::ClientCredentials => {
            crate::auth::login_client_credentials(
                &http,
                &options.keycloak_url,
                &admin_realm,
                &options.admin.keycloak_client_id,
                &options.admin.keycloak_client_secret,
            )
            .await
        }
    }
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
        let target_tenant_id = options.target_tenant_id.clone();

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
                target_tenant_id,
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

/// Either the tenant to reuse (`target_tenant_id` was set — no network
/// call, `slug` becomes just the report label) or `None`, meaning the
/// caller must create a fresh tenant itself.
fn resolved_tenant(target_tenant_id: Option<&str>, slug: &str) -> Option<provision::CreatedTenant> {
    target_tenant_id.map(|id| provision::CreatedTenant {
        id: id.to_string(),
        slug: slug.to_string(),
    })
}

/// Creates one tenant (or reuses `target_tenant_id`, if set), then runs
/// every one of its election events concurrently against it. A
/// tenant-creation failure fails every election event configured for it,
/// since none of them have anywhere to import into.
async fn run_tenant(
    http: reqwest::Client,
    endpoint_url: String,
    admin_bearer_token: String,
    keycloak_url: String,
    tenant_layer: TenantLayer,
    template_bytes: Vec<u8>,
    target_tenant_id: Option<String>,
) -> Vec<Result<ElectionEventReport>> {
    let admin_client = HasuraClient::new(http.clone(), endpoint_url.clone(), admin_bearer_token);

    let created_tenant = match resolved_tenant(target_tenant_id.as_deref(), &tenant_layer.slug) {
        Some(tenant) => tenant,
        None => match provision::create_tenant(&admin_client, &tenant_layer.slug).await {
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
        },
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolved_tenant_reuses_the_target_id_with_the_layer_slug_as_label() {
        let tenant = resolved_tenant(Some("90505c8a-23a9-4cdf-a26b-4e19f6a097d5"), "loadtest-a")
            .expect("a target tenant id should resolve to a reused tenant");
        assert_eq!(tenant.id, "90505c8a-23a9-4cdf-a26b-4e19f6a097d5");
        assert_eq!(tenant.slug, "loadtest-a");
    }

    #[test]
    fn resolved_tenant_is_none_when_no_target_id_is_given() {
        assert!(resolved_tenant(None, "loadtest-a").is_none());
    }

    fn admin_auth(username: Option<&str>, password: Option<&str>) -> AdminAuth {
        AdminAuth {
            tenant_id: "90505c8a-23a9-4cdf-a26b-4e19f6a097d5".to_string(),
            keycloak_client_id: "api-key-client".to_string(),
            keycloak_client_secret: "secret".to_string(),
            username: username.map(str::to_string),
            password: password.map(str::to_string),
        }
    }

    #[test]
    fn admin_login_mode_is_password_when_both_username_and_password_are_set() {
        let admin = admin_auth(Some("api-user"), Some("anything"));
        assert_eq!(resolve_admin_login_mode(&admin), AdminLoginMode::Password);
    }

    #[test]
    fn admin_login_mode_is_client_credentials_when_neither_is_set() {
        let admin = admin_auth(None, None);
        assert_eq!(
            resolve_admin_login_mode(&admin),
            AdminLoginMode::ClientCredentials
        );
    }

    #[test]
    fn admin_login_mode_falls_back_to_client_credentials_when_only_one_is_set() {
        assert_eq!(
            resolve_admin_login_mode(&admin_auth(Some("api-user"), None)),
            AdminLoginMode::ClientCredentials
        );
        assert_eq!(
            resolve_admin_login_mode(&admin_auth(None, Some("anything"))),
            AdminLoginMode::ClientCredentials
        );
    }
}
