// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/// Commands for managing cast votes
pub mod cast_vote;
/// Commands for completing key ceremonies
pub mod complete_key_ceremony;
/// Commands for configuring the CLI
pub mod configure;
/// Commands for confirming the tally ceremony key
pub mod confirm_tally_ceremoney_key;
/// Commands for creating applications
pub mod create_applications;
/// Commands for creating areas
pub mod create_area;
/// Commands for creating area contests
pub mod create_area_contest;
/// Commands for creating candidates
pub mod create_candidate;
/// Commands for creating contests
pub mod create_contest;
/// Commands for creating elections
pub mod create_election;
/// Commands for creating election events
pub mod create_election_event;
/// Commands for creating electoral logs
pub mod create_electoral_logs;
/// Commands for creating voters
pub mod create_voter;
/// Commands for downloading tally results
pub mod download_tally_results;
/// Commands for duplicating votes
pub mod duplicate_votes;
/// Commands for exporting cast votes
pub mod export_cast_votes;
/// Commands for exporting election events
pub mod export_election_event;
/// Commands for generating preview URLs
pub mod generate_preview;
/// Commands for generating voters
pub mod generate_voters;
/// Commands for hashing passwords
pub mod hash_passwords;
/// Commands for importing election events
pub mod import_election_event;
/// Commands for publishing changes
pub mod publish_changes;
/// Commands for refreshing tokens
pub mod refresh_token;
/// Commands for rendering templates
pub mod render_template;
/// Commands for starting key ceremonies
pub mod start_key_ceremony;
/// Commands for starting tally ceremonies
pub mod start_tally;
/// Commands for submitting tally resolutions
pub mod submit_tally_resolution;
/// Commands for updating election voting status
pub mod update_election_voting_status;
/// Commands for updating event voting status
pub mod update_event_voting_status;
/// Commands for updating tally status
pub mod update_tally_status;
/// Commands for updating voters
pub mod update_voter;
