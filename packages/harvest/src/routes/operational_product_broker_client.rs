// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The ordinary approval authority talks only to the independently pinned
//! production broker. No browser request is passed through; the exact
//! messages here come only from the checked server signing plan.

use super::operational_product_approval::{
    production_https, ProductApprovalBroker, ProductApprovalScope,
    ProductBrokerFailure, ProductBrokerStep, ProductRedirect,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use chrono::{DateTime, Utc};
use reqwest::{Certificate, Client, Identity, StatusCode, Url};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use uuid::Uuid;
use windmill::services::signing::operational_approval::{
    OperationalApprovalPlan, OperationalSigningMaterial,
};

const API: &str = "/private/v2/operational-signing/";
const MAX_RESPONSE: usize = 8192;
const MAX_SIGNING_MESSAGE: usize = 16 * 1024 * 1024;

pub(super) struct PrivateProductBrokerClient {
    client: Client,
    base: Url,
}

impl PrivateProductBrokerClient {
    pub(super) fn new(
        base: Url,
        trusted_root_pem: &[u8],
        client_pem: &[u8],
    ) -> Option<Self> {
        // Enrollment fixtures have their own separately scoped authority.
        if !production_https(&base)
            || base.query().is_some()
            || base.path() != "/"
        {
            return None;
        }
        // Do not use machine-wide trust roots, arbitrary outbound proxies, or redirects.
        let roots = Certificate::from_pem_bundle(trusted_root_pem).ok()?;
        if roots.is_empty() || roots.len() > 8 {
            return None;
        }
        let mut builder = Client::builder()
            .use_rustls_tls()
            .tls_built_in_root_certs(false)
            .identity(Identity::from_pem(client_pem).ok()?)
            .https_only(true)
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(Duration::from_secs(4))
            .timeout(Duration::from_secs(30));
        for root in roots {
            builder = builder.add_root_certificate(root);
        }
        Some(Self {
            client: builder.build().ok()?,
            base,
        })
    }

    async fn post(
        &self,
        operation: &str,
        body: &impl Serialize,
        expected: &ProductApprovalScope,
    ) -> Result<PrivateReply, ProductBrokerFailure> {
        let mut endpoint = self.base.clone();
        endpoint.set_path(&format!("{API}{operation}"));
        // Any lost/truncated reply can follow an already accepted external
        // operation. Never repeat the device command based on transport.
        let mut response = self
            .client
            .post(endpoint)
            .json(body)
            .send()
            .await
            .map_err(|_| ProductBrokerFailure::Reconcile)?;
        match response.status() {
            StatusCode::OK => (),
            StatusCode::BAD_REQUEST
            | StatusCode::UNAUTHORIZED
            | StatusCode::FORBIDDEN
            | StatusCode::NOT_FOUND
            | StatusCode::UNPROCESSABLE_ENTITY => {
                return Err(ProductBrokerFailure::Refused)
            }
            _ => return Err(ProductBrokerFailure::Reconcile),
        }
        if response
            .content_length()
            .is_some_and(|length| length > MAX_RESPONSE as u64)
        {
            return Err(ProductBrokerFailure::Reconcile);
        }
        let mut body = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| ProductBrokerFailure::Reconcile)?
        {
            if chunk.len() > MAX_RESPONSE - body.len() {
                return Err(ProductBrokerFailure::Reconcile);
            }
            body.extend_from_slice(&chunk);
        }
        let accepted: PrivateReply = serde_json::from_slice(&body)
            .map_err(|_| ProductBrokerFailure::Reconcile)?;
        if accepted.scope() != expected {
            return Err(ProductBrokerFailure::Reconcile);
        }
        Ok(accepted)
    }
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct Part<'a> {
    purpose: &'a str,
    key_id: &'a str,
    key_version: i32,
    cert_fingerprint: &'a str,
    cert_spki: &'a str,
    payload_sha256: &'a str,
    message_sha256: &'a str,
    artifact_sha256: &'a str,
    pdf_revision: Option<i32>,
    pdf_parent_sha256: Option<&'a str>,
    pdf_byte_range_sha256: Option<&'a str>,
    request_expires_at: Option<DateTime<Utc>>,
    exact_message_base64: String,
}

impl<'a> Part<'a> {
    fn from_plan(
        plan: &'a OperationalApprovalPlan,
        material: &'a OperationalSigningMaterial,
    ) -> Result<Self, ProductBrokerFailure> {
        let message = material.exact_message();
        if message.is_empty() || message.len() > MAX_SIGNING_MESSAGE {
            return Err(ProductBrokerFailure::Refused);
        }
        Ok(Self {
            purpose: material.purpose.as_str(),
            key_id: &plan.key_id,
            key_version: plan.key_version,
            cert_fingerprint: &plan.cert_fingerprint,
            cert_spki: &plan.cert_spki,
            payload_sha256: &plan.payload.message_sha256,
            message_sha256: &material.message_sha256,
            artifact_sha256: &material.artifact_sha256,
            pdf_revision: material.pdf_revision,
            pdf_parent_sha256: material.pdf_parent_sha256.as_deref(),
            pdf_byte_range_sha256: material.pdf_byte_range_sha256.as_deref(),
            request_expires_at: plan.request_expires_at,
            exact_message_base64: STANDARD.encode(message),
        })
    }
}

