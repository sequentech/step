// SPDX-FileCopyrightText: 2022 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};

#[allow(non_camel_case_types)]
#[derive(
    Display, Serialize, Deserialize, Debug, PartialEq, Eq, Clone, EnumString,
)]
pub enum Permissions {
    #[strum(serialize = "admin-user")]
    ADMIN_USER,
    #[strum(serialize = "tenant-create")]
    TENANT_CREATE,
    #[strum(serialize = "tenant-read")]
    TENANT_READ,
    #[strum(serialize = "tenant-write")]
    TENANT_WRITE,
    #[strum(serialize = "election-event-create")]
    ELECTION_EVENT_CREATE,
    #[strum(serialize = "election-event-read")]
    ELECTION_EVENT_READ,
    #[strum(serialize = "election-event-write")]
    ELECTION_EVENT_WRITE,
    #[strum(serialize = "election-event-delete")]
    ELECTION_EVENT_DELETE,
    #[strum(serialize = "voter-create")]
    VOTER_CREATE,
    #[strum(serialize = "voter-read")]
    VOTER_READ,
    #[strum(serialize = "voter-write")]
    VOTER_WRITE,
    #[strum(serialize = "user-create")]
    USER_CREATE,
    #[strum(serialize = "user-read")]
    USER_READ,
    #[strum(serialize = "user-write")]
    USER_WRITE,
    #[strum(serialize = "user-permission-create")]
    USER_PERMISSION_CREATE,
    #[strum(serialize = "user-permission-read")]
    USER_PERMISSION_READ,
    #[strum(serialize = "user-permission-write")]
    USER_PERMISSION_WRITE,
    #[strum(serialize = "role-create")]
    ROLE_CREATE,
    #[strum(serialize = "role-read")]
    ROLE_READ,
    #[strum(serialize = "role-write")]
    ROLE_WRITE,
    #[strum(serialize = "role-assign")]
    ROLE_ASSIGN,
    #[strum(serialize = "communication-template-create")]
    TEMPLATE_CREATE,
    #[strum(serialize = "communication-template-read")]
    TEMPLATE_READ,
    #[strum(serialize = "communication-template-write")]
    TEMPLATE_WRITE,
    #[strum(serialize = "notification-read")]
    NOTIFICATION_READ,
    #[strum(serialize = "notification-write")]
    NOTIFICATION_WRITE,
    #[strum(serialize = "notification-send")]
    NOTIFICATION_SEND,
    #[strum(serialize = "area-read")]
    AREA_READ,
    #[strum(serialize = "area-write")]
    AREA_WRITE,
    #[strum(serialize = "election-state-write")]
    ELECTION_STATE_WRITE,
    #[strum(serialize = "election-type-create")]
    ELECTION_TYPE_CREATE,
    #[strum(serialize = "election-type-read")]
    ELECTION_TYPE_READ,
    #[strum(serialize = "election-type-write")]
    ELECTION_TYPE_WRITE,
    #[strum(serialize = "voting-channel-read")]
    VOTING_CHANNEL_READ,
    #[strum(serialize = "voting-channel-write")]
    VOTING_CHANNEL_WRITE,
    #[strum(serialize = "trustee-create")]
    TRUSTEE_CREATE,
    #[strum(serialize = "trustee-read")]
    TRUSTEE_READ,
    #[strum(serialize = "trustee-write")]
    TRUSTEE_WRITE,
    #[strum(serialize = "tally-read")]
    TALLY_READ,
    #[strum(serialize = "trustee-ceremony")]
    TRUSTEE_CEREMONY,
    #[strum(serialize = "tally-start")]
    TALLY_START,
    #[strum(serialize = "tally-write")]
    TALLY_WRITE,
    #[strum(serialize = "tally-results-read")]
    TALLY_RESULTS_READ,
    #[strum(serialize = "publish-read")]
    PUBLISH_READ,
    #[strum(serialize = "publish-write")]
    PUBLISH_WRITE,
    #[strum(serialize = "logs-read")]
    LOGS_READ,
    #[strum(serialize = "tasks-read")]
    TASKS_READ,
    #[strum(serialize = "keys-read")]
    KEYS_READ,
    #[strum(serialize = "document-upload")]
    DOCUMENT_UPLOAD,
    #[strum(serialize = "document-download")]
    DOCUMENT_DOWNLOAD,
    #[strum(serialize = "admin-ceremony")]
    ADMIN_CEREMONY,
    #[strum(serialize = "admin-dashboard-view")]
    ADMIN_DASHBOARD_VIEW,
    #[strum(serialize = "tally-sheet-view")]
    TALLY_SHEET_VIEW,
    #[strum(serialize = "tally-sheet-create")]
    TALLY_SHEET_CREATE,
    #[strum(serialize = "tally-sheet-publish")]
    TALLY_SHEET_PUBLISH,
    #[strum(serialize = "tally-sheet-delete")]
    TALLY_SHEET_DELETE,
    #[strum(serialize = "cast-vote-read")]
    CAST_VOTE_READ,
    #[strum(serialize = "document-read")]
    DOCUMENT_READ,
    #[strum(serialize = "document-write")]
    DOCUMENT_WRITE,
    #[strum(serialize = "support-material-read")]
    SUPPORT_MATERIAL_READ,
    #[strum(serialize = "support-material-write")]
    SUPPORT_MATERIAL_WRITE,
    #[strum(serialize = "miru-create")]
    MIRU_CREATE,
    #[strum(serialize = "miru-download")]
    MIRU_DOWNLOAD,
    #[strum(serialize = "miru-send")]
    MIRU_SEND,
    #[strum(serialize = "miru-sign")]
    MIRU_SIGN,
    #[strum(serialize = "service-account")]
    SERVICE_ACCOUNT,
    #[strum(serialize = "permission-label-write")]
    PERMISSION_LABEL_WRITE,
    #[strum(serialize = "scheduled-event-write")]
    SCHEDULED_EVENT_WRITE,
    #[strum(serialize = "report-write")]
    REPORT_WRITE,
    #[strum(serialize = "report-read")]
    REPORT_READ,
    #[strum(serialize = "cloudflare-write")]
    CLOUDFLARE_WRITE,
    #[strum(serialize = "gold")]
    GOLD,
    #[strum(serialize = "silver")]
    SILVER,
}

#[allow(non_camel_case_types)]
#[derive(
    Display, Serialize, Deserialize, Debug, PartialEq, Eq, Clone, EnumString,
)]
pub enum VoterPermissions {
    #[strum(serialize = "user")]
    CAST_VOTE,
}
