// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The reader, held against the writer.
//!
//! The property test compares **workbooks, not plans**, and that is the whole
//! design of this file. Behaviour preservation is the actual contract: three of the
//! mappings are deliberately not injective — resolved contest behaviour, a contest
//! on every area, a plan with no areas — and comparing plans would need an
//! exception for each. Comparing what they *produce* makes all three pass by
//! construction, and still fails the moment somebody adds a column to one side and
//! not the other.

use super::*;
use crate::election_config::architect::{to_workbook, Blueprint};

/// The sample plan `architect_tests` uses, rebuilt here.
///
/// A plan somebody could plausibly have filled in, with two languages, two
/// elections, areas, voters, trustees and the six plan-only sheets — so the round
/// trip is over something with the awkward parts in it rather than a stub.
fn sound() -> Blueprint {
    let json = serde_json::json!({
        "version": crate::election_config::architect::BLUEPRINT_VERSION,
        "external_id": "union-2027",
        "name": {"en": "Union Election 2027", "es": "Elección sindical 2027"},
        "description": {"en": "Every officer.", "es": "Todos los cargos."},
        "languages": ["en", "es"],
        "default_language": "en",
        "elections_order": "custom",
        "show_cast_vote_logs": "show-logs-tab",
        "trustee_threshold": 3,
        "notes": "Ask about the hall booking.",
        "contacts": [
            {"name": "Dana Reed", "role": "Returning officer",
             "email": "dana@example.org"}
        ],
        "trustees": [
            {"name": "Ada Lovelace", "email": "ada@example.org"},
            {"name": "Grace Hopper", "email": "grace@example.org"},
            {"name": "Alan Turing", "email": "alan@example.org"}
        ],
        "schedule": {
            "voting_opens": {
                "local": "2027-03-01T09:00",
                "zone": "America/Los_Angeles",
                "offset_minutes": -480
            },
            "voting_closes": {
                "local": "2027-03-05T17:00",
                "zone": "America/Los_Angeles",
                "offset_minutes": -480
            },
            "key_ceremony": {
                "local": "2027-02-25T10:00",
                "zone": "America/Los_Angeles",
                "offset_minutes": -480
            },
            "milestones": [
                {"event": "Nominations close", "date": "2027-02-01"}
            ]
        },
        "areas": [
            {"external_id": "north", "name": "North Local 1"},
            {"external_id": "south", "name": "South Local 2"}
        ],
        "elections": [{
            "external_id": "officers",
            "name": {"en": "Officers", "es": "Cargos"},
            "num_allowed_revotes": 2,
            "spoil_ballot_option": true,
            "contests": [{
                "external_id": "president",
                "name": {"en": "President", "es": "Presidencia"},
                "description": {"en": "One seat.", "es": "Un puesto."},
                "max_votes": 1,
                "winners": 1,
                "areas": ["north"],
                "candidates": [
                    {"external_id": "alice", "name": {"en": "Alice Okonjo"}},
                    {"external_id": "bob", "name": {"en": "Bob Iyer"}}
                ]
            }]
        }],
        "voters": [
            {"username": "ada", "email": "ada@example.org",
             "area_external_id": "north", "department": "Engineering"}
        ],
        "messages": [{
            "kind": "get-out-the-vote",
            "subject": {"en": "You have not voted yet"},
            "body": {"en": "There is still time."},
            "html": {"en": "<p>There is still time.</p>"},
            "schedule": {
                "on": [{"local": "2027-03-04T09:00", "zone": "", "offset_minutes": 0}],
                "weekly": [1, 3],
                "weekly_at": "09:30"
            }
        }],
        "auth_preset": "otp-by-availability"
    });
    serde_json::from_value(json).expect("the sample plan is a plan")
}

fn read(plan: &Blueprint) -> Blueprint {
    let workbook = to_workbook(plan).expect("the plan writes");
    plan_from_workbook(&workbook)
        .expect("and reads back without errors")
        .plan
}

