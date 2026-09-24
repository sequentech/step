// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Pin every guarded route's permissions from both sides. With its minimum
//! permissions a request must get past authorization and stop at the first
//! backend, which the isolated child process leaves unconfigured. With any one
//! of them removed it must get the route's current denial. A route that starts
//! requiring another permission fails the first check; one that stops
//! requiring a permission fails the second.

use super::{
    bearer, client, http, is_isolated_child, run_isolated, OTHER_TENANT_ID,
    SUPER_ADMIN_TENANT_ID, TENANT_ID, USER_ID,
};
use crate::test_claims::Claims;
use chrono::Utc;
use rocket::http::{ContentType, Status};
use rocket::local::asynchronous::Client;
use sequent_core::types::permissions::{Permissions, VoterPermissions};
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::time::Duration;

const EVENT_ID: &str = "test-event";
const AREA_ID: &str = "test-area";
const USERNAME: &str = "test-username";
const VOTING_PORTAL_CLIENT: &str = "voting-portal";
// Some routes parse these ids as UUIDs before reaching a backend.
const UUID_TENANT_ID: &str = "00000000-0000-0000-0000-00000000000a";
const UUID_EVENT_ID: &str = "00000000-0000-0000-0000-00000000000e";
const ELECTION_ID: &str = "00000000-0000-0000-0000-000000000001";
const PEM: &str =
    "-----BEGIN CERTIFICATE-----\nAA==\n-----END CERTIFICATE-----\n";

/// How a route answered: its status, and whether the body was JSON or the
/// plain text of a handler's `(Status, String)` error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Reply {
    Json(Status),
    Text(Status),
}

/// The first backend call panicked and the catcher answered, or the route
/// mapped the failure to its own JSON error.
const BACKEND: Reply = Reply::Json(Status::InternalServerError);
/// The route mapped the backend failure to a plain-text error.
const BACKEND_TEXT: Reply = Reply::Text(Status::InternalServerError);
const BAD_REQUEST: Reply = Reply::Text(Status::BadRequest);
const UNAUTHORIZED: Reply = Reply::Text(Status::Unauthorized);
const UNAUTHORIZED_JSON: Reply = Reply::Json(Status::Unauthorized);
const FORBIDDEN: Reply = Reply::Text(Status::Forbidden);
const FORBIDDEN_JSON: Reply = Reply::Json(Status::Forbidden);

#[derive(Clone, Copy, Debug)]
enum Identity {
    /// An admin portal user of the fixture tenant.
    Admin,
    /// An admin who has just authenticated at the gold level.
    Gold,
    /// An admin of the super-admin tenant the child process configures.
    SuperAdmin,
    /// An admin of a tenant whose id is a UUID.
    UuidTenant,
    /// A voter of the fixture election, signed in to the voting portal.
    Voter,
}

impl Identity {
    fn claims(self) -> Claims {
        let tenant_id = match self {
            Self::SuperAdmin => SUPER_ADMIN_TENANT_ID,
            Self::UuidTenant => UUID_TENANT_ID,
            _ => TENANT_ID,
        };
        let claims = Claims::new(tenant_id, USER_ID).username(USERNAME);
        match self {
            Self::Gold => claims
                .acr(&Permissions::GOLD.to_string())
                .auth_time(Utc::now().timestamp()),
            Self::Voter => claims
                .azp(VOTING_PORTAL_CLIENT)
                .area(AREA_ID)
                .election_event(EVENT_ID)
                .authorized_elections(&[ELECTION_ID]),
            _ => claims,
        }
    }
}

struct Case {
    identity: Identity,
    path: &'static str,
    body: Value,
    permissions: Vec<String>,
    /// The answer with every permission in `permissions`.
    answer: Reply,
    /// The answer with any one of them removed.
    denied: Option<Reply>,
}

impl Case {
    async fn send(
        &self,
        client: &Client,
        permissions: &[String],
    ) -> Result<Reply, String> {
        let request = client
            .post(self.path)
            .header(ContentType::JSON)
            .header(bearer(&self.identity.claims().roles(permissions)))
            .body(self.body.to_string());
        let response =
            tokio::time::timeout(Duration::from_secs(3), request.dispatch())
                .await
                .map_err(|_| "no answer within 3 s".to_string())?;
        reply(&response)
    }
}

fn reply(
    response: &rocket::local::asynchronous::LocalResponse<'_>,
) -> Result<Reply, String> {
    let status = response.status();
    match response.content_type() {
        Some(media) if media == ContentType::JSON => Ok(Reply::Json(status)),
        Some(media) if media == ContentType::Plain => Ok(Reply::Text(status)),
        media => Err(format!("{status} with content type {media:?}")),
    }
}

/// One line per guarded route, or per request shape when a route's
/// permissions depend on the body: who calls it, the body, the minimum
/// permissions, the answer with them and the answer with one of them removed.
/// Routes without a permission to remove have no denial.
macro_rules! case {
    ($identity:ident, $path:literal, $body:tt, [$($permission:expr),+], $answer:expr, $denied:expr) => {
        Case {
            identity: Identity::$identity,
            path: $path,
            body: json!($body),
            permissions: vec![$($permission.to_string()),+],
            answer: $answer,
            denied: Some($denied),
        }
    };
    ($identity:ident, $path:literal, $body:tt, [], $answer:expr) => {
        Case {
            identity: Identity::$identity,
            path: $path,
            body: json!($body),
            permissions: vec![],
            answer: $answer,
            denied: None,
        }
    };
}

