// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The default Initialization Report prints when it was generated in the
//! Post's zone with the primary zone in brackets, and the voting period from
//! the Post's opening (its zone) to the common close (the primary zone)
//! (VOTE-LIFECYCLE design §7, §9). The zones come from the configuration
//! through the tally's report variables, never from the template.
//!
//! Uses the production report registry and shared timezone text variables,
//! so missing helpers or lost target zones fail this scoped test.

use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use chrono_tz::Tz;
use sequent_core::services::reports::render_template_text;
use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::path::Path;
use velvet::pipes::generate_reports::add_template_time_variables;
use windmill::services::ceremonies::velvet_tally::report_time_zones;

struct Configuration {
    primary: &'static str,
    /// The Post's own zone; `None` uses the primary.
    post: Option<&'static str>,
    /// The texts the event shows for each zone (`timezones.abbr.<zone>`).
    labels: &'static [(&'static str, &'static str)],
}

/// The COMELEC preset: a Post in Dubai, Manila primary.
const DUBAI_POST: Configuration = Configuration {
    primary: "Asia/Manila",
    post: Some("Asia/Dubai"),
    labels: &[("Asia/Manila", "PhST"), ("Asia/Dubai", "GMT+4")],
};

/// The Madrid association: its Canary Islands office, Madrid primary.
const CANARY_OFFICE: Configuration = Configuration {
    primary: "Europe/Madrid",
    post: Some("Atlantic/Canary"),
    labels: &[("Europe/Madrid", "CEST"), ("Atlantic/Canary", "WEST")],
};

/// A Post without its own zone.
const MADRID_OFFICE: Configuration = Configuration {
    primary: "Europe/Madrid",
    post: None,
    labels: &[("Europe/Madrid", "CEST"), ("Atlantic/Canary", "WEST")],
};

const POST_ID: &str = "post";
const GENERATED: &str = "2028-04-08T19:41:00Z";
const DATE_TIME: &str = "%B %d, %Y %H:%M";
const PERIOD: &str = "%d %B %Y %H:%M";

fn zone(name: &str) -> Tz {
    name.parse().unwrap()
}

fn label(configuration: &Configuration, zone: &str) -> String {
    configuration
        .labels
        .iter()
        .find(|(name, _)| *name == zone)
        .map(|(_, label)| label.to_string())
        .unwrap()
}

/// `instant` as `timezones.dateTimeZone` renders it in `zone`.
fn date_time_zone(
    configuration: &Configuration,
    instant: &str,
    zone_name: &str,
    format: &str,
) -> String {
    let instant = DateTime::parse_from_rfc3339(instant).unwrap();
    format!(
        "{} {}",
        instant.with_timezone(&zone(zone_name)).format(format),
        label(configuration, zone_name)
    )
}

/// The instant of a wall time in a zone.
fn instant_of(local: &str, zone_name: &str) -> String {
    let local = NaiveDateTime::parse_from_str(local, "%Y-%m-%dT%H:%M").unwrap();
    zone(zone_name)
        .from_local_datetime(&local)
        .single()
        .unwrap()
        .with_timezone(&Utc)
        .to_rfc3339()
}

fn post_zone(configuration: &Configuration) -> &'static str {
    configuration.post.unwrap_or(configuration.primary)
}

/// The report variables a tally of the configuration's Post produces.
fn variables(configuration: &Configuration, opening: &str, close: &str) -> Map<String, Value> {
    let event: sequent_core::types::hasura::core::ElectionEvent = serde_json::from_value(json!({
        "id": "event",
        "tenant_id": "tenant",
        "is_archived": false,
        "encryption_protocol": "RSA256",
        "presentation": {
            "timezones": {
                "configured": configuration
                    .labels
                    .iter()
                    .map(|(zone, _)| zone.to_string())
                    .collect::<Vec<_>>(),
                "primary": configuration.primary,
            },
        },
    }))
    .unwrap();
    let election: sequent_core::types::hasura::core::Election = serde_json::from_value(json!({
        "id": POST_ID,
        "tenant_id": "tenant",
        "election_event_id": "event",
        "presentation": { "timezone": configuration.post },
    }))
    .unwrap();
    let labels: Map<String, Value> = configuration
        .labels
        .iter()
        .map(|(zone, label)| (zone.to_string(), json!(label)))
        .collect();
    let defaults = json!({"en": {"timezones": {"abbr": labels}}});
    let (run_variables, election_zones) = report_time_zones(&event, &[election], &defaults);
    let mut variables = json!({
        "execution_annotations": {
            "date_printed": GENERATED,
            "results_hash": "report-hash",
        },
        "reports": [{
            "election_id": POST_ID,
            "election_name": "Post",
            "election_alias": "Event",
            "election_dates": {
                "scheduled_event_dates": {
                    "START_VOTING_PERIOD": { "scheduled_at": opening },
                    "END_VOTING_PERIOD": { "scheduled_at": close },
                },
            },
        }],
    })
    .as_object()
    .unwrap()
    .clone();
    add_template_time_variables(
        &mut variables,
        &run_variables,
        &election_zones,
        Some(POST_ID),
    );
    variables
}