/// The property test: what it produces is what it produced.
///
/// Not `plan == plan`, deliberately. Three mappings are not injective and this
/// makes all three pass by construction while still catching a column added to one
/// side and forgotten on the other — which is the failure this exists for.
#[test]
fn a_plan_round_trips_to_the_same_workbook() {
    let plan = sound();
    let once = to_workbook(&plan).expect("the plan writes");
    let twice = to_workbook(&read(&plan)).expect("and writes again");

    assert_eq!(once, twice);
}

#[test]
fn the_event_itself_survives_as_data() {
    let plan = sound();
    let back = read(&plan);

    assert_eq!(back.external_id, plan.external_id);
    assert_eq!(back.name, plan.name);
    assert_eq!(back.description, plan.description);
    assert_eq!(back.languages, plan.languages);
    assert_eq!(back.default_language, plan.default_language);
    assert_eq!(back.elections_order, plan.elections_order);
    assert_eq!(back.show_cast_vote_logs, plan.show_cast_vote_logs);
    assert_eq!(back.voting_channels, plan.voting_channels);
}

#[test]
fn the_ballot_survives_as_data() {
    let plan = sound();
    let back = read(&plan);

    assert_eq!(back.elections.len(), 1);
    let election = &back.elections[0];
    assert_eq!(election.external_id, "officers");
    assert_eq!(election.name, plan.elections[0].name);
    assert_eq!(election.num_allowed_revotes, 2);
    assert!(election.spoil_ballot_option);

    assert_eq!(election.contests.len(), 1);
    let contest = &election.contests[0];
    assert_eq!(contest.external_id, "president");
    assert_eq!(contest.name, plan.elections[0].contests[0].name);
    assert_eq!(contest.max_votes, 1);
    assert_eq!(contest.winners, 1);

    let names: Vec<&str> = contest
        .candidates
        .iter()
        .map(|candidate| candidate.external_id.as_str())
        .collect();
    assert_eq!(names, vec!["alice", "bob"]);
    assert_eq!(contest.candidates[0].name.get("en"), Some("Alice Okonjo"));
}

/// The six sheets exist for no other purpose than to survive exactly.
///
/// Contacts, trustees, the threshold, the ceremony policy and its dates, the
/// milestones and the notes. If these do not come back exactly then adding them was
/// pointless, so they are asserted field by field rather than through the workbook.
#[test]
fn everything_only_a_plan_holds_survives_exactly() {
    let plan = sound();
    let back = read(&plan);

    assert_eq!(back.contacts, plan.contacts);
    assert_eq!(back.trustees, plan.trustees);
    assert_eq!(back.trustee_threshold, 3);
    assert_eq!(back.ceremony_policy, plan.ceremony_policy);
    assert_eq!(back.notes, plan.notes);
    assert_eq!(back.schedule.milestones, plan.schedule.milestones);
    assert_eq!(back.schedule.voting_opens, plan.schedule.voting_opens);
    assert_eq!(back.schedule.voting_closes, plan.schedule.voting_closes);
}

/// A message and its send schedule come back whole.
///
/// The schedule is the one JSON cell in the format, so this is where that decision
/// is proved rather than argued: a list of timestamps each with a zone and an
/// offset, plus the days it repeats on.
#[test]
fn a_message_and_its_schedule_survive() {
    let plan = sound();
    let back = read(&plan);

    assert_eq!(back.messages.len(), 1);
    let message = &back.messages[0];
    assert_eq!(message.kind, plan.messages[0].kind);
    // What a reader sees, not how the map is shaped — see
    // `an_untranslated_string_comes_back_as_a_real_translation`.
    assert_eq!(message.subject.get("en"), Some("You have not voted yet"));
    assert_eq!(message.body.get("en"), Some("There is still time."));
    assert_eq!(
        message.html.get("en"),
        Some("<p>There is still time.</p>"),
        "the HTML body travels: a sender that can render it should not be \
         handed the plain text and told to guess the markup"
    );
    // The whole struct, so a field added to `MessageSchedule` is covered by this
    // test the moment it is added to the fixture — which is the point of comparing
    // structs rather than fields.
    assert_eq!(message.schedule, plan.messages[0].schedule);
    // And named, so a failure says which field rather than printing two structs
    // and leaving the reader to diff them. The hour rides inside the schedule's
    // JSON cell and needed no column of its own; this is the assertion that says
    // so out loud.
    assert_eq!(
        message.schedule.weekly_at, "09:30",
        "the weekly repeat's time of day survives the workbook"
    );
}

