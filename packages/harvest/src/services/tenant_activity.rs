// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Records the requests that belong to no election event in the tenant's log.
//!
//! A route handler names what it was asked for with [`TenantActivityRecorder`],
//! before it authorizes the caller, so refused requests are recorded too. Once
//! the request is answered, [`fairing`] adds how it ended and queues the entry
//! for Windmill, which signs it and appends it to the tenant's log.

use crate::services::dependencies::HarvestServices;
use rocket::fairing::{Fairing, Info, Kind};
use rocket::http::Status;
use rocket::request::{FromRequest, Outcome};
use rocket::{Request, Response};
use sequent_core::services::jwt::JwtClaims;
use serde_json::Value;
use std::sync::Mutex;
use strum_macros::{Display, EnumIter, EnumString};
use tracing::error;
use uuid::Uuid;
use windmill::services::tenant_log::{TenantActivity, TenantActivityOutcome};
use windmill::tasks::electoral_log::{
    enqueue_electoral_log_event, LogEventBody, LogEventInput, LogMessageType,
};

/// What an administrator can ask the platform for outside an election event.
/// The name of an action is part of the signed entries: rename none.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Hash, Display, EnumString, EnumIter,
)]
#[strum(serialize_all = "kebab-case")]
pub enum TenantAction {
    CreateElectionEvent,
    ImportElectionEvent,
    DeleteElectionEvent,
    CreateTenant,
    DeleteTenant,
    ImportTenantConfig,
    ApplyConfigurationPackage,
    ExportTenantConfig,
    CreateUser,
    EditUser,
    DeleteUser,
    DeleteUsers,
    ImportUsers,
    ExportUsers,
    CreateRole,
    DeleteRole,
    SetUserRole,
    DeleteUserRole,
    CreatePermission,
    DeletePermission,
    SetRolePermission,
    DeleteRolePermission,
    ImportTemplates,
    LimitAccessByCountries,
}

impl TenantAction {
    /// What was asked for, to complete "Request to …".
    fn request(self) -> &'static str {
        match self {
            Self::CreateElectionEvent => "create an election event",
            Self::ImportElectionEvent => "import an election event",
            Self::DeleteElectionEvent => "delete an election event",
            Self::CreateTenant => "create a tenant",
            Self::DeleteTenant => "delete a tenant",
            Self::ImportTenantConfig => "import the tenant's configuration",
            Self::ApplyConfigurationPackage => "apply a configuration package",
            Self::ExportTenantConfig => "export the tenant's configuration",
            Self::CreateUser => "create a staff account",
            Self::EditUser => "edit a staff account",
            Self::DeleteUser => "delete a staff account",
            Self::DeleteUsers => "delete staff accounts",
            Self::ImportUsers => "import staff accounts",
            Self::ExportUsers => "export staff accounts",
            Self::CreateRole => "create a role",
            Self::DeleteRole => "delete a role",
            Self::SetUserRole => "give a role to a staff account",
            Self::DeleteUserRole => "remove a role from a staff account",
            Self::CreatePermission => "create a permission",
            Self::DeletePermission => "delete a permission",
            Self::SetRolePermission => "add a permission to a role",
            Self::DeleteRolePermission => "remove a permission from a role",
            Self::ImportTemplates => "import templates",
            Self::LimitAccessByCountries => "limit access by countries",
        }
    }

    /// The entry's description for a request that ended as `outcome`.
    pub fn description(self, outcome: TenantActivityOutcome) -> String {
        let ending = match outcome {
            TenantActivityOutcome::Succeeded => "accepted",
            TenantActivityOutcome::Denied => "denied",
            TenantActivityOutcome::Failed => "failed",
        };
        format!("Request to {} {ending}.", self.request())
    }
}

/// How a request answered with `status` ended.
pub fn outcome_of(status: Status) -> TenantActivityOutcome {
    match status.code {
        200..=299 => TenantActivityOutcome::Succeeded,
        401 | 403 => TenantActivityOutcome::Denied,
        _ => TenantActivityOutcome::Failed,
    }
}