fn cases() -> Vec<Case> {
    use Permissions::*;
    use VoterPermissions::{ACK_SUPPORT_MATERIALS, CAST_VOTE};
    vec![
        case!(Voter, "/acknowledge-support-materials", {"election_event_id": EVENT_ID, "document_ids": []}, [ACK_SUPPORT_MATERIALS], BACKEND, UNAUTHORIZED),
        case!(Admin, "/apply-reconciliation-changes", {"election_event_id": EVENT_ID, "diff_document_id": "test-document"}, [ELECTION_EVENT_VOTER_LIST_SYNC], BACKEND, FORBIDDEN),
        case!(Admin, "/change-application-status", {"tenant_id": TENANT_ID, "election_event_id": EVENT_ID, "id": "test-application", "user_id": USER_ID}, [APPLICATION_WRITE], BACKEND, UNAUTHORIZED_JSON),
        case!(Admin, "/check-private-key", {"election_event_id": EVENT_ID, "keys_ceremony_id": "test-ceremony", "private_key_base64": "not-a-key"}, [TRUSTEE_CEREMONY], BACKEND, UNAUTHORIZED),
        case!(Admin, "/configure-results-website-policy", {"election_event_id": EVENT_ID, "status": "enabled", "access": "public", "visibility_scope": "full_event"}, [PUBLISH_RESULTS_WRITE], BACKEND, UNAUTHORIZED),
        case!(Admin, "/count-users", {"tenant_id": TENANT_ID}, [USER_READ], BACKEND, UNAUTHORIZED),
        case!(Admin, "/count-users", {"tenant_id": TENANT_ID, "election_event_id": EVENT_ID}, [VOTER_READ], BACKEND_TEXT, UNAUTHORIZED),
        case!(Voter, "/create-ballot-receipt", {"ballot_id": "test-ballot", "ballot_tracker_url": "https://tracker.invalid/test", "election_event_id": EVENT_ID, "election_id": ELECTION_ID}, [CAST_VOTE], BACKEND, UNAUTHORIZED),
        case!(Admin, "/create-election", {"election_event_id": EVENT_ID, "external_id": "test-election", "presentation": {}}, [ELECTION_EVENT_WRITE], BACKEND, UNAUTHORIZED),
        case!(Admin, "/create-keys-ceremony", {"election_event_id": EVENT_ID, "threshold": 1, "trustee_names": ["test-trustee"], "is_automatic_ceremony": false}, [ADMIN_CEREMONY], BACKEND, UNAUTHORIZED),
        case!(Admin, "/create-new-tally-sheet", {"election_event_id": EVENT_ID, "channel": "PAPER", "content": {"area_id": AREA_ID, "contest_id": "test-contest", "candidate_results": {}}, "contest_id": "test-contest", "area_id": AREA_ID}, [TALLY_SHEET_CREATE], BACKEND, UNAUTHORIZED),
        case!(Admin, "/create-permission", {"tenant_id": TENANT_ID, "permission": {}}, [USER_PERMISSION_CREATE], BACKEND_TEXT, UNAUTHORIZED),
        case!(Admin, "/create-phone-blacklist-entry", {"election_event_id": EVENT_ID, "phone_e164": "+34600000000"}, [PHONE_BLACKLIST_CREATE], BACKEND, UNAUTHORIZED),
        case!(Admin, "/create-reconciliation-import", {"election_event_id": EVENT_ID, "document_id": "test-document"}, [ELECTION_EVENT_VOTER_LIST_SYNC], BACKEND, FORBIDDEN),
        case!(Admin, "/create-role", {"tenant_id": TENANT_ID, "role": {}}, [ROLE_CREATE], BACKEND_TEXT, UNAUTHORIZED),
        case!(Admin, "/create-tally-ceremony", {"election_event_id": EVENT_ID, "election_ids": [], "tally_type": "ELECTORAL_RESULTS"}, [ADMIN_CEREMONY], BACKEND, UNAUTHORIZED_JSON),
        case!(Admin, "/create-tally-sheet-import", {"election_event_id": EVENT_ID, "document_id": "test-document", "source_format": "CANONICAL_CSV", "selected_channel": "PAPER"}, [TALLY_SHEET_IMPORT_CREATE], BACKEND, UNAUTHORIZED),
        case!(Admin, "/create-user", {"tenant_id": TENANT_ID, "user": {}}, [USER_CREATE], BACKEND, UNAUTHORIZED_JSON),
        case!(Admin, "/create-user", {"tenant_id": TENANT_ID, "user": {"attributes": {"permission_labels": ["test-label"]}}}, [USER_CREATE, PERMISSION_LABEL_WRITE], BACKEND, UNAUTHORIZED_JSON),
        case!(Admin, "/create-user", {"tenant_id": TENANT_ID, "election_event_id": EVENT_ID, "user": {}}, [VOTER_CREATE], BACKEND, UNAUTHORIZED_JSON),
        case!(Admin, "/create-user", {"tenant_id": TENANT_ID, "election_event_id": EVENT_ID, "user": {}, "secret_attributes": {"test-secret": ["value"]}}, [VOTER_CREATE, VOTER_SECRET_ATTRIBUTE_WRITE], BACKEND, UNAUTHORIZED_JSON),
        case!(UuidTenant, "/delete-certificate-authority", {"ids": [], "election_event_id": UUID_EVENT_ID}, [CA_WRITE], BACKEND, UNAUTHORIZED),
        // The task row is written before the permission check.
        case!(Admin, "/delete-election-event", {"election_event_id": EVENT_ID}, [ELECTION_EVENT_DELETE], BACKEND, BACKEND),
        case!(Admin, "/delete-permission", {"tenant_id": TENANT_ID, "permission_name": "test-permission"}, [USER_PERMISSION_WRITE], BACKEND_TEXT, UNAUTHORIZED),
        case!(Admin, "/delete-phone-blacklist-entry", {"id": "test-entry", "election_event_id": EVENT_ID}, [PHONE_BLACKLIST_DELETE], BACKEND, UNAUTHORIZED),
        case!(Admin, "/delete-role", {"tenant_id": TENANT_ID, "role_id": "test-role"}, [ROLE_WRITE], BACKEND_TEXT, UNAUTHORIZED),
        case!(Admin, "/delete-role-permission", {"tenant_id": TENANT_ID, "role_id": "test-role", "permission_name": "test-permission"}, [USER_PERMISSION_WRITE, ROLE_WRITE], BACKEND_TEXT, UNAUTHORIZED),
        // The task row is written before the permission check.
        case!(SuperAdmin, "/delete-tenant", {"tenant_id": TENANT_ID}, [TENANT_DELETE], BACKEND, BACKEND),
        case!(Admin, "/delete-user", {"tenant_id": TENANT_ID, "user_id": USER_ID}, [USER_WRITE], BACKEND_TEXT, UNAUTHORIZED),
        case!(Admin, "/delete-user", {"tenant_id": TENANT_ID, "election_event_id": EVENT_ID, "user_id": USER_ID}, [VOTER_DELETE], BACKEND, UNAUTHORIZED),
        case!(Admin, "/delete-user-role", {"tenant_id": TENANT_ID, "user_id": USER_ID, "role_id": "test-role"}, [USER_WRITE, ROLE_WRITE], BACKEND_TEXT, UNAUTHORIZED),
        case!(Admin, "/delete-users", {"tenant_id": TENANT_ID, "users_id": [USER_ID]}, [USER_WRITE], BACKEND_TEXT, UNAUTHORIZED),
        case!(Admin, "/delete-users", {"tenant_id": TENANT_ID, "election_event_id": EVENT_ID, "users_id": [USER_ID]}, [VOTER_DELETE], BACKEND, UNAUTHORIZED),
        case!(Admin, "/edit-user", {"tenant_id": TENANT_ID, "user_id": USER_ID}, [USER_WRITE], BACKEND, UNAUTHORIZED_JSON),
        case!(Admin, "/edit-user", {"tenant_id": TENANT_ID, "user_id": USER_ID, "attributes": {"permission_labels": ["test-label"]}}, [USER_WRITE, PERMISSION_LABEL_WRITE], BACKEND, UNAUTHORIZED_JSON),
        case!(Admin, "/edit-user", {"tenant_id": TENANT_ID, "election_event_id": EVENT_ID, "user_id": USER_ID, "first_name": "Ada"}, [VOTER_WRITE], BACKEND, UNAUTHORIZED_JSON),
        // VOTER_EMAIL_TLF_EDIT substitutes for VOTER_WRITE, whichever fields change.
        case!(Admin, "/edit-user", {"tenant_id": TENANT_ID, "election_event_id": EVENT_ID, "user_id": USER_ID, "first_name": "Ada"}, [VOTER_EMAIL_TLF_EDIT], BACKEND, UNAUTHORIZED_JSON),
        case!(Admin, "/edit-user", {"tenant_id": TENANT_ID, "election_event_id": EVENT_ID, "user_id": USER_ID, "password": "test-password"}, [VOTER_CHANGE_PASSWORD], BACKEND, UNAUTHORIZED_JSON),
        case!(Admin, "/edit-user", {"tenant_id": TENANT_ID, "election_event_id": EVENT_ID, "user_id": USER_ID, "password": "test-password", "first_name": "Ada"}, [VOTER_WRITE, VOTER_CHANGE_PASSWORD], BACKEND, UNAUTHORIZED_JSON),
        case!(Admin, "/edit-user", {"tenant_id": TENANT_ID, "election_event_id": EVENT_ID, "user_id": USER_ID, "secret_attributes": {"test-secret": ["value"]}}, [VOTER_WRITE, VOTER_SECRET_ATTRIBUTE_WRITE], BACKEND, UNAUTHORIZED_JSON),
        case!(Admin, "/election-event/stats", {"election_event_id": EVENT_ID, "start_date": "2026-01-01", "end_date": "2026-01-02", "user_timezone": "UTC"}, [ADMIN_DASHBOARD_VIEW], BACKEND, UNAUTHORIZED),
        case!(Admin, "/election-event/top-votes-by-ip", {"election_event_id": EVENT_ID}, [ADMIN_DASHBOARD_VIEW], BACKEND, UNAUTHORIZED),
        case!(Admin, "/election/stats", {"election_event_id": EVENT_ID, "election_id": "test-election", "start_date": "2026-01-01", "end_date": "2026-01-02", "user_timezone": "UTC"}, [ADMIN_DASHBOARD_VIEW], BACKEND, UNAUTHORIZED),
        case!(Admin, "/encrypt-report", {"election_event_id": EVENT_ID, "password": "test-password"}, [REPORT_WRITE], BACKEND, UNAUTHORIZED),
        // The task row is written before the permission check.
        case!(Admin, "/export-application", {"tenant_id": TENANT_ID, "election_event_id": EVENT_ID}, [APPLICATION_EXPORT], BACKEND, BACKEND),
        // The task row is written before the permission check.
        case!(Admin, "/export-ballot-publication", {"tenant_id": TENANT_ID, "election_event_id": EVENT_ID, "ballot_publication_id": "test-publication"}, [PUBLISH_WRITE], BACKEND, BACKEND),
        case!(Admin, "/export-certificate-authority", {"ids": [], "election_event_id": UUID_EVENT_ID}, [CA_READ], BACKEND, UNAUTHORIZED),
        case!(Admin, "/export-election-event", {"election_event_id": EVENT_ID, "export_configurations": {}}, [ELECTION_EVENT_READ], BACKEND, UNAUTHORIZED),
        case!(Admin, "/export-election-event-logs", {"election_event_id": EVENT_ID, "format": "CSV"}, [REPORT_WRITE], BACKEND, UNAUTHORIZED),
        case!(Admin, "/export-tally-results", {"election_event_id": EVENT_ID, "tally_session_id": "test-session"}, [TALLY_RESULTS_READ], BACKEND, UNAUTHORIZED),
        case!(Admin, "/export-tasks-execution", {"tenant_id": TENANT_ID, "election_event_id": EVENT_ID}, [TASKS_READ], BACKEND, UNAUTHORIZED),
        // The task row is written before the permission check.
        case!(Admin, "/export-template", {"tenant_id": TENANT_ID}, [TEMPLATE_WRITE], BACKEND, BACKEND),
        case!(Admin, "/export-tenant-config", {"tenant_id": TENANT_ID}, [TENANT_READ], BACKEND, UNAUTHORIZED),
        case!(Admin, "/export-tenant-users", {"tenant_id": TENANT_ID}, [USER_READ], BACKEND, UNAUTHORIZED),
        // The task row is written before the permission check.
        case!(Admin, "/export-trustees", {"password": "test-password"}, [TRUSTEES_EXPORT], BACKEND, BACKEND),
        case!(Admin, "/export-users", {"tenant_id": TENANT_ID}, [USER_READ], BACKEND, UNAUTHORIZED),
        // The secret-attribute check comes after the tenant-user rejection.
        case!(Admin, "/export-users", {"tenant_id": TENANT_ID, "include_secret_attributes": true}, [USER_READ], BAD_REQUEST, UNAUTHORIZED),
        case!(Admin, "/export-users", {"tenant_id": TENANT_ID, "election_event_id": EVENT_ID}, [VOTER_READ], BACKEND, UNAUTHORIZED),
        case!(Admin, "/export-users", {"tenant_id": TENANT_ID, "election_event_id": EVENT_ID, "include_secret_attributes": true}, [VOTER_READ, VOTER_SECRET_ATTRIBUTE_READ], BACKEND, UNAUTHORIZED),
        case!(Admin, "/fetch-document", {"document_id": "test-document"}, [DOCUMENT_DOWNLOAD], BACKEND, UNAUTHORIZED),
        case!(Admin, "/fetch-results-artifact", {"election_event_id": EVENT_ID, "publication_id": "test-publication"}, [], BACKEND),
        case!(Gold, "/generate-ballot-publication", {"election_event_id": EVENT_ID}, [PUBLISH_WRITE], BACKEND, UNAUTHORIZED_JSON),
        case!(Admin, "/generate-google-meeting", {"summary": "test", "description": "test", "start_date_time": "2030-01-01T10:00:00Z", "end_date_time": "2030-01-01T11:00:00Z", "time_zone": "UTC", "attendee_emails": []}, [GOOGLE_MEET_LINK], BACKEND, UNAUTHORIZED),
        case!(Admin, "/generate-preview-url", {"tenant_id": TENANT_ID, "document_id": "test-document"}, [GENERATE_PREVIEW], BACKEND, UNAUTHORIZED),
        case!(Admin, "/generate-report", {"report_id": "test-report", "tenant_id": TENANT_ID, "report_mode": "PREVIEW"}, [REPORT_READ], BACKEND, UNAUTHORIZED),
        // The permission is checked in the caller's tenant, not the body's.
        case!(Admin, "/generate-report", {"report_id": "test-report", "tenant_id": OTHER_TENANT_ID, "report_mode": "PREVIEW"}, [REPORT_READ], BACKEND, UNAUTHORIZED),
        case!(Admin, "/generate-template", {"type": "BallotImages", "election_event_id": EVENT_ID, "election_id": "test-election", "tally_session_id": "test-session"}, [REPORT_READ], BACKEND, UNAUTHORIZED),
        case!(Admin, "/generate-voter-information-letter", {"election_event_id": EVENT_ID, "voter_id": "test-voter"}, [VOTER_INFORMATION_LETTER, DOCUMENT_PASSWORD_READ], BACKEND, FORBIDDEN_JSON),
        case!(Voter, "/get-ballot-files-urls", {"election_event_id": EVENT_ID}, [CAST_VOTE], BACKEND, FORBIDDEN),
        case!(Admin, "/get-ballot-publication-changes", {"election_event_id": EVENT_ID, "ballot_publication_id": "test-publication"}, [PUBLISH_READ], BACKEND, UNAUTHORIZED),
        case!(Admin, "/get-custom-url", {"redirect_to": "https://redirect.invalid"}, [ELECTION_EVENT_READ], BACKEND_TEXT, UNAUTHORIZED),
        case!(Admin, "/get-document-password", {"document_id": "test-document"}, [DOCUMENT_DOWNLOAD, DOCUMENT_PASSWORD_READ], BACKEND, FORBIDDEN_JSON),
        case!(Admin, "/get-permissions", {"tenant_id": TENANT_ID}, [USER_PERMISSION_READ], BACKEND_TEXT, UNAUTHORIZED),
        case!(Admin, "/get-private-key", {"election_event_id": EVENT_ID, "keys_ceremony_id": "test-ceremony"}, [TRUSTEE_CEREMONY], BACKEND, UNAUTHORIZED_JSON),
        case!(Admin, "/get-realm-attributes", {"election_event_id": EVENT_ID}, [KEYCLOAK_REALM_ATTRIBUTES_READ], BACKEND_TEXT, FORBIDDEN),
        case!(Admin, "/get-realm-password-policy", {"election_event_id": EVENT_ID}, [ELECTION_EVENT_READ], BACKEND, FORBIDDEN_JSON),
        case!(Admin, "/get-roles", {"tenant_id": TENANT_ID}, [ROLE_READ], BACKEND_TEXT, UNAUTHORIZED),
        case!(Voter, "/get-support-materials-acknowledgment", {"election_event_id": EVENT_ID}, [ACK_SUPPORT_MATERIALS], BACKEND_TEXT, UNAUTHORIZED),
        case!(Admin, "/get-upload-url", {"name": "test.csv", "media_type": "text/csv", "size": 1, "is_public": false}, [DOCUMENT_UPLOAD], BACKEND, UNAUTHORIZED),
        case!(Admin, "/get-user", {"tenant_id": TENANT_ID, "user_id": USER_ID}, [USER_READ], BACKEND_TEXT, UNAUTHORIZED),
        case!(Admin, "/get-user", {"tenant_id": TENANT_ID, "election_event_id": EVENT_ID, "user_id": USER_ID}, [VOTER_READ], BACKEND_TEXT, UNAUTHORIZED),
        case!(Admin, "/get-user-profile-attributes", {"tenant_id": TENANT_ID}, [USER_READ], BACKEND_TEXT, UNAUTHORIZED),
        case!(Admin, "/get-user-profile-attributes", {"tenant_id": TENANT_ID, "election_event_id": EVENT_ID}, [VOTER_READ], BACKEND_TEXT, UNAUTHORIZED),
        case!(Admin, "/get-user-profile-configuration", {"tenant_id": TENANT_ID}, [USER_READ], BACKEND_TEXT, UNAUTHORIZED),
        case!(Admin, "/get-user-profile-configuration", {"tenant_id": TENANT_ID, "election_event_id": EVENT_ID}, [VOTER_READ], BACKEND_TEXT, UNAUTHORIZED),
        case!(Admin, "/get-user-template", {"template_type": "test-template"}, [REPORT_READ], BACKEND_TEXT, UNAUTHORIZED),
        case!(Admin, "/get-users", {"tenant_id": TENANT_ID}, [USER_READ], BACKEND, UNAUTHORIZED),
        case!(Admin, "/get-users", {"tenant_id": TENANT_ID, "election_event_id": EVENT_ID}, [VOTER_READ], BACKEND_TEXT, UNAUTHORIZED),
        case!(Admin, "/immudb/electoral-log", {"tenant_id": TENANT_ID, "election_event_id": EVENT_ID}, [LOGS_READ], BACKEND_TEXT, UNAUTHORIZED),
        case!(Admin, "/immudb/pgaudit-list", {"tenant_id": TENANT_ID, "election_event_id": EVENT_ID}, [LOGS_READ], BACKEND_TEXT, UNAUTHORIZED),
        case!(Admin, "/import-application", {"tenant_id": TENANT_ID, "election_event_id": EVENT_ID, "document_id": "test-document"}, [APPLICATION_IMPORT], BACKEND, UNAUTHORIZED),
        case!(Admin, "/import-areas", {"election_event_id": EVENT_ID, "document_id": "test-document"}, [AREA_WRITE], BACKEND, UNAUTHORIZED),
        // The task row is written before the permission check.
        case!(Admin, "/import-candidates", {"election_event_id": EVENT_ID, "document_id": "test-document"}, [ADMIN_USER], BACKEND, BACKEND),
        case!(UuidTenant, "/import-certificate-authority", {"election_event_id": UUID_EVENT_ID, "pem_content": PEM}, [CA_WRITE], BACKEND, UNAUTHORIZED),
        // The permission list is empty, so only the body tenant is checked.
        case!(Admin, "/import-election-event", {"tenant_id": TENANT_ID, "document_id": "test-document"}, [], BACKEND),
        case!(Admin, "/import-election-event", {"tenant_id": OTHER_TENANT_ID, "document_id": "test-document"}, [], UNAUTHORIZED),
        // The task row is written before the permission check.
        case!(Admin, "/import-templates", {"tenant_id": TENANT_ID, "document_id": "test-document"}, [TEMPLATE_WRITE], BACKEND, BACKEND),
        case!(Admin, "/import-tenant-config", {"tenant_id": TENANT_ID, "document_id": "test-document", "import_configurations": {}}, [TENANT_WRITE], BACKEND, UNAUTHORIZED),
        // The task row is written before the permission check.
        case!(Admin, "/import-users", {"tenant_id": TENANT_ID, "document_id": "test-document"}, [USER_CREATE], BACKEND, BACKEND),
        case!(Admin, "/import-users", {"tenant_id": TENANT_ID, "election_event_id": EVENT_ID, "document_id": "test-document"}, [VOTER_CREATE], BACKEND, BACKEND),
        case!(Voter, "/insert-cast-vote", {"ballot_id": "test-ballot", "election_id": ELECTION_ID, "content": "test-content"}, [CAST_VOTE], BACKEND, UNAUTHORIZED_JSON),
        case!(Admin, "/insert-election-event", {"tenant_id": TENANT_ID, "name": "test-event"}, [ELECTION_EVENT_CREATE], BACKEND, UNAUTHORIZED),
        // The task row is written before the permission check.
        case!(SuperAdmin, "/insert-tenant", {"slug": "test-slug"}, [TENANT_CREATE], BACKEND, BACKEND),
        case!(Admin, "/limit-access-by-countries", {"voting_countries": [], "enroll_countries": []}, [CLOUDFLARE_WRITE], BACKEND, UNAUTHORIZED_JSON),
        case!(Voter, "/list-cast-vote-messages", {"tenant_id": TENANT_ID, "election_event_id": EVENT_ID, "election_id": ELECTION_ID, "ballot_id": "test-ballot-id"}, [CAST_VOTE], BACKEND, UNAUTHORIZED_JSON),
        // Either ceremony permission is enough.
        case!(Admin, "/list-keys-ceremonies", {"election_event_id": EVENT_ID}, [ADMIN_CEREMONY], BACKEND, UNAUTHORIZED),
        case!(Admin, "/list-keys-ceremonies", {"election_event_id": EVENT_ID}, [TRUSTEE_CEREMONY], BACKEND, UNAUTHORIZED),
        case!(Admin, "/list-user-roles", {"tenant_id": TENANT_ID, "user_id": USER_ID}, [USER_READ, ROLE_READ], BACKEND_TEXT, UNAUTHORIZED),
        case!(Admin, "/list-user-roles", {"tenant_id": TENANT_ID, "election_event_id": EVENT_ID, "user_id": USER_ID}, [VOTER_READ, ROLE_READ], BACKEND_TEXT, UNAUTHORIZED),
        case!(Admin, "/manage-election-dates", {"election_event_id": EVENT_ID, "event_processor": "START_VOTING_PERIOD"}, [SCHEDULED_EVENT_WRITE], BACKEND, UNAUTHORIZED_JSON),
        // The task row is written before the permission check.
        case!(Admin, "/miru/create-transmission-package", {"election_event_id": EVENT_ID, "election_id": "test-election", "area_id": AREA_ID, "tally_session_id": "test-session", "force": false}, [MIRU_CREATE], BACKEND, BACKEND),
        case!(Admin, "/miru/send-transmission-package", {"election_id": "test-election", "area_id": AREA_ID, "tally_session_id": "test-session"}, [MIRU_SEND], BACKEND, UNAUTHORIZED),
        case!(Admin, "/miru/upload-signature", {"election_id": "test-election", "area_id": AREA_ID, "tally_session_id": "test-session", "document_id": "test-document", "password": "test-password"}, [MIRU_SIGN], BACKEND_TEXT, UNAUTHORIZED),
        // Plugins receive the claims and make their own decisions.
        case!(Admin, "/plugin", {"path": "/test-plugin/test-route", "data": {}}, [], BACKEND_TEXT),
        case!(Admin, "/prepare-ballot-publication-preview", {"election_event_id": EVENT_ID, "ballot_publication_id": "test-publication"}, [DOCUMENT_UPLOAD], BACKEND, UNAUTHORIZED),
        case!(Admin, "/preview-tally-sheet-import", {"election_event_id": EVENT_ID, "document_id": "test-document", "source_format": "CANONICAL_CSV", "selected_channel": "PAPER"}, [TALLY_SHEET_IMPORT_CREATE], BACKEND, UNAUTHORIZED),
        case!(Admin, "/publish-ballot", {"election_event_id": EVENT_ID, "ballot_publication_id": "test-publication"}, [PUBLISH_WRITE], BACKEND, UNAUTHORIZED_JSON),
        case!(Admin, "/publish-results-website", {"election_event_id": EVENT_ID, "tally_session_id": "test-session", "tally_session_execution_id": "test-execution", "results_event_id": "test-results-event", "route_scope": "event", "election_ids": ["test-election"], "contest_ids": ["test-contest"], "access": "public", "visibility_scope": "full_event"}, [PUBLISH_RESULTS_WRITE], BACKEND, UNAUTHORIZED),
        case!(Admin, "/recount-tally-session", {"election_event_id": EVENT_ID, "tally_session_id": "test-session"}, [TALLY_RECOUNT_EXECUTE], BACKEND, UNAUTHORIZED),
        case!(Admin, "/refresh-results-publication-index", {"election_event_id": EVENT_ID}, [PUBLISH_RESULTS_WRITE], BACKEND, UNAUTHORIZED),
        case!(Admin, "/render-document-pdf", {"document_id": "test-document"}, [REPORT_READ], BACKEND, UNAUTHORIZED),
        // Readers are authorized by the service after it loads the publication.
        case!(Admin, "/resolve-results-publication", {"ee_id": EVENT_ID}, [], BACKEND),
        case!(Admin, "/restore-private-key", {"election_event_id": EVENT_ID, "private_key_base64": "not-a-key", "tally_session_id": "test-session"}, [TRUSTEE_CEREMONY], BACKEND, UNAUTHORIZED),
        case!(Admin, "/reveal-voter-secret-attribute", {"tenant_id": TENANT_ID, "election_event_id": EVENT_ID, "user_id": USER_ID, "attribute_name": "test-secret"}, [VOTER_READ, VOTER_SECRET_ATTRIBUTE_READ], BACKEND_TEXT, UNAUTHORIZED),
        case!(Admin, "/review-tally-sheet", {"election_event_id": EVENT_ID, "tally_sheet_id": "test-sheet", "new_status": "DISAPPROVED"}, [TALLY_SHEET_REVIEW], BACKEND, UNAUTHORIZED),
        case!(Admin, "/review-tally-sheet-import", {"election_event_id": EVENT_ID, "import_id": "test-import", "decision": "DISAPPROVE"}, [TALLY_SHEET_IMPORT_REVIEW], BACKEND, UNAUTHORIZED),
        case!(Admin, "/revoke-results-publication", {"election_event_id": EVENT_ID, "publication_id": "test-publication"}, [PUBLISH_RESULTS_WRITE], BACKEND, UNAUTHORIZED),
        case!(Admin, "/scheduled-event", {"tenant_id": TENANT_ID, "event_processor": "SEND_TEMPLATE", "event_payload": {}}, [NOTIFICATION_SEND], BACKEND, UNAUTHORIZED),
        // An empty permission list checked in the caller's tenant; the body
        // tenant, which the report task receives, is not checked.
        case!(Admin, "/scheduled-event", {"tenant_id": OTHER_TENANT_ID, "election_event_id": EVENT_ID, "event_processor": "CREATE_REPORT", "event_payload": {}}, [], BACKEND),
        // The other processors are not authorized, and only return an id.
        case!(Admin, "/scheduled-event", {"tenant_id": OTHER_TENANT_ID, "event_processor": "START_VOTING_PERIOD", "event_payload": {}}, [], BACKEND),
        case!(Admin, "/set-custom-url", {"origin": "https://origin.invalid", "redirect_to": "https://redirect.invalid", "dns_prefix": "test", "election_id": EVENT_ID, "key": "login"}, [ELECTION_EVENT_WRITE], BACKEND, FORBIDDEN),
        case!(Admin, "/set-role-permission", {"tenant_id": TENANT_ID, "role_id": "test-role", "permission_name": "test-permission"}, [USER_PERMISSION_WRITE, ROLE_WRITE], BACKEND_TEXT, UNAUTHORIZED),
        case!(Admin, "/set-user-role", {"tenant_id": TENANT_ID, "user_id": USER_ID, "role_id": "test-role"}, [USER_WRITE, ROLE_WRITE], BACKEND_TEXT, UNAUTHORIZED),
        // An empty permission list checked in the caller's own tenant.
        case!(Admin, "/set-voter-authentication", {"election_event_id": EVENT_ID, "enrollment": "enabled", "otp": "enabled"}, [], BACKEND),
        case!(Admin, "/submit-tally-resolution", {"election_event_id": EVENT_ID, "tally_session_id": "test-session", "resolutions": [{"contest_id": "test-contest", "selected_candidate_id": "test-candidate"}]}, [TALLY_RESOLUTION_SUBMIT], BACKEND, UNAUTHORIZED),
        case!(Gold, "/update-election-voting-status", {"election_event_id": EVENT_ID, "election_id": "test-election", "voting_status": "OPEN"}, [ELECTION_STATE_WRITE], BACKEND, UNAUTHORIZED),
        case!(Gold, "/update-event-voting-status", {"election_event_id": EVENT_ID, "voting_status": "OPEN"}, [ELECTION_STATE_WRITE], BACKEND, UNAUTHORIZED),
        case!(Admin, "/update-realm-attributes", {"election_event_id": EVENT_ID, "attributes": {}}, [KEYCLOAK_REALM_ATTRIBUTES_WRITE], BACKEND_TEXT, FORBIDDEN),
        case!(Admin, "/update-realm-password-policy", {"election_event_id": EVENT_ID, "minimum_length": 8, "maximum_length": 64, "include_uppercase": true, "include_lowercase": true, "include_digits": true, "include_special_characters": false}, [ELECTION_EVENT_WRITE], BACKEND, FORBIDDEN_JSON),
        case!(Admin, "/update-tally-ceremony", {"election_event_id": EVENT_ID, "tally_session_id": "test-session", "status": "IN_PROGRESS"}, [ADMIN_CEREMONY], BACKEND, UNAUTHORIZED_JSON),
        case!(Admin, "/upsert-area", {"name": "test-area", "election_event_id": UUID_EVENT_ID, "tenant_id": UUID_TENANT_ID, "area_contest_ids": []}, [AREA_CREATE], BACKEND, UNAUTHORIZED),
        case!(Admin, "/upsert-areas", {"election_event_id": EVENT_ID, "document_id": "test-document"}, [AREA_WRITE], BACKEND, UNAUTHORIZED),
        case!(Admin, "/verify-application", {"applicant_id": "test-applicant", "applicant_data": {}, "tenant_id": TENANT_ID, "election_event_id": EVENT_ID, "annotations": {}}, [SERVICE_ACCOUNT], BACKEND, UNAUTHORIZED_JSON),
    ]
}