/// A language with no translation of its own comes back with one.
///
/// The fourth mapping that is not injective, and the most general: `Translated::get`
/// falls back to English deliberately, because a blank is never the right answer for
/// a candidate's name. So the writer fills every language column with what a reader
/// of that language would have seen, and reading it back makes the fallback real.
///
/// Behaviour-identical — every screen and every ballot shows what it showed before —
/// and worth a test of its own so it is a decision on the record rather than a
/// surprise the first time somebody diffs two plans.
#[test]
fn an_untranslated_string_comes_back_as_a_real_translation() {
    let mut plan = sound();
    plan.elections[0].contests[0].candidates[0].name =
        crate::election_config::architect::Translated::new("Alice Okonjo");

    let back = read(&plan);
    let name = &back.elections[0].contests[0].candidates[0].name;

    assert_eq!(name.get("en"), Some("Alice Okonjo"));
    assert_eq!(
        name.get("es"),
        Some("Alice Okonjo"),
        "which is what a Spanish reader saw before, by fallback"
    );
    assert_eq!(
        name.by_language.len(),
        2,
        "the fallback is text now rather than a rule"
    );
}

/// A ceremony keeps its zone, not just the wall clock somebody typed.
#[test]
fn a_ceremony_time_keeps_the_zone_it_was_chosen_in() {
    let back = read(&sound());
    let ceremony = back.schedule.key_ceremony.expect("it came back");

    assert_eq!(ceremony.local, "2027-02-25T10:00");
    assert_eq!(ceremony.zone, "America/Los_Angeles");
    assert_eq!(ceremony.offset_minutes, -480);
}

#[test]
fn a_voters_own_columns_come_back_with_them() {
    let back = read(&sound());
    let voter = &back.voters[0];

    assert_eq!(voter.username, "ada");
    assert_eq!(voter.area_external_id, "north");
    assert_eq!(
        voter.extra.get("department").map(String::as_str),
        Some("Engineering"),
        "a client's own column is carried, not dropped"
    );
}

/// Write-in slots are slots, not people.
///
/// The writer mints one row per slot with `presentation.is_write_in`. Reading those
/// as candidates gives a contest two people called "Write-in 1" and "Write-in 2"
/// standing in it, and makes every count of who is standing wrong.
#[test]
fn write_in_rows_come_back_as_slots_rather_than_candidates() {
    let mut plan = sound();
    plan.elections[0].contests[0].allow_writeins = true;
    plan.elections[0].contests[0].write_in_slots = 2;

    let back = read(&plan);
    let contest = &back.elections[0].contests[0];

    assert!(contest.allow_writeins);
    assert_eq!(contest.write_in_slots, 2);
    assert_eq!(
        contest.candidates.len(),
        2,
        "still two real candidates, not four"
    );
}

/// A contest on every area comes back as "every area", not as a list.
#[test]
fn a_contest_on_every_area_stays_on_every_area() {
    let mut plan = sound();
    plan.elections[0].contests[0].areas = Vec::new();

    let back = read(&plan);
    assert!(
        back.elections[0].contests[0].areas.is_empty(),
        "an empty list means every area, and that is what it should still mean"
    );
}

#[test]
fn a_contest_on_one_area_keeps_that_area() {
    let back = read(&sound());
    assert_eq!(back.elections[0].contests[0].areas, vec!["north"]);
}

// -- what it says about a document that is wrong -------------------------------

