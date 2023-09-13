// SPDX-FileCopyrightText: 2022 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::ballot;
use crate::hasura_types;

fn create_ballot_style(
    election_event: hasura_types::ElectionEvent,
    elections: Vec<hasura_types::Election>,
    contests: Vec<hasura_types::Contest>,
    candidates: Vec<hasura_types::Candidate>,
) -> ballot::ElectionDTO {
    ballot::ElectionDTO {
        id: election_event.id.clone(),
        configuration: ballot::ElectionConfig {
            id: election_event.id.clone(),
            layout: "simultaneous-questions".to_string(),
            director: "protocol-manager".to_string(),
            authorities: vec![],
            title: election_event.name.clone(),
            description: election_event
                .description
                .clone()
                .unwrap_or("".to_string()),
            questions: elections
                .into_iter()
                .map(|election| {
                    let election_contests = contests
                        .clone()
                        .into_iter()
                        .filter(|c| c.election_id == election.id)
                        .collect::<Vec<hasura_types::Contest>>();
                    let election_contest_ids = election_contests
                        .iter()
                        .map(|c| c.id.clone())
                        .collect::<Vec<hasura_types::Uuid>>();
                    let election_candidates = candidates
                        .clone()
                        .into_iter()
                        .filter(|c| {
                            election_contest_ids.contains(&c.contest_id)
                        })
                        .collect::<Vec<hasura_types::Candidate>>();
                    create_question(
                        election,
                        election_contests,
                        election_candidates,
                    )
                })
                .collect(),
            start_date: None,
            end_date: None,
            presentation: ballot::ElectionPresentation {
                share_text: None,
                theme: "default".to_string(),
                urls: vec![],
                theme_css: "default".to_string(),
                extra_options: None,
                show_login_link_on_home: None,
                election_board_ceremony: None,
                conditional_questions: None,
                pdf_url: None,
                anchor_continue_btn_to_bottom: None,
                i18n_override: None,
            },
            extra_data: None,
            tallyPipesConfig: None,
            ballotBoxesResultsConfig: None,
            r#virtual: false,
            tally_allowed: false,
            publicCandidates: false,
            segmentedMixing: None,
            virtualSubelections: None,
            mixingCategorySegmentation: None,
            logo_url: None,
        },
        state: "started".to_string(),
        startDate: None,
        endDate: None,
        pks: None,
        tallyPipesConfig: None,
        ballotBoxesResultsConfig: None,
        results: None,
        resultsUpdated: None,
        r#virtual: false,
        tallyAllowed: false,
        publicCandidates: true,
        logo_url: None,
        trusteeKeysState: vec![],
        segmentedMixing: None,
    }
}

fn create_question(
    election: hasura_types::Election,
    contests: Vec<hasura_types::Contest>,
    candidates: Vec<hasura_types::Candidate>,
) -> ballot::Question {
    ballot::Question {
        id: election.id,
        description: election.description.unwrap_or("".to_string()),
        layout: "".to_string(),
        max: 0,
        min: 0,
        num_winners: 0,
        title: election.name,
        tally_type: "".to_string(),
        answer_total_votes_percentage: "".to_string(),
        answers: vec![],
        extra_options: None,
    }
}
