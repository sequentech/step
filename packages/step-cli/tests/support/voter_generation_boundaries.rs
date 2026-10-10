// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Synthetic voters derive their areas, elections and embassy from an
//! exported election event. Randomness and the current date are passed in, so
//! each rule is checked with seeded or scripted generators.

use super::*;
use rand::{rngs::StdRng, RngCore, SeedableRng};
use serde_json::json;

const COLUMNS: [&str; 17] = [
    "username",
    "last_name",
    "first_name",
    "middleName",
    "dateOfBirth",
    "sex",
    "country",
    "embassy",
    "clusteredPrecinct",
    "overseasReferences",
    "area_name",
    "authorized-election-ids",
    "password",
    "email",
    "password_salt",
    "hashed_password",
    "email_verified",
];

/// Every draw returns the same word, so `random::<u32>()` yields `self.0`.
struct Constant(u32);

impl RngCore for Constant {
    fn next_u32(&mut self) -> u32 {
        self.0
    }
    fn next_u64(&mut self) -> u64 {
        u64::from(self.0)
    }
    fn fill_bytes(&mut self, destination: &mut [u8]) {
        destination.fill(0);
    }
}

fn date(year: i32, month: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, month, day).unwrap()
}

fn today() -> NaiveDate {
    date(2026, 9, 24)
}

fn seeded(seed: u64) -> StdRng {
    StdRng::seed_from_u64(seed)
}

fn realm(options: Value) -> Value {
    let profile = json!({"attributes": [
        {"name": "firstName"},
        {"name": "country", "validations": {"options": {"options": options}}}
    ]});
    json!({"components": {"org.keycloak.userprofile.UserProfileProvider": [
        {"config": {"kc.user.profile.config": [profile.to_string()]}}
    ]}})
}

/// North has two elections through three contests, South one, Empty none.
fn event() -> Value {
    json!({
        "areas": [
            {"id": "area-north", "name": "North"},
            {"id": "area-south", "name": "South"},
            {"id": "area-empty", "name": "Empty"}
        ],
        "area_contests": [
            {"area_id": "area-north", "contest_id": "contest-mayor"},
            {"area_id": "area-north", "contest_id": "contest-council"},
            {"area_id": "area-north", "contest_id": "contest-referendum"},
            {"area_id": "area-south", "contest_id": "contest-council"}
        ],
        "contests": [
            {"id": "contest-mayor", "election_id": "election-local"},
            {"id": "contest-council", "election_id": "election-local"},
            {"id": "contest-referendum", "election_id": "election-national"}
        ],
        "elections": [
            {
                "id": "election-local",
                "presentation": {"i18n": {"en": {"alias": "Madrid - Local"}}},
                "annotations": {"clustered_precint_id": "P-7"}
            },
            {"id": "election-national", "presentation": {"i18n": {"en": {"name": "National"}}}}
        ],
        "keycloak_event_realm": realm(json!(["Spain/Madrid", "Andorra"]))
    })
}

fn config() -> VotersConfig {
    VotersConfig {
        csv_file_name: "voters".into(),
        fields: COLUMNS.map(String::from).to_vec(),
        excluded_columns: Vec::new(),
        email_prefix: "synthetic".into(),
        domain: "example.test".into(),
        sequence_email_number: true,
        sequence_start_number: 5,
        username_start_number: 100,
        voter_password: "synthetic-password".into(),
        voter_password_policy: VoterPasswordPolicy::Fixed,
        password_salt: "synthetic-salt".into(),
        hashed_password: "synthetic-hash".into(),
        overseas_reference: "B".into(),
        min_age: 18,
        max_age: 90,
        authorized_elections_count: 0,
        email_verified: true,
    }
}

/// The values of the `voter`-th voter by column name. The configuration
/// excludes no columns, so values follow `cfg.fields`.
struct Voter {
    columns: Vec<String>,
    values: Vec<String>,
}

