// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use crate::ballot::{
    ElectionEventLanguageConf, ElectionEventTimeZones, LogTimeZonePolicy,
};
use crate::services::reports::render_template_text;
use serde_json::json;

/// A slice of the generated `i18n_defaults.json`.
fn defaults() -> Value {
    json!({
        "en": {
            "timezones": {
                "dateTimeZone": "{{dateTime}} {{zone}}",
                "voterDateTimeZone": "{{dateTime}} {{zoneName}}",
                "abbr": {
                    "Asia/Manila": "PhST",
                    "Asia/Dubai": "GMT+4",
                    "America/New_York": "EST",
                    "Europe/Madrid": "CET",
                    "Atlantic/Canary": "WET",
                    "UTC": "UTC"
                },
                "abbrDaylight": {
                    "America/New_York": "EDT",
                    "Europe/Madrid": "CEST",
                    "Atlantic/Canary": "WEST"
                },
                "name": {
                    "Asia/Manila": "Philippine Standard Time",
                    "Asia/Dubai": "Gulf Standard Time",
                    "America/New_York": "Eastern Standard Time",
                    "Europe/Madrid": "Central European Standard Time",
                    "Atlantic/Canary": "Western European Standard Time"
                },
                "nameDaylight": {
                    "America/New_York": "Eastern Daylight Time",
                    "Europe/Madrid": "Central European Summer Time",
                    "Atlantic/Canary": "Western European Summer Time"
                }
            }
        },
        "es": {
            "timezones": {
                "abbr": {"Europe/Madrid": "CET"},
                "abbrDaylight": {"Europe/Madrid": "CEST"},
                "name": {"Europe/Madrid": "hora estándar de Europa central"},
                "nameDaylight": {"Europe/Madrid": "hora de verano de Europa central"}
            }
        }
    })
}

fn overrides(entries: &[(&str, &str)]) -> HashMap<String, Option<String>> {
    entries
        .iter()
        .map(|(key, value)| (key.to_string(), Some(value.to_string())))
        .collect()
}

fn texts(entries: &[(&str, &str)]) -> TimeZoneTexts {
    TimeZoneTexts::build(defaults().get("en"), Some(&overrides(entries)))
}

fn at(rfc3339: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(rfc3339)
        .unwrap()
        .with_timezone(&Utc)
}

/// The two configurations of design §10.
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
    event.i18n = Some(HashMap::from([(language.to_string(), overrides(i18n))]));
    let mut office = ElectionPresentation::default();
    office.timezone = Some(office_zone.to_string());
    Configuration { event, office }
}

fn comelec_preset(i18n: &[(&str, &str)]) -> Configuration {
    configuration(
        "en",
        "Asia/Manila",
        "Asia/Dubai",
        LogTimeZonePolicy::PRIMARY,
        i18n,
    )
}

fn madrid_association(i18n: &[(&str, &str)]) -> Configuration {
    configuration(
        "es",
        "Europe/Madrid",
        "Atlantic/Canary",
        LogTimeZonePolicy::ELECTION,
        i18n,
    )
}

fn render(template: &str, variables: Map<String, Value>) -> String {
    render_template_text(template, variables).expect("template renders")
}

#[test]
fn templates_scope_beats_unprefixed_beats_global_in_labels() {
    let at = at("2028-04-09T00:00:00Z");
    let all = texts(&[
        ("global:timezones.abbr.Asia/Manila", "G"),
        ("timezones.abbr.Asia/Manila", "L"),
        ("templates:timezones.abbr.Asia/Manila", "T"),
    ]);
    let legacy_and_global = texts(&[
        ("global:timezones.abbr.Asia/Manila", "G"),
        ("timezones.abbr.Asia/Manila", "L"),
    ]);
    let global_only = texts(&[("global:timezones.abbr.Asia/Manila", "G")]);
    let voting_portal_only =
        texts(&[("votingPortal:timezones.abbr.Asia/Manila", "VP")]);

    assert_eq!(all.label("Asia/Manila", at), "T");
    assert_eq!(legacy_and_global.label("Asia/Manila", at), "L");
    assert_eq!(global_only.label("Asia/Manila", at), "G");
    assert_eq!(voting_portal_only.label("Asia/Manila", at), "PhST");
}