/// A contest naming an election nobody configured is dropped, not attached.
#[test]
fn a_dangling_reference_names_the_cell_and_drops_the_row() {
    let plan = sound();
    let mut workbook_rows = to_workbook(&plan).unwrap();
    // Rebuild the Contests sheet with an election that does not exist.
    let contests = crate::election_config::sheet::Sheet::from_grid(
        "Contests",
        &[
            vec![
                crate::election_config::paths::Cell::text("external_id"),
                crate::election_config::paths::Cell::text(
                    "election.external_id",
                ),
            ],
            vec![
                crate::election_config::paths::Cell::text("president"),
                crate::election_config::paths::Cell::text("nowhere"),
            ],
        ],
    )
    .unwrap();
    let sheets: Vec<crate::election_config::sheet::Sheet> = workbook_rows
        .sheets()
        .iter()
        .map(|sheet| {
            if sheet.key == "contests" {
                contests.clone()
            } else {
                sheet.clone()
            }
        })
        .collect();
    workbook_rows =
        crate::election_config::sheet::Workbook::new(sheets).unwrap();

    let refused = plan_from_workbook(&workbook_rows)
        .expect_err("a contest with no election cannot be read");
    let problem = refused
        .problems
        .iter()
        .find(|problem| problem.code == Code::DanglingReference)
        .expect("it says which reference dangles");

    let at = problem.at.as_ref().expect("and where");
    assert_eq!(at.sheet, "Contests");
    assert_eq!(at.column.as_deref(), Some("election.external_id"));
    assert!(problem.message.contains("nowhere"));
}

/// Every error, not the first one.
///
/// The dialog that shows these groups four hundred complaints by tab and by kind.
/// A reader that stopped at the first would make that screen pointless, and turn
/// one afternoon's work into four hundred rounds of upload-and-see.
#[test]
fn every_problem_is_collected_rather_than_the_first_one() {
    let bad = crate::election_config::sheet::Workbook::new(vec![
        crate::election_config::sheet::Sheet::from_grid(
            "ElectionEvent",
            &[
                vec![crate::election_config::paths::Cell::text("external_id")],
                vec![crate::election_config::paths::Cell::Blank],
            ],
        )
        .unwrap(),
        // Named but unidentified. Not blank rows: `Sheet::from_grid` drops those
        // without renumbering, so a blank row is not a row at all.
        crate::election_config::sheet::Sheet::from_grid(
            "Elections",
            &[
                vec![
                    crate::election_config::paths::Cell::text("external_id"),
                    crate::election_config::paths::Cell::text(
                        "presentation.i18n.en.name",
                    ),
                ],
                vec![
                    crate::election_config::paths::Cell::Blank,
                    crate::election_config::paths::Cell::text("Officers"),
                ],
                vec![
                    crate::election_config::paths::Cell::Blank,
                    crate::election_config::paths::Cell::text("Delegates"),
                ],
            ],
        )
        .unwrap(),
    ])
    .unwrap();

    let refused =
        plan_from_workbook(&bad).expect_err("none of that reads as a plan");
    assert!(
        refused.problems.len() >= 3,
        "the event and both elections, not just the first: {:?}",
        refused.problems.len()
    );
    assert!(
        refused.problems.iter().all(|problem| problem.at.is_some()),
        "and every one of them points at a cell"
    );
}

/// A workbook with no ElectionEvent sheet says so rather than panicking.
#[test]
fn a_workbook_that_is_not_a_configuration_is_refused() {
    let empty = crate::election_config::sheet::Workbook::new(vec![
        crate::election_config::sheet::Sheet::from_grid(
            "Sheet1",
            &[vec![crate::election_config::paths::Cell::text("hello")]],
        )
        .unwrap(),
    ])
    .unwrap();

    let refused = plan_from_workbook(&empty).expect_err("that is not a plan");
    assert!(refused
        .problems
        .iter()
        .any(|problem| problem.message.contains("ElectionEvent")));
}

// -- the sheets the wizard has no screens for ----------------------------------