impl Voter {
    fn generate(voter: usize, event: &Value, cfg: &VotersConfig, rng: &mut impl Rng) -> Self {
        let index = ElectionIndex::from_event(event);
        let values = voter_record(voter, index.area(voter), &index, cfg, rng, today());
        assert_eq!(values.len(), cfg.fields.len());
        Self {
            columns: cfg.fields.clone(),
            values,
        }
    }

    fn get(&self, column: &str) -> &str {
        let position = self.columns.iter().position(|name| name == column);
        &self.values[position.unwrap()]
    }
}

#[test]
fn voters_are_assigned_to_the_areas_round_robin() {
    let index = ElectionIndex::from_event(&event());
    let areas: Vec<_> = (0..7)
        .map(|voter| index.area(voter)["id"].as_str().unwrap())
        .collect();
    assert_eq!(
        areas,
        [
            "area-north",
            "area-south",
            "area-empty",
            "area-north",
            "area-south",
            "area-empty",
            "area-north"
        ]
    );
}

#[test]
fn without_areas_every_voter_gets_unknown_area_details() {
    let event = json!({"elections": event()["elections"]});
    let index = ElectionIndex::from_event(&event);
    assert_eq!(index.area(3), &Value::Null);
    let voter = Voter::generate(3, &event, &config(), &mut seeded(1));
    assert_eq!(voter.get("area_name"), "Unknown");
    assert_eq!(voter.get("authorized-election-ids"), "Unknown");
    assert_eq!(voter.get("clusteredPrecinct"), "Unknown");
    assert_eq!(voter.get("country"), "Unknown/Unknown");
    assert_eq!(voter.get("embassy"), "Unknown");
}

#[test]
fn an_area_lists_each_of_its_elections_once_in_contest_order() {
    let index = ElectionIndex::from_event(&event());
    let north = index.area_elections("area-north");
    assert_eq!(north.ids, ["election-local", "election-national"]);
    assert_eq!(north.aliases, ["Madrid - Local", "National"]);
    assert_eq!(north.clustered_precincts, ["P-7", "Unknown"]);
    assert!(index.area_elections("area-empty").ids.is_empty());
}

#[test]
fn a_contest_missing_from_the_event_authorizes_an_unknown_election() {
    // Pinned as found: "Unknown" is written among the authorized election ids.
    let mut event = event();
    event["area_contests"]
        .as_array_mut()
        .unwrap()
        .push(json!({"area_id": "area-south", "contest_id": "contest-deleted"}));
    let index = ElectionIndex::from_event(&event);
    assert_eq!(
        index.area_elections("area-south").ids,
        ["election-local", "Unknown"]
    );
    let south = Voter::generate(1, &event, &config(), &mut seeded(1));
    assert_eq!(
        south.get("authorized-election-ids"),
        "election-local|Unknown"
    );
}

#[test]
fn election_aliases_fall_back_from_the_english_alias_to_any_name() {
    for (i18n, expected) in [
        (
            json!({"fr": {"alias": "Alias FR"}, "en": {"alias": "Alias EN", "name": "Name EN"}}),
            "Alias EN",
        ),
        (
            json!({"fr": {"alias": "Alias FR"}, "en": {"alias": null, "name": "Name EN"}}),
            "Name EN",
        ),
        (
            json!({"de": {"name": "Name DE"}, "fr": {"alias": "Alias FR"}, "en": {}}),
            "Alias FR",
        ),
        (json!({"de": {"name": "Name DE"}}), "Name DE"),
        (json!({"de": {"description": "no labels"}}), "Unknown"),
    ] {
        let election = json!({"presentation": {"i18n": i18n}});
        assert_eq!(election_alias(&election), expected, "{election}");
    }
    assert_eq!(election_alias(&json!({"id": "no-presentation"})), "Unknown");
}

