// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use crate::monitoring::config::{AgeBand, Split};
use crate::monitoring::presets;

fn mapping(yaml: &str) -> DimensionMapping {
    serde_yaml::from_str(yaml).unwrap()
}

fn day(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
}

#[test]
fn an_age_band_is_the_first_whose_end_the_age_does_not_pass() {
    let bands = DimensionMapping {
        age_bands: vec![
            AgeBand {
                label: "18-24".into(),
                to: Some(24),
            },
            AgeBand {
                label: "25-39".into(),
                to: Some(39),
            },
            AgeBand {
                label: "40+".into(),
                to: None,
            },
        ],
        ..mapping("voter_attribute: dateOfBirth")
    };
    let on = day(2026, 5, 11);
    let band = |born: &str| dimension_value(&bands, Some(born), on);
    assert_eq!(
        band("2001-05-11").as_deref(),
        Some("25-39"),
        "25 on the day"
    );
    assert_eq!(
        band("2001-05-12").as_deref(),
        Some("18-24"),
        "24 until the next day"
    );
    assert_eq!(band("1986-05-12").as_deref(), Some("25-39"), "39");
    assert_eq!(band("1986-05-11").as_deref(), Some("40+"), "40");
    assert_eq!(band("12/05/2001").as_deref(), Some("18-24"), "day first");
    assert_eq!(band("2001-05-12T00:00:00Z").as_deref(), Some("18-24"));
    for unreadable in ["", "  ", "soon", "2030-01-01", "2001-02-30"] {
        assert_eq!(band(unreadable), None, "{unreadable:?}");
    }
    assert_eq!(dimension_value(&bands, None, on), None);
}

#[test]
fn a_compound_value_keeps_the_part_the_settings_name() {
    let country = DimensionMapping {
        split: Some(Split {
            separator: "/".into(),
            part: 0,
        }),
        ..mapping("voter_attribute: country")
    };
    let on = day(2026, 5, 11);
    assert_eq!(
        dimension_value(&country, Some(" Spain / Madrid "), on).as_deref(),
        Some("Spain")
    );
    assert_eq!(
        dimension_value(&country, Some("Spain"), on).as_deref(),
        Some("Spain")
    );
    assert_eq!(dimension_value(&country, Some("/Madrid"), on), None);
    let embassy = DimensionMapping {
        split: Some(Split {
            separator: "/".into(),
            part: 1,
        }),
        ..mapping("voter_attribute: country")
    };
    assert_eq!(
        dimension_value(&embassy, Some("Spain"), on),
        None,
        "no such part"
    );
    assert_eq!(
        dimension_value(&mapping("voter_attribute: sex"), Some(" F "), on)
            .as_deref(),
        Some("F")
    );
}

#[test]
fn a_voter_is_read_where_the_comelec_settings_say() {
    let preset = presets::load("comelec").unwrap().unwrap();
    let settings = preset.set.settings.as_ref().unwrap();
    let attributes: BTreeMap<String, Vec<String>> = [
        ("country", vec!["Spain/Madrid"]),
        ("sex", vec!["F"]),
        ("dateOfBirth", vec!["1990-01-01"]),
        ("landBasedOrSeafarer", vec!["", "Seafarer"]),
        (
            "sequent.read-only.id-card-number-validated",
            vec!["VERIFIED"],
        ),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v.into_iter().map(String::from).collect()))
    .collect();
    let election: BTreeMap<String, String> =
        [("miru:geographical-region".to_string(), "Europe".to_string())].into();
    let facts = voter_facts(
        settings,
        VoterSources {
            attributes: &attributes,
            election_annotations: &election,
            area_annotations: &BTreeMap::new(),
        },
        day(2026, 5, 11),
    );
    assert_eq!(facts.region.as_deref(), Some("Europe"));
    assert_eq!(facts.country.as_deref(), Some("Spain"));
    assert!(facts.pre_enrolled);
    assert_eq!(
        facts.dims,
        [("age_band", "25-39"), ("sex", "F"), ("status", "Seafarer")]
            .into_iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    );
    assert!(
        !facts.dims.contains_key("dateOfBirth"),
        "no raw date of birth"
    );

    let nobody = voter_facts(
        settings,
        VoterSources {
            attributes: &BTreeMap::new(),
            election_annotations: &BTreeMap::new(),
            area_annotations: &BTreeMap::new(),
        },
        day(2026, 5, 11),
    );
    assert_eq!(
        nobody,
        VoterFacts::default(),
        "all Unknown, not pre-enrolled"
    );
}
