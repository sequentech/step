// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;

#[test]
fn results_publication_action_is_a_structured_user_event() {
    let body = StatementBody::ResultsPublicationAction(ResultsPublicationDetails {
        publication_id: ResultsPublicationIdString("publication-id".to_string()),
        action: ResultsPublicationAction::Publish,
        route_scope: ResultsPublicationRouteScopeString("election".to_string()),
        route_election_id: ElectionIdString(Some("election-id".to_string())),
        access: ResultsPublicationAccessString("public".to_string()),
        visibility_scope: ResultsPublicationVisibilityScopeString("full_event".to_string()),
        contest_ids: vec![
            ContestIdString("contest-1".to_string()),
            ContestIdString("contest-2".to_string()),
        ],
    });

    let head = StatementHead::from_body(EventIdString("event-id".to_string()), &body);

    assert!(matches!(head.kind, StatementType::ResultsPublicationAction));
    assert!(matches!(head.event_type, StatementEventType::USER));
    assert_eq!(
        head.description,
        "Results publication publication-id published for election route (election-id), with public access, full_event visibility, and 2 contests."
    );
}