#[test]
fn country_options_are_keyed_by_their_lowercased_embassy() {
    let options = country_embassy_options(&json!({
        "keycloak_event_realm": realm(json!(["Spain/Madrid", "Andorra", "Italy/Rome/Vatican", 7]))
    }));
    assert_eq!(
        options,
        CountryEmbassies::from([
            option("madrid", "Spain", "Madrid"),
            option("andorra", "Andorra", "Unknown"),
            option("rome/vatican", "Italy", "Rome/Vatican"),
        ])
    );
}

fn option(key: &str, country: &str, embassy: &str) -> (String, (String, String)) {
    (key.into(), (country.into(), embassy.into()))
}

#[test]
fn spaces_around_the_slash_stay_in_the_option_key() {
    // Pinned as found: lookups trim the alias, so this option never matches.
    let options = country_embassy_options(&json!({
        "keycloak_event_realm": realm(json!(["Spain / Madrid"]))
    }));
    assert_eq!(
        options,
        CountryEmbassies::from([option(" madrid", "Spain", "Madrid")])
    );
}

#[test]
fn only_the_first_user_profile_provider_is_read() {
    let provider = |options: Value| {
        realm(options)["components"]["org.keycloak.userprofile.UserProfileProvider"][0].clone()
    };
    let with_providers = |providers: Value| {
        json!({"keycloak_event_realm": {"components": {
            "org.keycloak.userprofile.UserProfileProvider": providers
        }}})
    };
    let first = provider(json!(["Spain/Madrid"]));
    let second = provider(json!(["France/Paris"]));
    let keys = |event: Value| {
        country_embassy_options(&event)
            .into_keys()
            .collect::<Vec<_>>()
    };
    assert_eq!(
        keys(with_providers(json!([first, second.clone()]))),
        ["madrid"]
    );
    assert_eq!(keys(with_providers(second)), ["paris"]);
}

#[test]
fn malformed_user_profiles_contribute_no_country_options() {
    let with_profile = |profile: Value| {
        json!({"keycloak_event_realm": {"components": {
            "org.keycloak.userprofile.UserProfileProvider": [
                {"config": {"kc.user.profile.config": [profile]}}
            ]
        }}})
    };
    for event in [
        json!({}),
        with_profile(json!("{not json")),
        with_profile(json!({"attributes": []})),
        with_profile(json!(
            r#"{"attributes":[{"name":"firstName","validations":{"options":{"options":["Spain/Madrid"]}}}]}"#
        )),
    ] {
        assert!(country_embassy_options(&event).is_empty(), "{event}");
    }
}

#[test]
fn the_embassy_is_looked_up_by_the_first_alias_before_the_dash() {
    for (alias, country, embassy) in [
        ("Madrid - Local", "Spain/Madrid", "Madrid"),
        ("  MADRID - Local - 2026", "Spain/Madrid", "Madrid"),
        ("Madrid", "Spain/Madrid", "Madrid"),
        ("Madrid-Local", "Madrid-Local/Unknown", "Unknown"),
        ("ANDORRA - Local", "Andorra/Unknown", "Unknown"),
        ("Atlantis - Local", "Atlantis/Unknown", "Unknown"),
    ] {
        let mut event = event();
        event["elections"][0]["presentation"]["i18n"]["en"]["alias"] = json!(alias);
        // North's first election is the local one; the national one follows.
        let north = Voter::generate(0, &event, &config(), &mut seeded(1));
        assert_eq!(
            (north.get("country"), north.get("embassy")),
            (country, embassy),
            "{alias}"
        );
    }
}

#[test]
fn authorized_election_ids_are_a_random_subset_of_the_area_elections() {
    let mut cfg = config();
    cfg.authorized_elections_count = 1;
    let mut chosen = HashSet::new();
    for seed in 0..64 {
        let north = Voter::generate(0, &event(), &cfg, &mut seeded(seed));
        chosen.insert(north.get("authorized-election-ids").to_string());
    }
    assert_eq!(
        chosen,
        HashSet::from([
            "election-local".to_string(),
            "election-national".to_string()
        ])
    );
}

