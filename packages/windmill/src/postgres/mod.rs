// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

pub mod application;
pub mod area;
pub mod area_contest;
pub mod ballot_publication;
pub mod ballot_style;
pub mod candidate;
pub mod cast_vote;
pub mod certificate_authority;
pub mod contest;
pub mod document;
pub mod election;
pub mod election_event;
pub mod keycloak_realm;
pub mod keys_ceremony;
pub mod lock;
pub mod maintenance;
pub mod phone_blacklist;
pub mod preview;
pub mod render_report;
pub mod reports;
pub mod results_area_contest;
pub mod results_area_contest_candidate;
pub mod results_contest;
pub mod results_contest_candidate;
pub mod results_election;
pub mod results_election_area;
pub mod results_event;
pub mod scheduled_event;
pub mod secret;
pub mod tally_results_publication;
pub mod tally_session;
pub mod tally_session_contest;
pub mod tally_session_execution;
pub mod tally_session_resolution;
pub mod tally_sheet;
pub mod tally_sheet_import;
pub mod tasks_execution;
pub mod template;
pub mod tenant;
pub mod trustee;

/// Keeps the rows that convert and logs the ones that do not, so that one
/// unreadable row does not hide the others in a query that spans every tenant.
pub fn skip_unparseable_rows<T>(rows: impl IntoIterator<Item = anyhow::Result<T>>) -> Vec<T> {
    rows.into_iter()
        .filter_map(|row| row.map_err(|err| tracing::error!("{err:#}")).ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::anyhow;

    #[test]
    fn skip_unparseable_rows_keeps_the_rows_that_convert() {
        let rows = vec![
            Ok("first"),
            Err(anyhow!("Error mapping \"FOO\" into an EventProcessor")),
            Ok("third"),
            Err(anyhow!("error deserializing encryption_policy")),
            Ok("last"),
        ];
        assert_eq!(skip_unparseable_rows(rows), ["first", "third", "last"]);
    }

    #[test]
    fn skip_unparseable_rows_with_no_rows() {
        let rows: Vec<anyhow::Result<u8>> = vec![];
        assert!(skip_unparseable_rows(rows).is_empty());
    }
}