fn render(configuration: &'static Configuration) -> (String, String, String) {
    let template = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../.devcontainer/minio/public-assets/initialization_report_user.hbs"),
    )
    .unwrap();
    // The Post opens at 00:00 in its zone; the common close is 19:00 in the
    // primary zone.
    let opening = instant_of("2028-04-09T00:00", post_zone(configuration));
    let close = instant_of("2028-05-08T19:00", configuration.primary);
    let rendered =
        render_template_text(&template, variables(configuration, &opening, &close)).unwrap();
    let text = rendered.split_whitespace().collect::<Vec<_>>().join(" ");
    (text, opening, close)
}

fn assert_prints_both_zones(configuration: &'static Configuration) {
    let (text, opening, close) = render(configuration);
    let post = post_zone(configuration);
    let generated = format!(
        "Generated {} ({})",
        date_time_zone(configuration, GENERATED, post, DATE_TIME),
        date_time_zone(configuration, GENERATED, configuration.primary, DATE_TIME),
    );
    assert!(text.contains(&generated), "{generated:?} not in {text}");
    let period = format!(
        "Voting Period: {} - {}",
        date_time_zone(configuration, &opening, post, PERIOD),
        date_time_zone(configuration, &close, configuration.primary, PERIOD),
    );
    assert!(text.contains(&period), "{period:?} not in {text}");
    assert!(text.contains("Report hash: report-hash"), "{text}");
}

#[test]
fn a_dubai_post_report_prints_its_zone_and_the_manila_primary() {
    assert_prints_both_zones(&DUBAI_POST);
    // As in the draft: 23:41 GMT+4 (03:41 the next day PhST).
    let (text, _, _) = render(&DUBAI_POST);
    assert!(text.contains("April 08, 2028 23:41 GMT+4 (April 09, 2028 03:41 PhST)"));
    assert!(text.contains("09 April 2028 00:00 GMT+4 - 08 May 2028 19:00 PhST"));
}

#[test]
fn a_canary_office_report_prints_its_zone_and_the_madrid_primary() {
    assert_prints_both_zones(&CANARY_OFFICE);
}

#[test]
fn a_post_in_the_primary_zone_prints_it_once() {
    let (text, _, _) = render(&MADRID_OFFICE);
    let generated = format!(
        "Generated {} ",
        date_time_zone(&MADRID_OFFICE, GENERATED, MADRID_OFFICE.primary, DATE_TIME)
    );
    assert!(text.contains(&generated), "{text}");
    assert!(!text.contains(&format!("{generated}(")), "{text}");
}

#[test]
fn reports_of_elections_without_a_zone_use_the_events() {
    let mut variables = Map::new();
    let run =
        json!({ "electionEventTimezone": "Europe/Madrid", "electionTimezone": "Europe/Madrid" })
            .as_object()
            .unwrap()
            .clone();
    add_template_time_variables(&mut variables, &run, &HashMap::new(), Some("other"));
    assert_eq!(variables["electionTimezone"], json!("Europe/Madrid"));
    // A report's own value wins.
    let mut variables = json!({ "electionTimezone": "UTC" })
        .as_object()
        .unwrap()
        .clone();
    add_template_time_variables(
        &mut variables,
        &run,
        &HashMap::from([("other".to_string(), "Atlantic/Canary".to_string())]),
        Some("other"),
    );
    assert_eq!(variables["electionTimezone"], json!("UTC"));
}

#[test]
fn the_post_zone_overrides_the_run_fallback_and_event_wide_reports_keep_primary() {
    let run =
        json!({"electionEventTimezone": "Europe/Madrid", "electionTimezone": "Europe/Madrid"})
            .as_object()
            .unwrap()
            .clone();
    let zones = HashMap::from([("post".to_string(), "Atlantic/Canary".to_string())]);
    let mut post = Map::new();
    add_template_time_variables(&mut post, &run, &zones, Some("post"));
    assert_eq!(post["electionTimezone"], json!("Atlantic/Canary"));
    let mut event = Map::new();
    add_template_time_variables(&mut event, &run, &zones, None);
    assert_eq!(event["electionTimezone"], json!("Europe/Madrid"));
}