#[test]
fn a_count_of_at_least_the_area_elections_authorizes_each_once() {
    for count in [2, 5] {
        let mut cfg = config();
        cfg.authorized_elections_count = count;
        for seed in 0..32 {
            let north = Voter::generate(0, &event(), &cfg, &mut seeded(seed));
            let mut ids: Vec<_> = north.get("authorized-election-ids").split('|').collect();
            ids.sort_unstable();
            assert_eq!(ids, ["election-local", "election-national"], "{count}");
        }
    }
}

#[test]
fn a_non_positive_count_authorizes_every_election_in_contest_order() {
    for count in [0, -1] {
        let mut cfg = config();
        cfg.authorized_elections_count = count;
        let north = Voter::generate(0, &event(), &cfg, &mut seeded(1));
        assert_eq!(
            north.get("authorized-election-ids"),
            "election-local|election-national"
        );
    }
}

#[test]
fn an_area_without_contests_is_authorized_for_unknown() {
    let mut cfg = config();
    cfg.authorized_elections_count = 1;
    let empty = Voter::generate(2, &event(), &cfg, &mut seeded(1));
    assert_eq!(empty.get("area_name"), "Empty");
    assert_eq!(empty.get("authorized-election-ids"), "Unknown");
    assert_eq!(empty.get("clusteredPrecinct"), "Unknown");
    assert_eq!(empty.get("country"), "Unknown/Unknown");
}

#[test]
fn clustered_precincts_are_joined_with_unknown_for_elections_without_one() {
    let north = Voter::generate(0, &event(), &config(), &mut seeded(1));
    assert_eq!(north.get("clusteredPrecinct"), "P-7|Unknown");
}

#[test]
fn usernames_count_up_from_the_configured_start() {
    let usernames: Vec<_> = (0..3)
        .map(|voter| {
            Voter::generate(voter, &event(), &config(), &mut seeded(1))
                .get("username")
                .to_string()
        })
        .collect();
    assert_eq!(usernames, ["100", "101", "102"]);
}

#[test]
fn sequential_emails_count_from_the_sequence_start() {
    for (voter, email) in [
        (0, "synthetic+5@example.test"),
        (3, "synthetic+8@example.test"),
    ] {
        let generated = Voter::generate(voter, &event(), &config(), &mut seeded(1));
        assert_eq!(generated.get("email"), email);
    }
}

#[test]
fn random_emails_use_a_number_from_100000_to_900099999() {
    let mut cfg = config();
    cfg.sequence_email_number = false;
    for (word, email) in [
        (0, "synthetic+100000@example.test"),
        (899_999_999, "synthetic+900099999@example.test"),
        (900_000_000, "synthetic+100000@example.test"),
    ] {
        let voter = Voter::generate(4, &event(), &cfg, &mut Constant(word));
        assert_eq!(voter.get("email"), email, "{word}");
    }
}

#[test]
fn a_fixed_password_is_given_to_every_voter_verbatim() {
    for voter in 0..3 {
        let generated = Voter::generate(voter, &event(), &config(), &mut seeded(voter as u64));
        assert_eq!(generated.get("password"), "synthetic-password");
    }
}

#[test]
fn random_pins_have_exactly_the_configured_number_of_digits() {
    let mut rng = seeded(9);
    for digits in [1, 6, 16] {
        let mut cfg = config();
        cfg.voter_password_policy = VoterPasswordPolicy::RandomNumeric { digits };
        for voter in 0..16 {
            let pin = Voter::generate(voter, &event(), &cfg, &mut rng)
                .get("password")
                .to_string();
            assert_eq!(pin.len(), digits as usize, "{pin}");
            assert!(pin.bytes().all(|byte| byte.is_ascii_digit()), "{pin}");
        }
    }
}

#[test]
fn random_pins_keep_leading_zeros() {
    let mut cfg = config();
    cfg.voter_password_policy = VoterPasswordPolicy::RandomNumeric { digits: 16 };
    let voter = Voter::generate(0, &event(), &cfg, &mut Constant(0));
    assert_eq!(voter.get("password"), "0000000000000000");
}

