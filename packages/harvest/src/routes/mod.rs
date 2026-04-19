// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
/// Datafix API endpoints.
pub mod api_datafix;
/// Application verification and status management endpoints.
pub mod applications;
/// Area management endpoints.
pub mod areas;
/// Ballot publication and generation endpoints.
pub mod ballot_publication;
/// Ballot publication preview preparation endpoints.
pub mod ballot_publication_prepare_preview;
/// Ballot receipt creation endpoints.
pub mod create_ballot_receipt;
/// Custom URL management endpoints.
pub mod custom_urls;
/// Certificate authority deletion endpoints.
pub mod delete_certificate_authority;
/// Election event deletion endpoints.
pub mod delete_election_event;
/// Election dates management endpoints.
pub mod election_dates;
/// Election event statistics endpoints.
pub mod election_event_stats;
/// Per-election statistics (distinct voters, areas, votes per day).
pub mod election_stats;
/// Election endpoints.
pub mod elections;
/// Electoral log endpoints.
pub mod electoral_log;
/// Error catchers endpoints.
pub mod error_catchers;
/// Export application endpoints.
pub mod export_application;
/// Export ballot publication endpoints.
pub mod export_ballot_publication;
/// Export election event endpoints.
pub mod export_election_event;
/// Export election event logs endpoints.
pub mod export_election_event_logs;
/// Export tally results endpoints.
pub mod export_tally_results;
/// Export tasks execution endpoints.
pub mod export_tasks_execution;
pub mod export_template;
/// Export tenant config endpoints.
pub mod export_tenant_config;
/// Fetch document endpoints.
pub mod fetch_document;
/// Generate preview URL endpoints.
pub mod generate_preview_url;
/// Get certificate authorities PEM endpoints.
pub mod get_certificate_authorities_pem;
pub mod google_meet;
pub mod immudb_log_audit;
/// Import application endpoints.
pub mod import_application;
/// Import areas endpoints.
pub mod import_areas;
/// Import candidates endpoints.
pub mod import_candidates;
pub mod import_certificate_authority;
/// Import templates endpoints.
pub mod import_templates;
/// Import tenant config endpoints.
pub mod import_tenant_config;
/// Insert cast vote endpoints.
pub mod insert_cast_vote;
/// Insert election event endpoints.
pub mod insert_election_event;
/// Insert tenant endpoints.
pub mod insert_tenant;
/// Keys ceremony endpoints.
pub mod keys_ceremony;
pub mod limit_access_by_countries;
/// Miru plugin endpoints.
pub mod miru_plugin;
/// Permissions endpoints.
pub mod permissions;
/// Plugins endpoints.
pub mod plugins;
/// Reports endpoints.
pub mod reports;
/// Roles endpoints.
pub mod roles;
pub mod scheduled_event;
/// Voter authentication endpoints.
pub mod set_voter_authentication;
/// Tally ceremony endpoints.
pub mod tally_ceremony;
/// Tally sheets endpoints.
pub mod tally_sheets;
/// Templates endpoints.
pub mod templates;
/// Trustees endpoints.
pub mod trustees;
/// Upload document endpoints.
pub mod upload_document;
/// Users endpoints.
pub mod users;
/// Voter electoral log endpoints.
pub mod voter_electoral_log;
/// Voting status endpoints.
pub mod voting_status;