#[test]
fn daylight_defaults_follow_the_instant_and_an_override_applies_all_year() {
    let winter = at("2028-01-15T12:00:00Z");
    let summer = at("2028-07-15T12:00:00Z");
    let defaults_only = texts(&[]);
    let overridden = texts(&[
        ("templates:timezones.abbr.America/New_York", "NYC"),
        ("templates:timezones.name.America/New_York", "New York time"),
    ]);

    assert_eq!(defaults_only.label("America/New_York", winter), "EST");
    assert_eq!(defaults_only.label("America/New_York", summer), "EDT");
    assert_eq!(
        defaults_only.name("America/New_York", summer),
        "Eastern Daylight Time"
    );
    assert_eq!(overridden.label("America/New_York", winter), "NYC");
    assert_eq!(overridden.label("America/New_York", summer), "NYC");
    assert_eq!(overridden.name("America/New_York", summer), "New York time");
}

#[test]
fn a_combined_override_that_drops_a_placeholder_falls_back_to_the_default() {
    let at = at("2028-04-08T16:00:00Z");
    let dropped_zone =
        texts(&[("templates:timezones.dateTimeZone", "{{dateTime}}")]);
    let dropped_date =
        texts(&[("global:timezones.voterDateTimeZone", "at {{zoneName}}")]);
    let reworded = texts(&[(
        "templates:timezones.dateTimeZone",
        "{{dateTime}} ({{zone}})",
    )]);

    assert_eq!(
        dropped_zone.date_time_zone(
            at,
            "Asia/Dubai",
            "%H:%M",
            DateTimeZoneStyle::Label
        ),
        "20:00 GMT+4"
    );
    assert_eq!(
        dropped_date.date_time_zone(
            at,
            "Asia/Dubai",
            "%H:%M",
            DateTimeZoneStyle::Voter
        ),
        "20:00 Gulf Standard Time"
    );
    assert_eq!(
        reworded.date_time_zone(
            at,
            "Asia/Dubai",
            "%H:%M",
            DateTimeZoneStyle::Label
        ),
        "20:00 (GMT+4)"
    );
}

#[test]
fn combined_text_validation_matches_the_localization_tabs() {
    assert!(is_invalid_timezone_text(
        "timezones.dateTimeZone",
        "{{dateTime}}"
    ));
    assert!(is_invalid_timezone_text(
        "timezones.myTime",
        "{{zone}} · my time"
    ));
    assert!(is_invalid_timezone_text(
        "timezones.voterDateTimeZone",
        "{{dateTime}} {{zone}}"
    ));
    assert!(is_invalid_timezone_text(
        "timezones.gap",
        "{{dateTime}} does not exist"
    ));
    assert!(!is_invalid_timezone_text(
        "timezones.onThisDevice",
        "Here: {{dateTime}}"
    ));
    assert!(!is_invalid_timezone_text(
        "timezones.abbr.Asia/Manila",
        "PHT"
    ));
}

#[test]
fn template_variables_follow_each_configuration() {
    for configuration in [comelec_preset(&[]), madrid_association(&[])] {
        let event = &configuration.event;
        let zones = event.timezones.as_ref().unwrap();
        let office_zone = configuration.office.timezone.clone().unwrap();

        let variables = template_time_variables(
            Some(event),
            Some(&configuration.office),
            &defaults(),
        );
        let event_wide =
            template_time_variables(Some(event), None, &defaults());

        assert_eq!(
            variables[ELECTION_EVENT_TIMEZONE_VAR],
            json!(zones.primary)
        );
        assert_eq!(variables[ELECTION_TIMEZONE_VAR], json!(office_zone));
        assert_eq!(event_wide[ELECTION_TIMEZONE_VAR], json!(zones.primary));
    }
}

