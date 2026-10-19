// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The default Transmission Report renders what the report's data holds: one
//! page per area with its Post, country and polling station, its counts, the
//! status of each transmission destination and its SBEI members, also when a
//! destination has not been sent the ER yet (no dates).

use sequent_core::ballot::StringifiedPeriodDates;
use sequent_core::services::reports::render_template_text;
use serde_json::Value;
use std::path::Path;
use windmill::services::reports::report_variables::{ExecutionAnnotations, InspectorData};
use windmill::services::reports::transmission_report::{UserData, UserDataArea};
use windmill::services::transmission::ServerData;

fn user_template() -> String {
    std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../.devcontainer/minio/public-assets/transmission_report_user.hbs"),
    )
    .unwrap()
}

fn destination(transmitted: bool) -> ServerData {
    let (transmitted, received, date) = if transmitted {
        ("Transmitted", "Received", "2028-05-09T11:05:00Z")
    } else {
        ("Not Transmitted", "Not Received", "")
    };
    ServerData {
        server_code: "ccs-manila".to_string(),
        transmitted: transmitted.to_string(),
        server_name: "CCS Manila (SBOC)".to_string(),
        received: received.to_string(),
        date_transmitted: date.to_string(),
        date_received: date.to_string(),
    }
}

fn data(transmitted: bool) -> UserData {
    UserData {
        areas: vec![UserDataArea {
            election_title: "2028 Overseas Voting".to_string(),
            geographical_region: "EUROPE".to_string(),
            post: "MADRID PE".to_string(),
            country: "Spain".to_string(),
            voting_center: "MADRID PE".to_string(),
            station_id: "90010001".to_string(),
            station_name: "SPAIN".to_string(),
            registered_voters: Some(1250),
            ballots_counted: Some(5),
            voters_turnout: Some(0.4),
            servers: vec![destination(transmitted)],
            inspectors: vec![
                InspectorData {
                    role: "Chairperson".to_string(),
                    name: "Maria L. Santos".to_string(),
                },
                InspectorData {
                    role: "Poll Clerk".to_string(),
                    name: "Jose R. Dela Cruz".to_string(),
                },
            ],
        }],
        election_dates: StringifiedPeriodDates::default(),
        execution_annotations: ExecutionAnnotations {
            date_printed: "2028-05-09T12:00:00Z".to_string(),
            report_hash: "report-hash".to_string(),
            app_version: "1.2.3".to_string(),
            software_version: "1.2.3".to_string(),
            app_hash: "app-hash".to_string(),
            executer_username: Some("sbei".to_string()),
            results_hash: Some("results-hash".to_string()),
        },
    }
}

fn render(transmitted: bool) -> String {
    let Value::Object(variables) = serde_json::to_value(data(transmitted)).unwrap() else {
        panic!("the report's data is an object");
    };
    let rendered = render_template_text(&user_template(), variables).unwrap();
    rendered.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn assert_has(text: &str, expected: &[&str]) {
    for line in expected {
        assert!(text.contains(line), "{line:?} not in {text}");
    }
}

#[test]
fn an_areas_page_names_where_it_was_counted_and_its_counts() {
    assert_has(
        &render(false),
        &[
            "TRANSMISSION REPORT",
            "2028 Overseas Voting",
            "Geographical Region:</strong> </td> <td class=\"value\"> EUROPE",
            "Post:</strong> </td> <td class=\"value\"> MADRID PE",
            "Country:</strong> </td> <td class=\"value\"> Spain",
            "Voting Center:</strong> </td> <td class=\"value\"> MADRID PE",
            "Polling Station ID:</strong> </td> <td class=\"value\"> 90010001",
            "Polling Station:</strong> </td> <td class=\"value\"> SPAIN",
            "Number of Registered Voters:</strong> </td> <td class=\"value\"> 1,250",
            "Number of Ballots Counted:</strong> </td> <td class=\"value\"> 5",
            "Voters Turnout (%):</strong> </td> <td class=\"value\">0.40%",
            "Results hash: results-hash",
            "Report hash: report-hash",
            "Software version: 1.2.3",
            "System hash: app-hash",
            "End of Report",
        ],
    );
}

#[test]
fn a_destination_not_yet_sent_the_er_has_no_dates() {
    let text = render(false);
    assert_has(
        &text,
        &[
            "Server name:</strong> </td> <td class=\"value servers-value\"> CCS Manila (SBOC) ccs-manila",
            "Transmission status:</strong> </td> <td class=\"value servers-value\"> Not Transmitted </td>",
            "Received status:</strong> </td> <td class=\"value servers-value\"> Not Received </td>",
        ],
    );
}

#[test]
fn a_destination_sent_the_er_prints_when_it_was_sent_and_received() {
    let text = render(true);
    assert_has(
        &text,
        &[
            "Transmission status:</strong> </td> <td class=\"value servers-value\"> Transmitted / May 09, 2028 11:05",
            "Received status:</strong> </td> <td class=\"value servers-value\"> Received / May 09, 2028 11:05",
        ],
    );
}

#[test]
fn the_sbei_members_sign_by_name_and_role() {
    assert_has(
        &render(false),
        &[
            "The Special Board of Election Inspectors",
            "Maria L. Santos</strong> <br />Chairperson",
            "Jose R. Dela Cruz</strong> <br />Poll Clerk",
            "Watchers:",
        ],
    );
}