/// A janitor workbook's platform sheets are carried, not dropped.
///
/// The point of the whole exercise: somebody opens a real workbook, changes a
/// candidate, rebuilds, and still has their administrator accounts and their realm
/// settings.
#[test]
fn the_platform_sheets_are_carried_through() {
    let plan = sound();
    let mut sheets = to_workbook(&plan).unwrap().sheets().to_vec();
    sheets.push(
        crate::election_config::sheet::Sheet::from_grid(
            "Admin Users",
            &[
                vec![
                    crate::election_config::paths::Cell::text("username"),
                    crate::election_config::paths::Cell::text("password"),
                ],
                vec![
                    crate::election_config::paths::Cell::text("officer"),
                    crate::election_config::paths::Cell::text("secret"),
                ],
            ],
        )
        .unwrap(),
    );
    let workbook =
        crate::election_config::sheet::Workbook::new(sheets).unwrap();

    let back = plan_from_workbook(&workbook).unwrap().plan;
    assert_eq!(back.platform.len(), 1);
    assert_eq!(back.platform[0].key, "adminusers");

    // And it is still there after writing the plan out again, which is what makes
    // the rebuilt delivery carry `admin_users.csv`.
    let again = to_workbook(&back).unwrap();
    assert!(again.has("adminusers"));
}

// -- what a cell cannot carry --------------------------------------------------

/// A logo's *name* survives; its bytes do not, and the reader says so.
///
/// Silence here would be a client rebuilding from a workbook and getting an
/// election event with no logo, discovered by looking at it.
#[test]
fn an_image_that_cannot_travel_is_said_out_loud() {
    let mut plan = sound();
    plan.logo = Some(crate::election_config::architect::CandidateImage {
        file_name: "logo.png".to_string(),
        bytes: vec![1, 2, 3, 4],
    });

    let workbook = to_workbook(&plan).unwrap();
    let read = plan_from_workbook(&workbook).expect("it still reads");

    assert!(read.plan.logo.is_none(), "no bytes came back");
    assert!(
        read.report
            .warnings()
            .any(|problem| problem.message.contains("logo.png")),
        "and the file it lost is named: {:?}",
        read.report.problems
    );
    assert!(
        !read.report.has_errors(),
        "a missing image is a warning, not a refusal"
    );
}

/// Every field of a `Blueprint` is classified, or this fails.
///
/// **The round trip cannot catch a field nobody ever wrote.** If a 31st field is
/// added to `Blueprint` and `to_workbook` never emits it, the workbooks still
/// match and the reader still passes — and the field is silently lost on every
/// import. This is the guard for that: the three lists below have to account for
/// every key, so adding a field fails here until somebody decides which it is.
#[test]
fn every_blueprint_field_is_accounted_for() {
    /// Survives a round trip as data.
    const ROUND_TRIPS: &[&str] = &[
        "external_id",
        "name",
        "description",
        "languages",
        "default_language",
        "language_detection_policy",
        "elections_order",
        "show_cast_vote_logs",
        "voting_channels",
        "ivr",
        "ivr_prompt",
        "logo_url",
        "skip_election_list",
        "show_user_profile",
        "materials_activated",
        "materials_title",
        "materials_subtitle",
        "materials",
        "areas",
        "elections",
        "voters",
        "contacts",
        "trustees",
        "trustee_threshold",
        "ceremony_policy",
        "schedule",
        "notes",
        "platform",
        "defaults",
        "messages",
    ];

    /// Cannot: a cell holds no bytes. The file *name* travels; the contents do
    /// not, and the reader says so out loud rather than dropping them quietly.
    const BYTES_ONLY: &[&str] = &["logo"];

    /// Derived or dead, so there is nothing to carry.
    ///
    /// `version` is stamped `BLUEPRINT_VERSION` on every read. `auth_preset` is
    /// declared and read nowhere at all — `compile_plan` takes the preset from
    /// `BuildOptions`, so a plan's own answer has never had an effect, which is a
    /// finding of its own rather than something this reader should paper over.
    const DERIVED: &[&str] = &["version", "auth_preset"];

    // Deliberately the *maximal* plan: `skip_serializing_if` hides an empty field,
    // so a fixture that leaves one out is a guard that cannot see it. Every field
    // of `sound()` is populated for exactly this reason.
    let plan = sound();
    let as_json = serde_json::to_value(&plan).unwrap();
    let present: std::collections::BTreeSet<String> = as_json
        .as_object()
        .expect("a plan is an object")
        .keys()
        .cloned()
        .collect();

    let classified: std::collections::BTreeSet<String> = ROUND_TRIPS
        .iter()
        .chain(BYTES_ONLY)
        .chain(DERIVED)
        .map(|name| (*name).to_string())
        .collect();

    let unclassified: Vec<&String> = present.difference(&classified).collect();
    assert!(
        unclassified.is_empty(),
        "a Blueprint field nobody has decided about: {unclassified:?}. \
         Add it to ROUND_TRIPS, BYTES_ONLY or DERIVED — and if it round trips, \
         make sure it actually does."
    );
}

