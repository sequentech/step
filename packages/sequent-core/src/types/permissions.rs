// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
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
    #[strum(serialize = "election-event-archive")]
    ELECTION_EVENT_ARCHIVE,
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
    #[strum(serialize = "monitoring-dashboard-view-election-event")]
    MONITORING_DASHBOARD_VIEW_ELECTION_EVENT,
    #[strum(serialize = "monitoring-dashboard-view-election")]
    MONITORING_DASHBOARD_VIEW_ELECTION,
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
    #[strum(serialize = "datafix-account")]
    DATAFIX_ACCOUNT,
    #[strum(serialize = "contest-write")]
    CONTEST_WRITE,
    #[strum(serialize = "contest-read")]
    CONTEST_READ,
    #[strum(serialize = "contest-create")]
    CONTEST_CREATE,
    #[strum(serialize = "contest-delete")]
    CONTEST_DELETE,
    #[strum(serialize = "candidate-write")]
    CANDIDATE_WRITE,
    #[strum(serialize = "candidate-read")]
    CANDIDATE_READ,
    #[strum(serialize = "candidate-create")]
    CANDIDATE_CREATE,
    #[strum(serialize = "candidate-delete")]
    CANDIDATE_DELETE,
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
    #[strum(serialize = "election-create")]
    ELECTION_CREATE,
    #[strum(serialize = "election-read")]
    ELECTION_READ,
    #[strum(serialize = "election-write")]
    ELECTION_WRITE,
    #[strum(serialize = "election-delete")]
    ELECTION_DELETE,
    #[strum(serialize = "election-event-data-tab")]
    ELECTION_EVENT_DATA_TAB,
    #[strum(serialize = "election-event-voters-tab")]
    ELECTION_EVENT_VOTERS_TAB,
    #[strum(serialize = "election-event-areas-tab")]
    ELECTION_EVENT_AREAS_TAB,
    #[strum(serialize = "election-event-keys-tab")]
    ELECTION_EVENT_KEYS_TAB,
    #[strum(serialize = "election-event-tally-tab")]
    ELECTION_EVENT_TALLY_TAB,
    #[strum(serialize = "election-event-publish-tab")]
    ELECTION_EVENT_PUBLISH_TAB,
    #[strum(serialize = "election-event-tasks-tab")]
    ELECTION_EVENT_TASKS_TAB,
    #[strum(serialize = "election-event-logs-tab")]
    ELECTION_EVENT_LOGS_TAB,
    #[strum(serialize = "election-event-scheduled-tab")]
    ELECTION_EVENT_SCHEDULED_TAB,
    #[strum(serialize = "election-event-reports-tab")]
    ELECTION_EVENT_REPORTS_TAB,
    #[strum(serialize = "election-data-tab")]
    ELECTION_DATA_TAB,
    #[strum(serialize = "election-voters-tab")]
    ELECTION_VOTERS_TAB,
    #[strum(serialize = "election-publish-tab")]
    ELECTION_PUBLISH_TAB,
    #[strum(serialize = "users-menu")]
    USERS_MENU,
    #[strum(serialize = "settings-menu")]
    SETTINGS_MENU,
    #[strum(serialize = "templates-menu")]
    TEMPLATES_MENU,
    #[strum(serialize = "settings-election-types-tab")]
    SETTINGS_ELECTION_TYPES_TAB,
    #[strum(serialize = "settings-voting-channels-tab")]
    SETTINGS_VOTING_CHANNELS_TAB,
    #[strum(serialize = "settings-templates-tab")]
    SETTINGS_TEMPLATES_TAB,
    #[strum(serialize = "settings-languages-tab")]
    SETTINGS_LANGUAGES_TAB,
    #[strum(serialize = "settings-localization-tab")]
    SETTINGS_LOCALIZATION_TAB,
    #[strum(serialize = "settings-look-feel-tab")]
    SETTINGS_LOOK_FEEL_TAB,
    #[strum(serialize = "settings-trustees-tab")]
    SETTINGS_TRUSTEES_TAB,
    #[strum(serialize = "settings-countries-tab")]
    SETTINGS_COUNTRIES_TAB,
    #[strum(serialize = "application-read")]
    APPLICATION_READ,
    #[strum(serialize = "application-write")]
    APPLICATION_WRITE,
    #[strum(serialize = "ee-voters-columns")]
    EE_VOTERS_COLUMNS,
    #[strum(serialize = "ee-voters-filters")]
    EE_VOTERS_FILTERS,
    #[strum(serialize = "ee-voters-logs")]
    EE_VOTERS_LOGS,
    #[strum(serialize = "voter-import")]
    VOTER_IMPORT,
    #[strum(serialize = "voter-export")]
    VOTER_EXPORT,
    #[strum(serialize = "voter-delete")]
    VOTER_DELETE,
    #[strum(serialize = "voter-manually-verify")]
    VOTER_MANUALLY_VERIFY,
    #[strum(serialize = "voter-change-password")]
    VOTER_CHANGE_PASSWORD,
    #[strum(serialize = "election-event-localization-selector")]
    EE_LOCALIZATION_SELECTOR,
    #[strum(serialize = "localization-create")]
    LOCALIZATION_CREATE,
    #[strum(serialize = "localization-read")]
    LOCALIZATION_READ,
    #[strum(serialize = "localization-write")]
    LOCALIZATION_WRITE,
    #[strum(serialize = "localization-delete")]
    LOCALIZATION_DELETE,
    #[strum(serialize = "area-create")]
    AREA_CREATE,
    #[strum(serialize = "area-delete")]
    AREA_DELETE,
    #[strum(serialize = "area-export")]
    AREA_EXPORT,
    #[strum(serialize = "area-import")]
    AREA_IMPORT,
    #[strum(serialize = "area-upsert")]
    AREA_UPSERT,
    #[strum(serialize = "election-event-areas-columns")]
    EE_AREAS_COLUMNS,
    #[strum(serialize = "election-event-areas-filters")]
    EE_AREAS_FILTERS,
    #[strum(serialize = "election-event-tasks-back-button")]
    EE_TASKS_BACK_BUTTON,
    #[strum(serialize = "election-event-tasks-columns")]
    EE_TASKS_COLUMNS,
    #[strum(serialize = "election-event-tasks-filters")]
    EE_TASKS_FILTERS,
    #[strum(serialize = "task-export")]
    TASKS_EXPORT,
    #[strum(serialize = "logs-export")]
    LOGS_EXPORT,
    #[strum(serialize = "election-event-logs-columns")]
    EE_LOGS_COLUMNS,
    #[strum(serialize = "election-events-logs-filters")]
    EE_LOGS_FILTERS,
    #[strum(serialize = "election-event-scheduled-event-columns")]
    EE_SCHEDULED_EVENT_COLUMNS,
    #[strum(serialize = "scheduled-event-create")]
    SCHEDULED_EVENT_CREATE,
    #[strum(serialize = "scheduled-event-delete")]
    SCHEDULED_EVENT_DELETE,
    #[strum(serialize = "election-event-reports-columns")]
    EE_REPORTS_COLUMNS,
    #[strum(serialize = "report-create")]
    REPORT_CREATE,
    #[strum(serialize = "report-delete")]
    REPORT_DELETE,
    #[strum(serialize = "report-generate")]
    REPORT_GENERATE,
    #[strum(serialize = "report-preview")]
    REPORT_PREVIEW,
    #[strum(serialize = "transmission-report-generate")]
    TRANSMISSION_REPORT_GENERATE,
    #[strum(serialize = "monitor-authenticated-voters")]
    MONITOR_AUTHENTICATED_VOTERS,
    #[strum(serialize = "monitor-all-approve-disapprove-voters")]
    MONITOR_ALL_APPROVE_DISAPPROVE_VOTERS,
    #[strum(serialize = "monitor-automatic-approve-disapprove-voters")]
    MONITOR_AUTOMATIC_APPROVE_DISAPPROVE_VOTERS,
    #[strum(serialize = "monitor-manually-approve-disapprove-voters")]
    MONITOR_MANUALLY_APPROVE_DISAPPROVE_VOTERS,
    #[strum(serialize = "monitor-enrolled-overseas-voters")]
    MONITOR_ENROLLED_OVERSEAS_VOTERS,
    #[strum(serialize = "monitor-posts-already-closed-voting")]
    MONITOR_POSTS_ALREADY_CLOSED_VOTING,
    #[strum(serialize = "monitor-posts-already-generated-election-results")]
    MONITOR_POSTS_ALREADY_GENERATED_ELECTION_RESULTS,
    #[strum(serialize = "monitor-posts-already-opened-voting")]
    MONITOR_POSTS_ALREADY_OPENED_VOTING,
    #[strum(serialize = "monitor-posts-already-started-counting-votes")]
    MONITOR_POSTS_ALREADY_STARTED_COUNTING_VOTES,
    #[strum(serialize = "monitor-posts-initialized-the-system")]
    MONITOR_POSTS_INITIALIZED_THE_SYSTEM,
    #[strum(serialize = "monitor-posts-started-voting")]
    MONITOR_POSTS_STARTED_VOTING,
    #[strum(serialize = "monitor-posts-transmitted-results")]
    MONITOR_POSTS_TRANSMITTED_RESULTS,
    #[strum(serialize = "monitor-voters-voted-test-election")]
    MONITOR_VOTERS_VOTED_TEST_ELECTION,
    #[strum(serialize = "application-export")]
    APPLICATION_EXPORT,
    #[strum(serialize = "application-import")]
    APPLICATION_IMPORT,
    #[strum(serialize = "monitor-voters-who-voted")]
    MONITOR_VOTERS_WHO_VOTED,
    #[strum(serialize = "election-event-publish-preview")]
    EE_PUBLISH_PREVIEW,
    #[strum(serialize = "election-event-publish-back-button")]
    EE_PUBLISH_BACK_BUTTON,
    #[strum(serialize = "election-event-publish-columns")]
    EE_PUBLISH_COLUMNS,
    #[strum(serialize = "election-event-publish-filters")]
    EE_PUBLISH_FILTERS,
    #[strum(serialize = "publish-create")]
    PUBLISH_CREATE,
    #[strum(serialize = "publish-regenerate")]
    PUBLISH_REGENERATE,
    #[strum(serialize = "publish-export")]
    PUBLISH_EXPORT,
    #[strum(serialize = "publish-start-voting")]
    PUBLISH_START_VOTING,
    #[strum(serialize = "publish-pause-voting")]
    PUBLISH_PAUSE_VOTING,
    #[strum(serialize = "publish-stop-voting")]
    PUBLISH_STOP_VOTING,
    #[strum(serialize = "publish-changes")]
    PUBLISH_CHANGES,
    #[strum(serialize = "election-event-publish-view")]
    EE_PUBLISH_VIEW,
    #[strum(serialize = "election-event-keys-columns")]
    EE_KEYS_COLUMNS,
    #[strum(serialize = "create-ceremony")]
    CREATE_CEREMONY,
    #[strum(serialize = "export-ceremony")]
    EXPORT_CEREMONY,
    #[strum(serialize = "trustees-export")]
    TRUSTEES_EXPORT,
    #[strum(serialize = "election-event-tally-columns")]
    EE_TALLY_COLUMNS,
    #[strum(serialize = "election-event-tally-back-button")]
    EE_TALLY_BACK_BUTTON,
    #[strum(serialize = "transmition-ceremony")]
    TRANSMITION_CEREMONY,
    #[strum(serialize = "admin-ip-address-view")]
    ADMIN_IP_ADDRESS_VIEW,
    #[strum(serialize = "election-ip-address-view")]
    ELECTION_IP_ADDRESS_VIEW,
    #[strum(serialize = "election-dashboard-tab")]
    ELECTION_DASHBOARD_TAB,
    #[strum(serialize = "user-import")]
    USER_IMPORT,
    // permission to edit a voter that has already cast a vote
    #[strum(serialize = "voter-voted-edit")]
    VOTER_VOTED_EDIT,
    // permission to edit voter fields: email and mobile number
    #[strum(serialize = "voter-email-tlf-edit")]
    VOTER_EMAIL_TLF_EDIT,
    #[strum(serialize = "google-meet-link")]
    GOOGLE_MEET_LINK,
}

#[allow(non_camel_case_types)]
#[derive(
    Display, Serialize, Deserialize, Debug, PartialEq, Eq, Clone, EnumString,
)]
pub enum VoterPermissions {
    #[strum(serialize = "user")]
    CAST_VOTE,
}