fn guarded_routes() -> BTreeSet<&'static str> {
    include_str!("../fixtures/guarded-post-routes.txt")
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect()
}

#[rocket::async_test]
async fn minimum_permissions_reach_the_backend_and_one_fewer_is_denied() {
    let cases = cases();
    assert_eq!(
        cases.iter().map(|case| case.path).collect::<BTreeSet<_>>(),
        guarded_routes(),
        "every guarded route needs its permission cases"
    );
    if !is_isolated_child() {
        // Nothing is scripted, so every Keycloak call gets HTTP 500.
        let peer = http::HttpServer::start(vec![]);
        run_isolated(
            "request_boundaries::route_permissions::minimum_permissions_reach_the_backend_and_one_fewer_is_denied",
            &peer.url,
        );
        peer.finish();
        return;
    }
    let client = client().await;
    // Report every mismatch at once: each row is independent.
    let mut failures = vec![];
    for case in &cases {
        let answer = case.send(&client, &case.permissions).await;
        if answer != Ok(case.answer) {
            failures.push(format!(
                "{} {} with {:?}: {answer:?}, expected {:?}",
                case.path, case.body, case.permissions, case.answer
            ));
        }
        let Some(denied) = case.denied else {
            continue;
        };
        for removed in &case.permissions {
            let remaining: Vec<String> = case
                .permissions
                .iter()
                .filter(|permission| *permission != removed)
                .cloned()
                .collect();
            let answer = case.send(&client, &remaining).await;
            if answer != Ok(denied) {
                failures.push(format!(
                    "{} {} without {removed}: {answer:?}, expected {denied:?}",
                    case.path, case.body
                ));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[rocket::async_test]
async fn gold_routes_refuse_complete_permissions_without_a_fresh_gold_login() {
    use Permissions::{ELECTION_STATE_WRITE, GOLD, PUBLISH_WRITE};
    let client = client().await;
    let now = Utc::now().timestamp();
    let refusal = "Insufficient privileges";
    for (path, body, permission, expected) in [
        (
            "/generate-ballot-publication",
            json!({"election_event_id": EVENT_ID}),
            PUBLISH_WRITE,
            json!({"message": refusal, "extensions": {"code": "Unauthorized"}}),
        ),
        (
            "/update-event-voting-status",
            json!({"election_event_id": EVENT_ID, "voting_status": "OPEN"}),
            ELECTION_STATE_WRITE,
            json!(refusal),
        ),
        (
            "/update-election-voting-status",
            json!({"election_event_id": EVENT_ID, "election_id": "test-election", "voting_status": "OPEN"}),
            ELECTION_STATE_WRITE,
            json!(refusal),
        ),
    ] {
        for claims in [
            Claims::new(TENANT_ID, USER_ID).auth_time(now),
            Claims::new(TENANT_ID, USER_ID)
                .acr(&GOLD.to_string())
                .auth_time(now - 3600),
        ] {
            let response = client
                .post(path)
                .header(ContentType::JSON)
                .header(bearer(&claims.roles([&permission])))
                .body(body.to_string())
                .dispatch()
                .await;
            assert_eq!(response.status(), Status::Forbidden, "{path}");
            let answer = match &expected {
                Value::String(_) => json!(response.into_string().await),
                _ => response.into_json::<Value>().await.unwrap(),
            };
            assert_eq!(answer, expected, "{path}");
        }
    }
}

#[rocket::async_test]
async fn secret_attributes_outside_an_election_event_are_refused_before_the_permission_check(
) {
    let client = client().await;
    let secrets = json!({"test-secret": ["value"]});
    for (path, body) in [
        ("/create-user", json!({"tenant_id": TENANT_ID, "user": {}})),
        (
            "/edit-user",
            json!({"tenant_id": TENANT_ID, "user_id": USER_ID}),
        ),
    ] {
        // The caller holds no permission at all: the 400 comes first.
        let mut with_secrets = body.clone();
        with_secrets["secret_attributes"] = secrets.clone();
        let response = client
            .post(path)
            .header(ContentType::JSON)
            .header(bearer(&Claims::new(TENANT_ID, USER_ID)))
            .body(with_secrets.to_string())
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::BadRequest, "{path}");
        assert_eq!(
            response.into_json::<Value>().await.unwrap(),
            json!({
                "message": "Encrypted attributes are only supported for election-event voters",
                "extensions": {"code": "UnknownError"}
            }),
            "{path}"
        );

        // An empty map changes no secret, so the permission check runs.
        let mut without_secrets = body;
        without_secrets["secret_attributes"] = json!({});
        let response = client
            .post(path)
            .header(ContentType::JSON)
            .header(bearer(&Claims::new(TENANT_ID, USER_ID)))
            .body(without_secrets.to_string())
            .dispatch()
            .await;
        assert_eq!(reply(&response), Ok(UNAUTHORIZED_JSON), "{path}");
    }
}
