// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! What signing routes answer on failure (design §5, the signing API
//! contract): a JSON body `{"message", "extensions": {"code", "check"?,
//! "status"?}}` that Hasura forwards to the portal, and a 403 for a missing
//! permission. Also the display name signing rows record for the caller.

use crate::services::authorization::authorize;
use rocket::http::{ContentType, Status};
use rocket::request::Request;
use rocket::response::{self, Responder, Response};
use sequent_core::services::jwt::JwtClaims;
use sequent_core::signing::{CertificateCheckId, SigningRequestStatus};
use sequent_core::types::permissions::Permissions;
use serde_json::{json, Map, Value};
use std::collections::HashSet;
use std::io::Cursor;
use strum_macros::{Display, EnumString};
use tracing::error;
use uuid::Uuid;
use windmill::services::signing::directory::{display_name, UserDirectory};

/// `extensions.code` of a signing route's error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, EnumString)]
#[strum(serialize_all = "kebab-case")]
pub enum SigningErrorCode {
    /// 401: not signed in to this tenant.
    Unauthorized,
    /// 403: a permission or the Post is missing.
    Forbidden,
    /// 404.
    NotFound,
    /// 400: the input is not valid.
    Invalid,
    /// 409: someone saved the settings first (a stale `expected_revision`),
    /// or the row is already in the state asked for.
    Conflict,
    /// 409: the signing request is no longer waiting; `extensions.status`
    /// is its status.
    RequestClosed,
    /// 409: the prepared PDF revision is stale; prepare again.
    StaleRevision,
    /// 422: a certificate or signature check refused; `extensions.check`
    /// names it.
    SigningRefused,
    /// 422: this user, certificate, key or holder already signed.
    AlreadySigned,
    /// 500.
    Internal,
    /// 409: the election event is locked down; its signing rules can't
    /// change.
    LockedDown,
    /// 409: the change needs signatures; start the Post-level action.
    SigningRequired,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SigningError {
    pub status: Status,
    pub code: SigningErrorCode,
    pub message: String,
    pub extensions: Map<String, Value>,
}

impl SigningError {
    pub fn new(
        status: Status,
        code: SigningErrorCode,
        message: impl Into<String>,
    ) -> Self {
        SigningError {
            status,
            code,
            message: message.into(),
            extensions: Map::new(),
        }
    }

    pub fn with(mut self, key: &str, value: impl Into<Value>) -> Self {
        self.extensions.insert(key.to_owned(), value.into());
        self
    }

    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::new(Status::Forbidden, SigningErrorCode::Forbidden, message)
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(Status::NotFound, SigningErrorCode::NotFound, message)
    }

    pub fn invalid(message: impl Into<String>) -> Self {
        Self::new(Status::BadRequest, SigningErrorCode::Invalid, message)
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self::new(Status::Conflict, SigningErrorCode::Conflict, message)
    }

    pub fn request_closed(status: SigningRequestStatus) -> Self {
        Self::new(
            Status::Conflict,
            SigningErrorCode::RequestClosed,
            format!("The request is {status}"),
        )
        .with("status", status.to_string())
    }

    pub fn refused(
        check: CertificateCheckId,
        message: impl Into<String>,
    ) -> Self {
        Self::new(
            Status::UnprocessableEntity,
            SigningErrorCode::SigningRefused,
            message,
        )
        .with("check", check.to_string())
    }

    /// Logs the error; the answer says only that it failed.
    pub fn internal(error: impl std::fmt::Debug) -> Self {
        error!("Signing route failed: {error:?}");
        Self::new(
            Status::InternalServerError,
            SigningErrorCode::Internal,
            "Internal error",
        )
    }

    pub fn body(&self) -> Value {
        let mut extensions = self.extensions.clone();
        extensions.insert("code".into(), Value::String(self.code.to_string()));
        json!({"message": self.message, "extensions": extensions})
    }
}

/// A refusal from [`authorize`]: 401 for another tenant.
impl From<(Status, String)> for SigningError {
    fn from((status, message): (Status, String)) -> Self {
        let code = match status.code {
            401 => SigningErrorCode::Unauthorized,
            403 => SigningErrorCode::Forbidden,
            404 => SigningErrorCode::NotFound,
            400 => SigningErrorCode::Invalid,
            _ => SigningErrorCode::Internal,
        };
        SigningError::new(status, code, message)
    }
}

impl<'r> Responder<'r, 'static> for SigningError {
    fn respond_to(self, _: &'r Request<'_>) -> response::Result<'static> {
        let body = self.body().to_string();
        Response::build()
            .status(self.status)
            .header(ContentType::JSON)
            .sized_body(body.len(), Cursor::new(body))
            .ok()
    }
}

pub type SigningResult<T> = Result<T, SigningError>;

/// Authorizes a signing route: the caller's tenant as [`authorize`] checks
/// it (401), then every permission in `permissions`, answering 403 when one
/// is missing.
pub fn authorize_403(
    claims: &JwtClaims,
    tenant_id: Option<String>,
    permissions: Vec<Permissions>,
) -> SigningResult<()> {
    authorize(claims, true, tenant_id, vec![])?;
    let roles: HashSet<&String> =
        claims.hasura_claims.allowed_roles.iter().collect();
    let missing: Vec<String> = permissions
        .iter()
        .map(|permission| permission.to_string())
        .filter(|permission| !roles.contains(permission))
        .collect();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(SigningError::forbidden(format!(
            "Missing permission {}",
            missing.join(", ")
        )))
    }
}

