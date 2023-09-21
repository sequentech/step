// SPDX-FileCopyrightText: 2022 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::ballot;
use crate::hasura_types;

pub fn create_ballot_style(
    election_event: hasura_types::ElectionEvent, // Election
    election: hasura_types::Election,            // Election
    contests: Vec<hasura_types::Contest>,        // Question
    candidates: Vec<hasura_types::Candidate>,    // Answer
) -> ballot::ElectionDTO {
    ballot::ElectionDTO {
        id: election.id.clone(),
        configuration: ballot::ElectionConfig {
            id: election.id.clone(),
            layout: "simultaneous-questions".to_string(),
            director: "protocol-manager".to_string(),
            authorities: vec![],
            title: election.name.clone(),
            description: election.description.clone().unwrap_or("".to_string()),
            questions: contests
                .into_iter()
                .map(|contest| {
                    let election_candidates = candidates
                        .clone()
                        .into_iter()
                        .filter(|c| c.contest_id == contest.id)
                        .collect::<Vec<hasura_types::Candidate>>();
                    create_question(contest, election_candidates)
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
    contest: hasura_types::Contest,
    candidates: Vec<hasura_types::Candidate>,
) -> ballot::Question {
    ballot::Question {
        id: contest.id,
        description: contest.description.unwrap_or("".to_string()),
        layout: "".to_string(),
        max: contest.max_votes.unwrap_or(0),
        min: contest.min_votes.unwrap_or(0),
        num_winners: 1,
        title: contest.name,
        tally_type: contest.counting_algorithm.unwrap_or("".to_string()),
        answer_total_votes_percentage: "".to_string(),
        answers: candidates
            .iter()
            .enumerate()
            .map(|(i, candidate)| ballot::Answer {
                id: candidate.id.clone(),
                category: candidate.r#type.clone().unwrap_or("".to_string()),
                details: candidate
                    .description
                    .clone()
                    .unwrap_or("".to_string()),
                sort_order: i as i64,
                urls: vec![],
                text: candidate.name.clone(),
            })
            .collect(),
        extra_options: None,
    }
}