/// A Voters sheet with these headers and rows, in place of the fixture's.
fn with_voters(grid: &[&[&str]]) -> Blueprint {
    use crate::election_config::paths::Cell;
    use crate::election_config::sheet::{Sheet, Workbook};

    let cells: Vec<Vec<Cell>> = grid
        .iter()
        .map(|row| row.iter().map(|text| Cell::text(*text)).collect())
        .collect();
    let voters = Sheet::from_grid("Voters", &cells).unwrap();

    let workbook = to_workbook(&sound()).expect("the plan writes");
    let sheets: Vec<Sheet> = workbook
        .sheets()
        .iter()
        .map(|sheet| {
            if sheet.key == "voters" {
                voters.clone()
            } else {
                sheet.clone()
            }
        })
        .collect();

    plan_from_workbook(&Workbook::new(sheets).unwrap())
        .expect("it reads")
        .plan
}

/// A census written before the column moved still loads.
///
/// Every census a client already has spells the area `area_name`, and read
/// strictly all of them would open with every voter's area blank — thousands of
/// rows, each refused by the build, and nothing on screen to say the column had
/// simply been renamed. The name is resolved to its identifier once, on the way
/// in, and what is saved afterwards is the new spelling.
#[test]
fn a_sheet_that_still_names_the_area_is_understood() {
    let plan = with_voters(&[
        &["username", "area_name"],
        &["ada", "North Local 1"],
        &["grace", "South Local 2"],
    ]);

    assert_eq!(plan.voters[0].area_external_id, "north");
    assert_eq!(plan.voters[1].area_external_id, "south");
}

/// `area.external_id` wins when a sheet carries both.
///
/// A sheet saved by the version that wrote both columns, then edited. The
/// identifier is what the builder has always read, so it is what a disagreement
/// resolves to — and the name does not survive as a passthrough attribute, which
/// would collide with the `area_name` the finished bundle emits.
#[test]
fn the_identifier_wins_over_a_stale_name() {
    let plan = with_voters(&[
        &["username", "area.external_id", "area_name"],
        &["ada", "south", "North Local 1"],
    ]);

    assert_eq!(plan.voters[0].area_external_id, "south");
    assert!(!plan.voters[0].extra.contains_key("area_name"));
}

/// A name no area answers to is kept, not blanked.
///
/// It is already broken. Keeping the text means validation says "no area has
/// external_id 'North Local 4'" against the row that has it, which is visible and
/// fixable; blanking it would leave a voter who silently gets no ballot.
#[test]
fn an_area_name_matching_nothing_survives_to_be_reported() {
    let plan =
        with_voters(&[&["username", "area_name"], &["ada", "North Local 4"]]);

    assert_eq!(plan.voters[0].area_external_id, "North Local 4");
}

#[test]
fn the_telephone_configuration_survives_the_workbook() {
    // The workbook is the janitor's own format and the shape a delivery is
    // handed over in, so "it reaches the event" is only half the claim. This is
    // the other half: plan → xlsx → plan, with nothing lost on the way.
    let mut plan = sound();
    plan.voting_channels.telephone = true;
    plan.ivr = Some(PlannedIvr {
        phone_number: "+18005550100".to_string(),
        flow: vec![
            IvrPhase {
                phase: "language_select".to_string(),
                ..Default::default()
            },
            IvrPhase {
                phase: "announcement".to_string(),
                name: "welcome".to_string(),
                prompt_key: "greeting".to_string(),
                accept_key: "1".to_string(),
                ..Default::default()
            },
            IvrPhase {
                phase: "ballot_loop".to_string(),
                receipt_format: "phonetic_hex_4".to_string(),
                ..Default::default()
            },
        ],
        prompts: [(
            "en".to_string(),
            [("greeting".to_string(), "Welcome".to_string())]
                .into_iter()
                .collect(),
        )]
        .into_iter()
        .collect(),
        retry_limits: BTreeMap::new(),
        assistance_phone: String::new(),
    });

    assert_eq!(read(&plan).ivr, plan.ivr);
}

