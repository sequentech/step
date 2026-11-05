// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;

fn utc(text: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(text)
        .unwrap()
        .with_timezone(&Utc)
}

fn at(local: &str, zone: &str) -> Resolved {
    resolve_local(parse_local(local).unwrap(), parse_zone(zone).unwrap())
}

#[test]
fn midnight_local_in_posts_with_whole_and_fractional_offsets() {
    assert_eq!(
        at("2028-04-09T00:00", "Asia/Dubai"),
        Resolved::Exact(utc("2028-04-08T20:00:00Z"))
    );
    assert_eq!(
        at("2028-04-09T00:00", "Asia/Manila"),
        Resolved::Exact(utc("2028-04-08T16:00:00Z"))
    );
    assert_eq!(
        at("2028-04-09T00:00", "Asia/Kolkata"),
        Resolved::Exact(utc("2028-04-08T18:30:00Z"))
    );
    assert_eq!(
        at("2028-04-09T00:00", "Asia/Kathmandu"),
        Resolved::Exact(utc("2028-04-08T18:15:00Z"))
    );
    assert_eq!(
        at("2028-04-09T00:00", "Asia/Tehran"),
        Resolved::Exact(utc("2028-04-08T20:30:00Z"))
    );
    assert_eq!(
        at("2028-04-09T00:00", "Asia/Yangon"),
        Resolved::Exact(utc("2028-04-08T17:30:00Z"))
    );
}

#[test]
fn a_time_that_doesnt_exist_runs_after_the_change() {
    // Toronto springs forward at 02:00 on 12 March 2028.
    assert_eq!(
        at("2028-03-12T02:30", "America/Toronto"),
        Resolved::Gap {
            shifted: utc("2028-03-12T07:30:00Z")
        }
    );
    assert_eq!(
        local_in(
            parse_zone("America/Toronto").unwrap(),
            utc("2028-03-12T07:30:00Z")
        )
        .to_string(),
        "2028-03-12 03:30:00"
    );
}

#[test]
fn a_time_that_happens_twice_uses_the_first() {
    // Toronto falls back at 02:00 on 5 November 2028.
    let resolved = at("2028-11-05T01:30", "America/Toronto");
    assert_eq!(
        resolved,
        Resolved::Overlap {
            first: utc("2028-11-05T05:30:00Z"),
            second: utc("2028-11-05T06:30:00Z")
        }
    );
    assert_eq!(resolved.instant(), utc("2028-11-05T05:30:00Z"));
}

#[test]
fn cairo_changes_offset_during_the_voting_period() {
    let cairo = parse_zone("Africa/Cairo").unwrap();
    assert_eq!(
        offset_seconds_at(cairo, utc("2028-04-09T00:00:00Z")),
        2 * 3600
    );
    assert_eq!(
        offset_seconds_at(cairo, utc("2028-05-08T00:00:00Z")),
        3 * 3600
    );
}

#[test]
fn browser_aliases_become_canonical_names() {
    assert_eq!(canonical_zone("Asia/Calcutta"), "Asia/Kolkata");
    assert_eq!(parse_zone("Asia/Calcutta").unwrap().name(), "Asia/Kolkata");
    assert!(parse_zone("Mars/Olympus").is_err());
}

#[test]
fn offsets_for_eml_are_hours_or_minutes() {
    assert!(matches!(
        offset_at(
            parse_zone("Asia/Manila").unwrap(),
            utc("2028-05-08T11:00:00Z")
        ),
        TimeZone::Offset(8)
    ));
    assert!(matches!(
        offset_at(
            parse_zone("Asia/Kathmandu").unwrap(),
            utc("2028-05-08T11:00:00Z")
        ),
        TimeZone::OffsetMinutes(345)
    ));
}

fn presentation(configured: &[&str], primary: &str) -> ElectionEventPresentation {
    ElectionEventPresentation {
        timezones: Some(ElectionEventTimeZones {
            configured: configured.iter().map(|zone| zone.to_string()).collect(),
            primary: primary.to_owned(),
            logs: Default::default(),
        }),
        ..Default::default()
    }
}

fn post(zone: Option<&str>) -> ElectionPresentation {
    ElectionPresentation {
        timezone: zone.map(str::to_owned),
        ..Default::default()
    }
}