/// The caller's display name from the token (given and family name, or
/// `name`), if it carries one.
pub fn token_display_name(claims: &JwtClaims) -> Option<String> {
    let name = display_name(
        claims.given_name.as_deref(),
        claims.family_name.as_deref(),
        "",
    );
    if !name.is_empty() {
        return Some(name);
    }
    claims
        .name
        .as_deref()
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
}

/// The caller's display name: from the token, else from `directory`, else
/// the username.
pub async fn caller_display_name(
    claims: &JwtClaims,
    tenant_id: Uuid,
    directory: &dyn UserDirectory,
) -> SigningResult<String> {
    if let Some(name) = token_display_name(claims) {
        return Ok(name);
    }
    let user_id = claims.hasura_claims.user_id.clone();
    let names = directory
        .display_names(tenant_id, std::slice::from_ref(&user_id))
        .await
        .map_err(SigningError::internal)?;
    Ok(names.get(&user_id).cloned().unwrap_or_else(|| {
        claims.preferred_username.clone().unwrap_or(user_id)
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_claims::Claims;

    fn claims(tenant: &str, roles: &[Permissions]) -> JwtClaims {
        Claims::new(tenant, "user").roles(roles).build()
    }

    #[test]
    fn a_missing_permission_is_forbidden_and_another_tenant_unauthorized() {
        let with = claims("tenant", &[Permissions::SIGNING_ISSUERS_WRITE]);
        let tenant = Some("tenant".to_owned());
        assert_eq!(
            authorize_403(
                &with,
                tenant.clone(),
                vec![Permissions::SIGNING_ISSUERS_WRITE]
            ),
            Ok(())
        );
        let error = authorize_403(
            &with,
            tenant,
            vec![
                Permissions::SIGNING_ISSUERS_WRITE,
                Permissions::SIGNING_CHECKS_WRITE,
            ],
        )
        .unwrap_err();
        assert_eq!(error.status, Status::Forbidden);
        assert_eq!(
            error.body(),
            json!({
                "message": "Missing permission signing-checks-write",
                "extensions": {"code": "forbidden"}
            })
        );
        let error = authorize_403(
            &with,
            Some("other-tenant".to_owned()),
            vec![Permissions::SIGNING_ISSUERS_WRITE],
        )
        .unwrap_err();
        assert_eq!(error.status, Status::Unauthorized);
        assert_eq!(error.code, SigningErrorCode::Unauthorized);
    }

    #[test]
    fn refusals_name_their_check_and_internal_errors_hide_their_cause() {
        assert_eq!(
            SigningError::refused(
                CertificateCheckId::RegisteredToOther,
                "refused"
            )
            .body(),
            json!({
                "message": "refused",
                "extensions": {"code": "signing-refused", "check": "registered-to-other"}
            })
        );
        let internal =
            SigningError::internal("connection string with a secret");
        assert_eq!(internal.status, Status::InternalServerError);
        assert_eq!(internal.message, "Internal error");
        for (error, status, code) in [
            (SigningError::not_found("x"), Status::NotFound, "not-found"),
            (SigningError::conflict("x"), Status::Conflict, "conflict"),
            (SigningError::invalid("x"), Status::BadRequest, "invalid"),
        ] {
            assert_eq!(error.status, status);
            assert_eq!(error.body()["extensions"]["code"], code);
        }
        assert_eq!(
            SigningError::request_closed(SigningRequestStatus::Expired).body(),
            json!({
                "message": "The request is expired",
                "extensions": {"code": "request-closed", "status": "expired"}
            })
        );
        assert_eq!(
            SigningErrorCode::AlreadySigned.to_string(),
            "already-signed"
        );
        assert_eq!(
            SigningErrorCode::StaleRevision.to_string(),
            "stale-revision"
        );
    }

    struct Names;

    #[rocket::async_trait]
    impl UserDirectory for Names {
        async fn display_names(
            &self,
            _: Uuid,
            user_ids: &[String],
        ) -> anyhow::Result<std::collections::HashMap<String, String>> {
            Ok(user_ids
                .iter()
                .filter(|id| *id == "known")
                .map(|id| (id.clone(), "Known Person".to_owned()))
                .collect())
        }
    }

    #[tokio::test]
    async fn the_callers_name_comes_from_the_token_then_the_directory_then_the_username(
    ) {
        let tenant = Uuid::nil();
        let mut claims = Claims::new("tenant", "known").username("kp").build();
        claims.given_name = Some("Maria".to_owned());
        claims.family_name = Some("Santos".to_owned());
        assert_eq!(
            caller_display_name(&claims, tenant, &Names).await.unwrap(),
            "Maria Santos"
        );
        claims.given_name = None;
        claims.family_name = None;
        claims.name = Some(" Maria S. ".to_owned());
        assert_eq!(
            caller_display_name(&claims, tenant, &Names).await.unwrap(),
            "Maria S."
        );
        claims.name = None;
        assert_eq!(
            caller_display_name(&claims, tenant, &Names).await.unwrap(),
            "Known Person"
        );
        let unknown =
            Claims::new("tenant", "unknown").username("someone").build();
        assert_eq!(
            caller_display_name(&unknown, tenant, &Names).await.unwrap(),
            "someone"
        );
    }
}
