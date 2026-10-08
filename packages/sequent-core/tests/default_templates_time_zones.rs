// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The default report templates print times in the event's or election's
//! zone with its labels, never in the server's clock zone (VOTE-LIFECYCLE
//! design §7). Renders every default user template with its preview data
//! while the process runs in an unusual zone.
#![cfg(feature = "reports")]

use chrono::{DateTime, Utc};
use sequent_core::ballot::{
    ElectionEventLanguageConf, ElectionEventPresentation,
    ElectionEventTimeZones, ElectionPresentation, LogTimeZonePolicy,
};
use sequent_core::services::reports::{
    format_in_zone, render_template_text, template_time_variables,
    TimeZoneTexts,
};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// +12:45 / +13:45, labels CHAST/CHADT: nothing an event here configures.
const SERVER_ZONE: &str = "Pacific/Chatham";
const SERVER_ZONE_MARKERS: [&str; 6] = [
    "+12:45",
    "+13:45",
    "GMT+12:45",
    "GMT+13:45",
    "CHAST",
    "CHADT",
];

fn public_assets() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../.devcontainer/minio/public-assets")
}

fn i18n_defaults() -> Value {
    serde_json::from_str(
        &fs::read_to_string(public_assets().join("i18n_defaults.json"))
            .unwrap(),
    )
    .unwrap()
}

/// The default templates this stream owns: every `*_user.hbs` except the
/// Initialization Report's and the activity log's, plus velvet's own.
fn default_templates() -> Vec<(String, PathBuf, Option<PathBuf>)> {
    let mut templates: Vec<(String, PathBuf, Option<PathBuf>)> =
        fs::read_dir(public_assets())
            .unwrap()
            .filter_map(|entry| {
                let path = entry.unwrap().path();
                let name = path.file_name()?.to_str()?.to_string();
                let base = name.strip_suffix("_user.hbs")?.to_string();
                // The Initialization Report's belongs to VOTE-LIFECYCLE stream
                // I and the activity log's to stream E.
                if base.starts_with("initialization_report")
                    || base == "activity_logs"
                {
                    return None;
                }
                let data = public_assets().join(format!("{base}.json"));
                Some((base, path, data.exists().then_some(data)))
            })
            .collect();
    templates.push((
        "velvet ballot_images".to_string(),
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../velvet/src/resources/ballot_images_user.hbs"),
        None,
    ));
    templates.sort();
    templates
}

struct Configuration {
    event: ElectionEventPresentation,
    office: ElectionPresentation,
}

fn configuration(
    language: &str,
    primary: &str,
    office_zone: &str,
    logs: LogTimeZonePolicy,
    i18n: &[(&str, &str)],
) -> Configuration {
    let mut event = ElectionEventPresentation::default();
    event.timezones = Some(ElectionEventTimeZones {
        configured: vec![primary.to_string(), office_zone.to_string()],
        primary: primary.to_string(),
        logs,
    });
    event.language_conf = Some(ElectionEventLanguageConf {
        default_language_code: Some(language.to_string()),
        ..Default::default()
    });
    event.i18n = Some(HashMap::from([(
        language.to_string(),
        i18n.iter()
            .map(|(key, value)| (key.to_string(), Some(value.to_string())))
            .collect(),
    )]));
    let mut office = ElectionPresentation::default();
    office.timezone = Some(office_zone.to_string());
    Configuration { event, office }
}

/// The COMELEC preset (Manila primary, logs PRIMARY), with the label the
/// ticket's D3 discusses overridden for reports.
fn comelec_preset() -> Configuration {
    configuration(
        "en",
        "Asia/Manila",
        "Asia/Dubai",
        LogTimeZonePolicy::PRIMARY,
        &[("global:timezones.abbr.Asia/Manila", "PHT")],
    )
}

/// The Madrid association (Madrid primary, a Canary Islands office).
fn madrid_association() -> Configuration {
    configuration(
        "es",
        "Europe/Madrid",
        "Atlantic/Canary",
        LogTimeZonePolicy::ELECTION,
        &[],
    )
}

fn variables_for(
    configuration: &Configuration,
    with_election: bool,
    data: Option<&PathBuf>,
) -> Map<String, Value> {
    let mut variables = match data {
        Some(path) => match serde_json::from_str::<Value>(
            &fs::read_to_string(path).unwrap(),
        ) {
            Ok(Value::Object(map)) => map,
            _ => Map::new(),
        },
        None => Map::new(),
    };
    variables.extend(template_time_variables(
        Some(&configuration.event),
        with_election.then_some(&configuration.office),
        &i18n_defaults(),
    ));
    variables
}