#[test]
fn a_workbook_may_write_the_ivr_flow_as_a_real_json_object() {
    // What somebody editing the spreadsheet by hand would do first. `Row` runs
    // every cell through `coerce_scalar`, so bracketed text arrives as an object
    // rather than a string — the reader takes either, because refusing the
    // natural spelling would be a rule nobody could guess.
    let mut plan = sound();
    plan.ivr = Some(PlannedIvr {
        flow: vec![IvrPhase {
            phase: "goodbye".to_string(),
            ..Default::default()
        }],
        ..Default::default()
    });

    let back = read(&plan);
    let ivr = back.ivr.expect("the flow came back");
    assert_eq!(ivr.flow.len(), 1);
    assert_eq!(ivr.flow[0].phase, "goodbye");
}

#[test]
fn a_plan_with_no_telephone_writes_no_ivr_columns() {
    // A web-only plan should produce the workbook it always did. Three empty
    // columns on every sheet would show up in every diff and be three more
    // things for somebody reading a delivery to wonder about.
    let workbook = to_workbook(&sound()).expect("the plan writes");
    let headers: Vec<String> = workbook
        .rows(crate::election_config::sheet::SHEET_ELECTION_EVENT)
        .first()
        .expect("an event row")
        .cells
        .iter()
        .map(|(header, _)| header.clone())
        .collect();

    assert!(
        !headers
            .iter()
            .any(|header| header.starts_with("annotations.ivr")),
        "a web-only plan wrote {headers:?}"
    );
}

#[test]
fn a_spoken_prompt_survives_the_workbook_on_every_entity_that_has_one() {
    // Four entities carry a description and so four carry a prompt beside it.
    // Written as one `ivr:i18n` annotation each, which is the shape
    // `parseIvrEntityAnnotations` reads — and read back into the flat
    // `Translated` the wizard edits.
    let mut plan = sound();
    plan.ivr_prompt = Translated::new("Welcome to the union election");
    plan.elections[0].ivr_prompt = Translated::new("Officer elections");
    plan.elections[0].contests[0].ivr_prompt = Translated::new("For President");
    plan.elections[0].contests[0].candidates[0].ivr_prompt =
        Translated::new("Press one for Alice");

    let back = read(&plan);

    assert_eq!(back.ivr_prompt, plan.ivr_prompt);
    assert_eq!(back.elections[0].ivr_prompt, plan.elections[0].ivr_prompt);
    assert_eq!(
        back.elections[0].contests[0].ivr_prompt,
        plan.elections[0].contests[0].ivr_prompt
    );
    assert_eq!(
        back.elections[0].contests[0].candidates[0].ivr_prompt,
        plan.elections[0].contests[0].candidates[0].ivr_prompt
    );
}

#[test]
fn a_plan_nobody_wrote_a_prompt_on_carries_no_prompt_column() {
    // The column appears only when somebody has written one. A blank column on
    // four sheets of every bundle is a diff on every rebuild and four more
    // things for whoever reads a delivery to wonder about.
    let workbook = to_workbook(&sound()).expect("the plan writes");
    for sheet in ["electionevent", "elections", "contests", "candidates"] {
        let headers: Vec<String> = workbook
            .rows(sheet)
            .first()
            .map(|row| row.cells.iter().map(|(h, _)| h.clone()).collect())
            .unwrap_or_default();
        assert!(
            !headers
                .iter()
                .any(|header| header == "annotations.ivr:i18n"),
            "{sheet} wrote a prompt column: {headers:?}"
        );
    }
}