/// What a handler said its request was, and who made it.
#[derive(Clone, Debug, PartialEq)]
struct Recorded {
    tenant_id: String,
    user_id: String,
    username: Option<String>,
    action: TenantAction,
    details: Value,
}

#[derive(Debug, Default)]
struct Slot(Mutex<Option<Recorded>>);

/// Lets a route handler say that its request belongs in the tenant's log.
#[derive(Debug)]
pub struct TenantActivityRecorder<'r> {
    slot: &'r Slot,
}

impl TenantActivityRecorder<'_> {
    /// Name what the caller asked for and what it was asked on. Call it before
    /// authorizing the caller. The caller's tenant keeps the entry, so a request
    /// on another tenant is recorded where its author belongs. The last call of
    /// a request is the one recorded.
    pub fn record(
        &self,
        claims: &JwtClaims,
        action: TenantAction,
        details: Value,
    ) {
        let recorded = Recorded {
            tenant_id: claims.hasura_claims.tenant_id.clone(),
            user_id: claims.hasura_claims.user_id.clone(),
            username: claims.preferred_username.clone(),
            action,
            details,
        };
        match self.slot.0.lock() {
            Ok(mut slot) => *slot = Some(recorded),
            Err(_) => {
                error!("tenant activity: the request's record is poisoned")
            }
        }
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for TenantActivityRecorder<'r> {
    type Error = std::convert::Infallible;

    async fn from_request(
        request: &'r Request<'_>,
    ) -> Outcome<Self, Self::Error> {
        Outcome::Success(Self {
            slot: request.local_cache(Slot::default),
        })
    }
}

/// The queued event of a recorded request that was answered with `status`.
fn log_event(
    recorded: Recorded,
    status: Status,
) -> serde_json::Result<LogEventInput> {
    let outcome = outcome_of(status);
    let activity = TenantActivity {
        action: recorded.action.to_string(),
        description: recorded.action.description(outcome),
        outcome,
        status: status.code,
        details: recorded.details,
    };
    Ok(LogEventInput {
        delivery_id: Some(Uuid::new_v4().to_string()),
        election_event_id: String::new(),
        message_type: LogMessageType::TenantActivity,
        user_id: Some(recorded.user_id),
        username: recorded.username,
        tenant_id: recorded.tenant_id,
        body: LogEventBody::Plain(serde_json::to_string(&activity)?),
        event_time_ms: Some(chrono::Utc::now().timestamp_millis()),
        details: None,
        admin_event: None,
    })
}

struct TenantActivityFairing;

#[rocket::async_trait]
impl Fairing for TenantActivityFairing {
    fn info(&self) -> Info {
        Info {
            name: "Tenant activity log",
            kind: Kind::Response,
        }
    }

    async fn on_response<'r>(
        &self,
        request: &'r Request<'_>,
        response: &mut Response<'r>,
    ) {
        let recorded = match request.local_cache(Slot::default).0.lock() {
            Ok(mut slot) => slot.take(),
            Err(_) => None,
        };
        let Some(recorded) = recorded else {
            return;
        };
        let action = recorded.action;
        let Some(services) = request.rocket().state::<HarvestServices>() else {
            error!(
                "tenant activity: no managed services, {action} is not logged"
            );
            return;
        };
        let event = match log_event(recorded, response.status()) {
            Ok(event) => event,
            Err(err) => {
                error!("tenant activity: {action} cannot be encoded: {err}");
                return;
            }
        };
        // The request is already answered: a queue that cannot be reached
        // loses the entry, and the service log says so.
        if let Err(err) = services
            .tasks
            .send_task(enqueue_electoral_log_event::new(event))
            .await
        {
            error!("tenant activity: {action} could not be queued: {err:#}");
        }
    }
}

/// Queues the tenant-log entry of every request whose handler recorded one.
pub fn fairing() -> impl Fairing {
    TenantActivityFairing
}

#[cfg(test)]
#[path = "../../tests/support/tenant_activity.rs"]
mod tests;