#[test]
fn no_default_template_prints_the_server_zone() {
    std::env::set_var("TZ", SERVER_ZONE);
    let mut labels_printed = 0;

    for (name, template_path, data) in default_templates() {
        let template = fs::read_to_string(&template_path).unwrap();
        assert!(
            !template.contains("with_timezone"),
            "{name} still names a fixed zone"
        );
        assert!(!template.contains("%:z"), "{name} prints a raw offset");

        // Unused and unbalanced since before timezones (a stray `{{/if}}`):
        // handlebars refuses it whatever the data.
        if name == "velvet_vote_receipt" {
            continue;
        }

        for configuration in [comelec_preset(), madrid_association()] {
            for with_election in [false, true] {
                let variables =
                    variables_for(&configuration, with_election, data.as_ref());
                let texts = TimeZoneTexts::from_variables(&variables);
                let zone =
                    variables["electionTimezone"].as_str().unwrap().to_string();
                let rendered = render_template_text(&template, variables)
                    .unwrap_or_else(|error| {
                        panic!("{name} does not render: {error:?}")
                    });

                for marker in SERVER_ZONE_MARKERS {
                    assert!(
                        !rendered.contains(marker),
                        "{name} prints the server's zone ({marker})"
                    );
                }
                for at in ["2026-01-15T12:00:00Z", "2026-07-15T12:00:00Z"] {
                    let at: DateTime<Utc> = at.parse().unwrap();
                    let label = texts.label(&zone, at);
                    labels_printed +=
                        rendered.matches(&format!(" {label}")).count();
                }
                assert!(
                    !rendered.contains(" UTC"),
                    "{name} prints UTC while the event is configured without it"
                );
            }
        }
    }

    assert!(labels_printed > 0, "no template printed a zone label");
}

#[test]
fn an_override_of_the_manila_label_reaches_the_rendered_receipt() {
    let template =
        fs::read_to_string(public_assets().join("ballot_receipt_user.hbs"))
            .unwrap();
    let data = public_assets().join("ballot_receipt.json");
    let configuration = comelec_preset();
    // A Post without its own zone uses the primary.
    let variables = variables_for(&configuration, false, Some(&data));
    let timestamp: DateTime<Utc> =
        variables["timestamp"].as_str().unwrap().parse().unwrap();

    let rendered = render_template_text(&template, variables).unwrap();

    assert!(
        rendered.contains(&format!(
            "{} PHT",
            format_in_zone(timestamp, "Asia/Manila", "%B %d, %Y %H:%M")
        )),
        "{rendered}"
    );
    assert!(!rendered.contains("PhST"));
}

#[test]
fn the_receipt_prints_the_post_zone_of_each_configuration() {
    let template =
        fs::read_to_string(public_assets().join("ballot_receipt_user.hbs"))
            .unwrap();
    let data = public_assets().join("ballot_receipt.json");
    for configuration in [comelec_preset(), madrid_association()] {
        let variables = variables_for(&configuration, true, Some(&data));
        let office_zone = configuration.office.timezone.clone().unwrap();
        let timestamp: DateTime<Utc> =
            variables["timestamp"].as_str().unwrap().parse().unwrap();
        let label = TimeZoneTexts::from_variables(&variables)
            .label(&office_zone, timestamp);

        let rendered = render_template_text(&template, variables).unwrap();

        assert!(
            rendered.contains(&format!(
                "{} {label}",
                format_in_zone(timestamp, &office_zone, "%B %d, %Y %H:%M")
            )),
            "{office_zone}"
        );
    }
}

/// The acceptance email of `i18n_defaults.json`, with a voter's election as
/// notifications pass it (windmill `NotificationTimeContext`).
fn render_acceptance_email(
    configuration: &Configuration,
    with_election: bool,
) -> String {
    let defaults = i18n_defaults();
    let template = defaults["en"]["application"]["accepted"]["email"]
        ["plaintext_body"]
        .as_str()
        .unwrap()
        .to_string();
    let mut variables = variables_for(configuration, with_election, None);
    if with_election {
        variables.insert(
            "election".to_string(),
            serde_json::json!({
                "id": "office",
                "name": "Office",
                "timezone": configuration.office.timezone,
                "opening": "2028-04-08T20:00:00Z",
                "close": "2028-05-08T11:00:00Z",
            }),
        );
    }
    render_template_text(&template, variables).unwrap()
}

#[test]
fn the_default_acceptance_email_names_the_opening_and_close_in_both_zones() {
    let comelec = render_acceptance_email(&comelec_preset(), true);
    assert!(
        comelec.contains("You are registered for Office."),
        "{comelec}"
    );
    assert!(
        comelec
            .contains("Voting opens: 9 April 2028, 00:00 Gulf Standard Time"),
        "{comelec}"
    );
    assert!(
        comelec.contains(
            "Voting closes: 8 May 2028, 19:00 Philippine Standard Time (15:00 Gulf Standard Time)"
        ),
        "{comelec}"
    );

    let madrid = madrid_association();
    let variables = variables_for(&madrid, true, None);
    let texts = TimeZoneTexts::from_variables(&variables);
    let office_zone = madrid.office.timezone.clone().unwrap();
    let close: DateTime<Utc> = "2028-05-08T11:00:00Z".parse().unwrap();
    let rendered = render_acceptance_email(&madrid, true);
    assert!(
        rendered.contains(&format!(
            "({} {})",
            format_in_zone(close, &office_zone, "%H:%M"),
            texts.name(&office_zone, close)
        )),
        "{rendered}"
    );

    let without_election = render_acceptance_email(&comelec_preset(), false);
    assert!(
        !without_election.contains("Voting opens"),
        "{without_election}"
    );
}