#[test]
fn helpers_print_in_the_election_zone_with_the_event_language_texts() {
    let opening = "2028-04-08T20:00:00Z";
    for configuration in [comelec_preset(&[]), madrid_association(&[])] {
        let mut variables = template_time_variables(
            Some(&configuration.event),
            Some(&configuration.office),
            &defaults(),
        );
        variables.insert("opening".to_string(), json!(opening));
        let office_zone = configuration.office.timezone.clone().unwrap();
        let primary = configuration.event.timezones.clone().unwrap().primary;
        let texts = TimeZoneTexts::from_variables(&variables);
        let at = at(opening);

        let expected_office = format!(
            "{} {}",
            format_in_zone(at, &office_zone, "%d %B %Y, %H:%M"),
            texts.name(&office_zone, at)
        );
        let expected_primary = format!(
            "{} {}",
            format_in_zone(at, &primary, "%H:%M"),
            texts.label(&primary, at)
        );

        assert_eq!(
            render(
                r#"{{datetime_zone opening output_format="%d %B %Y, %H:%M" style="voter"}}"#,
                variables.clone()
            ),
            expected_office
        );
        assert_eq!(
            render(
                r#"{{datetime_zone opening electionEventTimezone output_format="%H:%M"}}"#,
                variables
            ),
            expected_primary
        );
    }
}

#[test]
fn the_madrid_association_reads_its_spanish_defaults() {
    let configuration = madrid_association(&[]);
    let variables =
        template_time_variables(Some(&configuration.event), None, &defaults());

    assert_eq!(
        render(
            r#"{{timezone_name electionEventTimezone "2028-07-01T10:00:00Z"}}"#,
            variables.clone()
        ),
        "hora de verano de Europa central"
    );
    assert_eq!(
        render(
            r#"{{timezone_label electionEventTimezone "2028-01-01T10:00:00Z"}}"#,
            variables
        ),
        "CET"
    );
}

#[test]
fn an_event_override_of_the_manila_label_reaches_the_rendered_text() {
    let configuration =
        comelec_preset(&[("global:timezones.abbr.Asia/Manila", "PHT")]);
    let mut variables =
        template_time_variables(Some(&configuration.event), None, &defaults());
    variables
        .insert("timestamp".to_string(), json!("2028-04-09T01:30:00+00:00"));

    assert_eq!(
        render("{{datetime_zone timestamp}}", variables),
        "April 09, 2028 09:30 PHT"
    );
}

#[test]
fn helpers_use_the_root_zone_inside_blocks() {
    let configuration = comelec_preset(&[]);
    let mut variables = template_time_variables(
        Some(&configuration.event),
        Some(&configuration.office),
        &defaults(),
    );
    variables.insert(
        "rows".to_string(),
        json!([{"created": "2028-04-08T20:00:00Z"}, {"created": 1839182400}]),
    );

    assert_eq!(
        render(
            r#"{{#each rows}}{{datetime_zone created output_format="%H:%M"}};{{/each}}"#,
            variables
        ),
        format!(
            "00:00 GMT+4;{} GMT+4;",
            format_in_zone(
                DateTime::<Utc>::from_timestamp(1839182400, 0).unwrap(),
                "Asia/Dubai",
                "%H:%M"
            )
        )
    );
}

#[test]
fn without_texts_the_helpers_fall_back_to_the_tz_database() {
    let at = at("2028-07-15T12:00:00Z");
    let none = TimeZoneTexts::default();

    assert_eq!(none.label("America/New_York", at), "EDT");
    assert_eq!(none.label("Asia/Dubai", at), "GMT+4");
    assert_eq!(none.label("Asia/Kolkata", at), "IST");
    assert_eq!(none.label("Asia/Kathmandu", at), "GMT+5:45");
    assert_eq!(none.label("UTC", at), "UTC");
    assert_eq!(none.name("Asia/Dubai", at), "Asia/Dubai");
    assert_eq!(
        none.date_time_zone(
            at,
            "Asia/Dubai",
            "%H:%M",
            DateTimeZoneStyle::Label
        ),
        "16:00 GMT+4"
    );
}

#[test]
fn a_missing_instant_prints_the_fallback() {
    let variables = template_time_variables(None, None, &defaults());

    assert_eq!(render("{{datetime_zone missing}}", variables), "-");
}

#[test]
fn i18next_placeholder_spellings_count_and_render() {
    let at = at("2028-04-08T16:00:00Z");
    let spaced = texts(&[(
        "templates:timezones.dateTimeZone",
        "{{ dateTime }} ({{- zone}})",
    )]);

    assert!(!is_invalid_timezone_text(
        "timezones.dateTimeZone",
        "{{dateTime, datetime}} {{ zone }}"
    ));
    assert_eq!(
        spaced.date_time_zone(
            at,
            "Asia/Dubai",
            "%H:%M",
            DateTimeZoneStyle::Label
        ),
        "20:00 (GMT+4)"
    );
    assert_eq!(
        normalize_placeholders("{{other}} {{ city }}"),
        "{{other}} {{city}}"
    );
    assert_eq!(normalize_placeholders("open {{dateTime"), "open {{dateTime");
}