/// The two configurations the feature is proven under: a primary in Manila
/// with Posts abroad, and one in Madrid with an office in the Canaries.
fn configurations() -> [(ElectionEventPresentation, &'static str, &'static str); 2] {
    [
        (
            presentation(&["Asia/Manila", "Asia/Dubai"], "Asia/Manila"),
            "Asia/Manila",
            "Asia/Dubai",
        ),
        (
            presentation(&["Europe/Madrid", "Atlantic/Canary"], "Europe/Madrid"),
            "Europe/Madrid",
            "Atlantic/Canary",
        ),
    ]
}

#[test]
fn the_event_zone_is_the_presentation_primary() {
    for (event, primary, other) in configurations() {
        assert_eq!(primary_zone(Some(&event)).name(), primary);
        assert_eq!(
            election_zone(Some(&event), Some(&post(Some(other)))).name(),
            other
        );
        assert_eq!(
            election_zone(Some(&event), Some(&post(None))).name(),
            primary
        );
        assert_eq!(election_zone(Some(&event), None).name(), primary);
    }
}

#[test]
fn an_event_without_zones_or_with_an_unknown_one_uses_utc() {
    assert_eq!(primary_zone(None), Tz::UTC);
    assert_eq!(
        primary_zone(Some(&ElectionEventPresentation::default())),
        Tz::UTC
    );
    let unknown = presentation(&["Mars/Olympus"], "Mars/Olympus");
    assert_eq!(primary_zone(Some(&unknown)), Tz::UTC);
    // An alias reads as its canonical zone.
    let alias = presentation(&["Asia/Calcutta"], "Asia/Calcutta");
    assert_eq!(primary_zone(Some(&alias)), Tz::Asia__Kolkata);
}

/// The aliases browsers report, as the admin portal maps them (ui-core
/// `timeZones.ts`): browser and server store the same canonical names.
#[test]
fn each_browser_alias_parses_to_its_canonical_zone() {
    let browser_aliases = [
        ("Africa/Asmera", "Africa/Asmara"),
        ("America/Buenos_Aires", "America/Argentina/Buenos_Aires"),
        ("America/Catamarca", "America/Argentina/Catamarca"),
        ("America/Coral_Harbour", "America/Atikokan"),
        ("America/Cordoba", "America/Argentina/Cordoba"),
        ("America/Godthab", "America/Nuuk"),
        ("America/Indianapolis", "America/Indiana/Indianapolis"),
        ("America/Jujuy", "America/Argentina/Jujuy"),
        ("America/Louisville", "America/Kentucky/Louisville"),
        ("America/Mendoza", "America/Argentina/Mendoza"),
        ("Asia/Calcutta", "Asia/Kolkata"),
        ("Asia/Dacca", "Asia/Dhaka"),
        ("Asia/Katmandu", "Asia/Kathmandu"),
        ("Asia/Rangoon", "Asia/Yangon"),
        ("Asia/Saigon", "Asia/Ho_Chi_Minh"),
        ("Asia/Ulan_Bator", "Asia/Ulaanbaatar"),
        ("Atlantic/Faeroe", "Atlantic/Faroe"),
        ("Europe/Kiev", "Europe/Kyiv"),
        ("Pacific/Enderbury", "Pacific/Kanton"),
        ("Pacific/Ponape", "Pacific/Pohnpei"),
        ("Pacific/Truk", "Pacific/Chuuk"),
    ];
    assert_eq!(ALIASES.len(), browser_aliases.len());
    for (alias, canonical) in browser_aliases {
        assert_eq!(canonical_zone(alias), canonical);
        assert_eq!(parse_zone(alias).unwrap().name(), canonical, "{alias}");
        assert_eq!(parse_zone(canonical).unwrap().name(), canonical);
    }
}

#[test]
fn malformed_stored_timezones_count_as_none() {
    for malformed in [
        serde_json::json!({}),
        serde_json::json!("Asia/Manila"),
        serde_json::json!({"configured": "Asia/Manila", "primary": 8}),
    ] {
        assert_eq!(primary_zone(Some(&event_zones(Some(malformed)))), Tz::UTC);
    }
    assert_eq!(primary_zone(Some(&event_zones(None))), Tz::UTC);
    let stored = serde_json::json!({"configured": ["Europe/Madrid"], "primary": "Europe/Madrid"});
    assert_eq!(
        primary_zone(Some(&event_zones(Some(stored)))),
        Tz::Europe__Madrid
    );
}