#[derive(Serialize)]
struct Begin<'a> {
    scope: &'a ProductApprovalScope,
    parts: Vec<Part<'a>>,
}

#[derive(Serialize)]
struct Finish<'a> {
    scope: &'a ProductApprovalScope,
    attempt: Uuid,
    state: &'a str,
    code: &'a str,
}

#[derive(Deserialize)]
#[serde(tag = "step", rename_all = "kebab-case", deny_unknown_fields)]
enum PrivateReply {
    Redirect {
        scope: ProductApprovalScope,
        attempt: Uuid,
        url: String,
    },
    Completed {
        scope: ProductApprovalScope,
        payload_challenge: Uuid,
        artifact_challenge: Option<Uuid>,
    },
    Reconcile {
        scope: ProductApprovalScope,
    },
    Ready {
        scope: ProductApprovalScope,
    },
}

impl PrivateReply {
    fn scope(&self) -> &ProductApprovalScope {
        match self {
            Self::Redirect { scope, .. }
            | Self::Completed { scope, .. }
            | Self::Reconcile { scope }
            | Self::Ready { scope } => scope,
        }
    }
}

fn redirect(
    attempt: Uuid,
    url: &str,
) -> Result<ProductRedirect, ProductBrokerFailure> {
    if attempt.is_nil() || url.len() > 2048 {
        return Err(ProductBrokerFailure::Reconcile);
    }
    let authorization_url =
        Url::parse(url).map_err(|_| ProductBrokerFailure::Reconcile)?;
    Ok(ProductRedirect {
        attempt,
        authorization_url,
    })
}

#[async_trait::async_trait]
impl ProductApprovalBroker for PrivateProductBrokerClient {
    async fn begin(
        &self,
        scope: &ProductApprovalScope,
        plan: OperationalApprovalPlan,
    ) -> Result<ProductRedirect, ProductBrokerFailure> {
        if scope.tenant != plan.tenant_id
            || scope.event != plan.event_id
            || scope.request != plan.request_id
            || scope.actor != plan.actor
            || scope.action != plan.action
        {
            return Err(ProductBrokerFailure::Refused);
        }
        let mut parts = vec![Part::from_plan(&plan, &plan.payload)?];
        if let Some(part) = &plan.artifact {
            parts.push(Part::from_plan(&plan, part)?);
        }
        match self.post("begin", &Begin { scope, parts }, scope).await? {
            PrivateReply::Redirect { attempt, url, .. } => {
                redirect(attempt, &url)
            }
            _ => Err(ProductBrokerFailure::Reconcile),
        }
    }

    async fn finish(
        &self,
        scope: &ProductApprovalScope,
        attempt: Uuid,
        state: &str,
        code: &str,
    ) -> Result<ProductBrokerStep, ProductBrokerFailure> {
        let body = Finish {
            scope,
            attempt,
            state,
            code,
        };
        match self.post("finish", &body, scope).await? {
            PrivateReply::Redirect {
                attempt: next, url, ..
            } if next != attempt => {
                Ok(ProductBrokerStep::Next(redirect(next, &url)?))
            }
            PrivateReply::Completed {
                scope,
                payload_challenge,
                artifact_challenge,
            } => Ok(ProductBrokerStep::Complete {
                scope,
                payload_challenge,
                artifact_challenge,
            }),
            PrivateReply::Redirect { .. }
            | PrivateReply::Reconcile { .. }
            | PrivateReply::Ready { .. } => Ok(ProductBrokerStep::Reconcile),
        }
    }

    async fn recover(
        &self,
        scope: &ProductApprovalScope,
    ) -> Result<ProductBrokerStep, ProductBrokerFailure> {
        #[derive(Serialize)]
        struct Recovery<'a> {
            scope: &'a ProductApprovalScope,
        }
        match self.post("reconcile", &Recovery { scope }, scope).await? {
            PrivateReply::Completed {
                scope,
                payload_challenge,
                artifact_challenge,
            } => Ok(ProductBrokerStep::Complete {
                scope,
                payload_challenge,
                artifact_challenge,
            }),
            PrivateReply::Ready { .. } => Ok(ProductBrokerStep::Ready),
            PrivateReply::Redirect { .. } | PrivateReply::Reconcile { .. } => {
                Ok(ProductBrokerStep::Reconcile)
            }
        }
    }
}