fn zone_defaults(
    zone: &str,
    abbr: &str,
    abbr_daylight: Option<&str>,
    name: &str,
) -> TimeZoneTexts {
    let mut timezones = json!({
        "abbr": {zone: abbr},
        "name": {zone: name},
        "abbrDaylight": {},
    });
    if let Some(daylight) = abbr_daylight {
        timezones["abbrDaylight"] = json!({zone: daylight});
    }
    TimeZoneTexts::build(Some(&json!({"timezones": timezones})), None)
}

#[test]
fn a_negative_dst_zone_reads_daylight_by_its_offset() {
    // tzdata gives Europe/Dublin negative DST in winter; summer is still the
    // higher offset, as the generator decided.
    let dublin = zone_defaults(
        "Europe/Dublin",
        "GMT",
        Some("GMT+1"),
        "Greenwich Mean Time",
    );

    assert_eq!(
        dublin.label("Europe/Dublin", at("2028-07-15T12:00:00Z")),
        "GMT+1"
    );
    assert_eq!(
        dublin.label("Europe/Dublin", at("2028-01-15T12:00:00Z")),
        "GMT"
    );
}

#[test]
fn an_offset_outside_both_references_uses_the_fallback_label() {
    // Casablanca is +01:00 in January and July and +00:00 in Ramadan.
    let casablanca =
        zone_defaults("Africa/Casablanca", "GMT+1", None, "Morocco Time");
    let ramadan = at("2028-02-10T12:00:00Z");

    assert_eq!(
        casablanca.label("Africa/Casablanca", at("2028-07-15T12:00:00Z")),
        "GMT+1"
    );
    assert_eq!(casablanca.label("Africa/Casablanca", ramadan), "GMT");
    assert_eq!(
        casablanca.name("Africa/Casablanca", ramadan),
        "Morocco Time"
    );
}

#[test]
fn base_and_daylight_overrides_together_pick_by_the_instant() {
    let both = texts(&[
        ("templates:timezones.abbr.America/New_York", "ET standard"),
        (
            "templates:timezones.abbrDaylight.America/New_York",
            "ET summer",
        ),
    ]);
    let daylight_only = texts(&[(
        "templates:timezones.abbrDaylight.America/New_York",
        "ET summer",
    )]);
    let winter = at("2028-01-15T12:00:00Z");
    let summer = at("2028-07-15T12:00:00Z");

    for _ in 0..20 {
        assert_eq!(both.label("America/New_York", winter), "ET standard");
        assert_eq!(both.label("America/New_York", summer), "ET summer");
    }
    assert_eq!(daylight_only.label("America/New_York", winter), "EST");
    assert_eq!(daylight_only.label("America/New_York", summer), "ET summer");
}

#[test]
fn helper_output_cannot_become_markup_or_script() {
    let configuration = comelec_preset(&[(
        "templates:timezones.abbr.Asia/Manila",
        "<b>PHT</b>`${alert(1)}`\\",
    )]);
    let mut variables =
        template_time_variables(Some(&configuration.event), None, &defaults());
    variables.insert("timestamp".to_string(), json!("2028-04-09T01:30:00Z"));

    let rendered = render(
        "<script>let date = `{{datetime_zone timestamp}}`</script>{{timezone_label}}",
        variables,
    );

    assert_eq!(
        rendered,
        "<script>let date = `April 09, 2028 09:30 &lt;b&gt;PHT&lt;/b&gt;&#x60;&#36;{alert(1)}&#x60;&#92;`</script>&lt;b&gt;PHT&lt;/b&gt;&#x60;&#36;{alert(1)}&#x60;&#92;"
    );
}

#[test]
fn the_unescaped_dash_counts_only_right_after_the_braces() {
    assert_eq!(normalize_placeholders("{{- zone}}"), "{{zone}}");
    assert_eq!(normalize_placeholders("{{ -zone}}"), "{{ -zone}}");
    assert!(is_invalid_timezone_text(
        "timezones.dateTimeZone",
        "{{dateTime}} {{ -zone}}"
    ));
}