#[test]
fn configured_values_are_copied_and_unknown_columns_are_empty() {
    let mut cfg = config();
    cfg.fields.push("nickname".into());
    let voter = Voter::generate(1, &event(), &cfg, &mut seeded(1));
    assert_eq!(voter.get("area_name"), "South");
    assert_eq!(voter.get("overseasReferences"), "B");
    assert_eq!(voter.get("password_salt"), "synthetic-salt");
    assert_eq!(voter.get("hashed_password"), "synthetic-hash");
    assert_eq!(voter.get("email_verified"), "true");
    assert_eq!(voter.get("middleName"), "");
    assert_eq!(voter.get("nickname"), "");
    cfg.email_verified = false;
    let voter = Voter::generate(1, &event(), &cfg, &mut seeded(1));
    assert_eq!(voter.get("email_verified"), "false");
}

#[test]
fn names_and_sex_are_drawn_for_each_voter() {
    let mut rng = seeded(7);
    let mut first_names = HashSet::new();
    let mut sexes = HashSet::new();
    for voter in 0..64 {
        let generated = Voter::generate(voter, &event(), &config(), &mut rng);
        assert!(!generated.get("last_name").is_empty());
        first_names.insert(generated.get("first_name").to_string());
        sexes.insert(generated.get("sex").to_string());
    }
    assert!(first_names.len() > 1 && !first_names.contains(""));
    assert_eq!(sexes, HashSet::from(["M".to_string(), "F".to_string()]));
}

#[test]
fn dates_of_birth_fall_between_the_maximum_and_minimum_ages() {
    let mut rng = seeded(3);
    for voter in 0..256 {
        let generated = Voter::generate(voter, &event(), &config(), &mut rng);
        let dob = NaiveDate::parse_from_str(generated.get("dateOfBirth"), "%Y-%m-%d").unwrap();
        // 90 and 18 years of 365 days before 2026-09-24.
        assert!(
            date(1936, 10, 16) <= dob && dob <= date(2008, 9, 28),
            "{dob}"
        );
    }
}

#[test]
fn the_lowest_sample_is_the_youngest_eligible_age() {
    assert_eq!(sample_dob(0.0, today(), 18, 90), date(2008, 9, 28));
}

#[test]
fn the_voter_population_halves_every_twenty_five_years_of_age() {
    // With H = 25 * 365 and D = 72 * 365 days, the share of voters at most
    // x days older than the minimum age is (1 - 2^(-x/H)) / (1 - 2^(-D/H)).
    let truncation = 1.0 - 0.5_f64.powf(72.0 / 25.0);
    assert_eq!(
        sample_dob(0.5 / truncation, today(), 18, 90),
        date(1983, 10, 5)
    );
    assert_eq!(
        sample_dob(0.75 / truncation, today(), 18, 90),
        date(1958, 10, 11)
    );
}

#[test]
fn samples_grow_older_without_passing_the_maximum_age() {
    let dates: Vec<_> = (0..=1000)
        .map(|step| sample_dob(f64::from(step) / 1001.0, today(), 18, 90))
        .collect();
    assert!(dates.windows(2).all(|pair| pair[0] >= pair[1]));
    assert_eq!(
        sample_dob(1.0 - f64::EPSILON, today(), 18, 90),
        date(1936, 10, 17)
    );
    assert!(dates.iter().all(|dob| *dob >= date(1936, 10, 16)));
}

#[test]
fn equal_age_bounds_always_give_the_same_date() {
    for u in [0.0, 0.5, 0.999] {
        assert_eq!(sample_dob(u, today(), 30, 30), date(1996, 10, 1));
    }
}

#[test]
fn inverted_age_bounds_give_the_date_of_the_maximum_age() {
    for u in [0.0, 0.5, 0.999] {
        assert_eq!(sample_dob(u, today(), 30, 20), date(2006, 9, 29));
    }
}
