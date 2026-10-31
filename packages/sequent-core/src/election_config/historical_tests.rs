// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
use super::*;
use crate::election_config::{
    archive, build, sources::Sources, xlsx_write::write_xlsx, BuildOptions,
    TemplateSet,
};

fn fixture() -> (Vec<u8>, Vec<u8>) {
    let mut sql=String::from("CREATE TABLE `allbgy` (`ID_BARANGAY` text, `AREANAME` text); INSERT INTO `allbgy` VALUES ('B01','Area One');\nCREATE TABLE `allmun` (`ID_CITY` text, `AREANAME` text); INSERT INTO `allmun` VALUES ('001002','City One');\nCREATE TABLE `polling_centers` (`ID` text, `POLLING_PLACE` text); INSERT INTO `polling_centers` VALUES ('P01','Post One');\nCREATE TABLE `pop` (`CLUSTERPOLLCENTER` text, `POLLCENTER_CODE` text, `PROV_CODE` text, `MUN_CODE` text, `BRGY_CODE` text); INSERT INTO `pop` VALUES ('B01','P01','001','002','003');\nCREATE TABLE `trans_route` (`TRANS_SOURCE_ID` text, `TRANS_DEST_ID` text); INSERT INTO `trans_route` VALUES ('0010020003','DEST01');\nCREATE TABLE `seat` (`DISTRICTCODE` text, `CONTEST` text, `ELIGIBLEAMOUNT` int, `GEOCODE` text); INSERT INTO `seat` VALUES ('S01','Example Seat',1,'G01');\nCREATE TABLE `contest` (`POSTCODE` text, `POSTNAME` text, `SORT_ORDER` int); INSERT INTO `contest` VALUES ('C01','Example Seat',1);\nCREATE TABLE `candidate` (`CAND_CODE` text, `NAMEONBALLOT` text, `POST_CODE` text, `DISTRICTCODE` text, `NOMINATEDBY` text); INSERT INTO `candidate` VALUES ('0007','Example Candidate','C01','S01','PR01');\nCREATE TABLE `party_list` (`CODE_PARTY` text, `SHORT_NAME` text, `NAME_PARTY` text); INSERT INTO `party_list` VALUES ('PR01','EP','Example Party');\nCREATE TABLE `geographical` (`GEOCODE` text); INSERT INTO `geographical` VALUES ('G01');\nCREATE TABLE `voters` (`ID` text); INSERT INTO `voters` VALUES ('private-voter-id');\n");
    for name in [
        "allprov",
        "canvass_center",
        "district_type",
        "ref_typeboard",
    ] {
        sql.push_str(&format!("CREATE TABLE `{name}` (`ID` text);\n"));
    }
    let workbook = Workbook::new(vec![
        rows_sheet(
            "ElectionEvent",
            vec![row(
                "ElectionEvent",
                0,
                vec![
                    ("name", json!("Synthetic Event")),
                    ("description", json!("Test")),
                    ("annotations.miru_election_event_id", json!("evt-001")),
                ],
            )],
        ),
        rows_sheet(
            "Elections",
            vec![row(
                "Elections",
                0,
                vec![
                    ("election_post", json!("Post One")),
                    ("alias", json!("e1")),
                    ("name", json!("Example Election")),
                    ("description", json!("Test")),
                ],
            )],
        ),
        rows_sheet(
            "Areas",
            vec![row(
                "Areas",
                0,
                vec![
                    ("name", json!("Area One")),
                    ("description", json!("Test")),
                ],
            )],
        ),
        rows_sheet("CcsServers", vec![]),
        rows_sheet(
            "ScheduledEvents",
            vec![
                row(
                    "ScheduledEvents",
                    0,
                    vec![
                        ("election_alias", json!("e1")),
                        ("type", json!("START_VOTING_PERIOD")),
                        ("date", json!("2027-01-01T10:00:00+08:00")),
                    ],
                ),
                row(
                    "ScheduledEvents",
                    1,
                    vec![
                        ("election_alias", json!("e1")),
                        ("type", json!("END_VOTING_PERIOD")),
                        ("date", json!("2027-01-02T10:00:00+08:00")),
                    ],
                ),
            ],
        ),
    ])
    .unwrap();
    (sql.into_bytes(), write_xlsx(&workbook).unwrap())
}
fn reviewed() -> HistoricalOptions {
    HistoricalOptions {
        census: CensusPolicy::ConfigurationOnly,
        ballot: BallotPolicy::SeatPluralityAtLarge,
        scope: ScopePolicy::ConfiguredSeatsOnly,
        schedule_offset_minutes: None,
        ..HistoricalOptions::default()
    }
}
#[test]
fn refuses_undefined_census_eligibility_without_exposing_voter_values() {
    let (sql, xlsx) = fixture();
    let refused =
        adapt(&sql, Some(&xlsx), &HistoricalOptions::default()).unwrap_err();
    assert_eq!(refused.report.problems[0].code, Code::NotApproved);
    assert!(refused.report.problems[0].path.contains("voters"));
    assert!(!format!("{:?}", refused.report).contains("private-voter-id"));
    assert_eq!(refused.summary.source_rows["voters"], 1);
}
#[test]
fn preserves_codes_reconciles_explicit_exclusions_and_builds_repeatable_native_archive(
) {
    let (sql, xlsx) = fixture();
    let adapted = adapt(&sql, Some(&xlsx), &reviewed()).unwrap();
    assert_eq!(adapted.summary.selected_candidates, 1);
    assert_eq!(adapted.summary.excluded_voters, 1);
    assert_eq!(
        adapted.workbook.rows("candidates")[0]
            .text("annotations.miru:candidate-id"),
        Some("0007")
    );
    assert_eq!(adapted.summary.generated_rows["AreaContests"], 1);
    let templates = TemplateSet::builtin().unwrap();
    let options = BuildOptions::default();
    let a = build(&adapted.workbook, &templates, &options, &Sources::default())
        .unwrap();
    let schema: crate::election_config::ImportElectionEventSchema =
        serde_json::from_value(a.export.clone()).unwrap();
    assert!(!crate::election_config::validate(&schema).has_errors());
    assert_eq!(
        a.export["candidates"][0]["annotations"]["miru:candidate-id"],
        json!("0007")
    );
    let b = build(&adapted.workbook, &templates, &options, &Sources::default())
        .unwrap();
    let provenance: Value = serde_json::from_str(
        a.export["election_event"]["annotations"]["import:provenance"]
            .as_str()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(provenance["sourceRows"]["voters"], json!(1));
    assert_eq!(provenance["excludedVoters"], json!(1));
    assert_eq!(
        provenance["decisions"]["census"],
        json!("configuration-only")
    );
    let a = archive::layout(&a);
    let b = archive::layout(&b);
    assert_eq!(
        archive::zip(&a.importable).unwrap(),
        archive::zip(&b.importable).unwrap()
    );
    assert!(a
        .importable
        .iter()
        .any(|f| f.name.starts_with("export_election_event-")));
    assert!(a
        .importable
        .iter()
        .any(|f| f.name.starts_with("export_scheduled_events-")));
    assert!(!a
        .importable
        .iter()
        .any(|f| f.name.starts_with("export_voters-")));
}
fn preserving() -> HistoricalOptions {
    HistoricalOptions {
        census: CensusPolicy::PreserveSourceVoters,
        voter_area_join: Some(VoterAreaJoin::PopulationPrecinctTuple),
        ballot: BallotPolicy::SeatPluralityAtLarge,
        scope: ScopePolicy::ConfiguredSeatsOnly,
        schedule_offset_minutes: None,
        ..HistoricalOptions::default()
    }
}
/// A representative overseas source: three areas reached through one post, and 80
/// unique voters over four `(COUNTRY=PROV_CODE, EMBASSY=MUN_CODE, PRECINCT)` tuples,
/// 20 per tuple, status cycling `O/P/S/X` (20 of each). Precinct `0001A` appears
/// under two embassies AND two countries, so it must resolve to three different
/// areas — the full-tuple join. Every voter carries maternal name, suffix and a
/// full date of birth, so the optional-attribute mapping is exercised. `extra` lets
/// a test append raw pop rows and voter rows (`(extra_pop, extra_voters)`).
fn overseas_voter_fixture(
    extra_pop: &str,
    extra_voters: &str,
) -> (Vec<u8>, Vec<u8>) {
    overseas_voter_fixture_with(extra_pop, extra_voters, "WASHINGTON D.C.")
}

/// The overseas fixture with the description (the embassy label) of its
/// `UNITED STATES` area replaced.
fn overseas_voter_fixture_with(
    extra_pop: &str,
    extra_voters: &str,
    us_description: &str,
) -> (Vec<u8>, Vec<u8>) {
    let mut sql = String::new();
    sql.push_str("CREATE TABLE `allbgy` (`ID_BARANGAY` text, `AREANAME` text); INSERT INTO `allbgy` VALUES ('B06','KINGDOM OF THAILAND'),('B02','PEOPLES REPUBLIC OF BANGLADESH'),('B91','UNITED STATES');\n");
    sql.push_str("CREATE TABLE `allmun` (`ID_CITY` text, `AREANAME` text); INSERT INTO `allmun` VALUES ('9006','City A'),('9002','City B'),('9101','City C');\n");
    sql.push_str("CREATE TABLE `polling_centers` (`ID` text, `POLLING_PLACE` text); INSERT INTO `polling_centers` VALUES ('P01','Post One');\n");
    sql.push_str(&format!("CREATE TABLE `pop` (`CLUSTERPOLLCENTER` text, `POLLCENTER_CODE` text, `PROV_CODE` text, `MUN_CODE` text, `BRGY_CODE` text, `PRECINCT` text); INSERT INTO `pop` VALUES ('B06','P01','90','06','001','0001A'),('B06','P01','90','06','001','0002A'),('B02','P01','90','02','001','0001A'),('B91','P01','91','01','001','0001A'){extra_pop};\n"));
    sql.push_str("CREATE TABLE `trans_route` (`TRANS_SOURCE_ID` text, `TRANS_DEST_ID` text); INSERT INTO `trans_route` VALUES ('90060001','DESTA'),('90020001','DESTB'),('91010001','DESTC');\n");
    sql.push_str("CREATE TABLE `seat` (`DISTRICTCODE` text, `CONTEST` text, `ELIGIBLEAMOUNT` int, `GEOCODE` text); INSERT INTO `seat` VALUES ('S01','Example Seat',1,'G01');\n");
    sql.push_str("CREATE TABLE `contest` (`POSTCODE` text, `POSTNAME` text, `SORT_ORDER` int); INSERT INTO `contest` VALUES ('C01','Example Seat',1);\n");
    sql.push_str("CREATE TABLE `candidate` (`CAND_CODE` text, `NAMEONBALLOT` text, `POST_CODE` text, `DISTRICTCODE` text, `NOMINATEDBY` text); INSERT INTO `candidate` VALUES ('0007','Example Candidate','C01','S01','PR01');\n");
    sql.push_str("CREATE TABLE `party_list` (`CODE_PARTY` text, `SHORT_NAME` text, `NAME_PARTY` text); INSERT INTO `party_list` VALUES ('PR01','EP','Example Party');\n");
    sql.push_str("CREATE TABLE `geographical` (`GEOCODE` text); INSERT INTO `geographical` VALUES ('G01');\n");
    let tuples = [
        ("90", "06", "0001A"),
        ("90", "06", "0002A"),
        ("90", "02", "0001A"),
        ("91", "01", "0001A"),
    ];
    let statuses = ["O", "P", "S", "X"];
    let mut values: Vec<String> = (0..80usize)
        .map(|i| {
            let (country, embassy, precinct) = tuples[i / 20];
            let status = statuses[i % 4];
            let suffix = if i % 2 == 0 { "Jr" } else { "" };
            let month = (i % 12) + 1;
            let day = (i % 28) + 1;
            format!("('v-{i:04}','First{i}','Last{i}','Mid{i}','{suffix}','1990','{month}','{day}','{country}','{embassy}','{precinct}','{status}')")
        })
        .collect();
    if !extra_voters.is_empty() {
        values.push(extra_voters.to_string());
    }
    sql.push_str(&format!("CREATE TABLE `voters` (`ID` text, `FIRSTNAME` text, `LASTNAME` text, `MATERNALNAME` text, `SUFFIX` text, `DOBYEAR` text, `DOBMONTH` text, `DOBDAY` text, `COUNTRY` text, `EMBASSY` text, `PRECINCT` text, `ABROADSTATUS` text); INSERT INTO `voters` VALUES {};\n", values.join(",")));
    for name in [
        "allprov",
        "canvass_center",
        "district_type",
        "ref_typeboard",
    ] {
        sql.push_str(&format!("CREATE TABLE `{name}` (`ID` text);\n"));
    }
    let area = |n: usize, name: &str, description: &str| {
        row(
            "Areas",
            n,
            vec![("name", json!(name)), ("description", json!(description))],
        )
    };
    let schedule = |n: usize, kind: &str, date: &str| {
        row(
            "ScheduledEvents",
            n,
            vec![
                ("election_alias", json!("e1")),
                ("type", json!(kind)),
                ("date", json!(date)),
            ],
        )
    };
    let workbook = Workbook::new(vec![
        rows_sheet(
            "ElectionEvent",
            vec![row(
                "ElectionEvent",
                0,
                vec![
                    ("name", json!("Synthetic Event")),
                    ("description", json!("Test")),
                    ("annotations.miru_election_event_id", json!("evt-001")),
                ],
            )],
        ),
        rows_sheet(
            "Elections",
            vec![row(
                "Elections",
                0,
                vec![
                    ("election_post", json!("Post One")),
                    ("alias", json!("e1")),
                    ("name", json!("Example Election")),
                    ("description", json!("Test")),
                ],
            )],
        ),
        rows_sheet(
            "Areas",
            vec![
                area(0, "KINGDOM OF THAILAND", "BANGKOK PE"),
                area(1, "PEOPLES REPUBLIC OF BANGLADESH", "DHAKA PE"),
                area(2, "UNITED STATES", us_description),
            ],
        ),
        rows_sheet("CcsServers", vec![]),
        rows_sheet(
            "ScheduledEvents",
            vec![
                schedule(0, "START_VOTING_PERIOD", "2027-01-01T10:00:00+08:00"),
                schedule(1, "END_VOTING_PERIOD", "2027-01-02T10:00:00+08:00"),
            ],
        ),
    ])
    .unwrap();
    (sql.into_bytes(), write_xlsx(&workbook).unwrap())
}
#[test]
fn preserve_source_voters_builds_the_full_census_joined_by_the_whole_location_tuple(
) {
    let (sql, xlsx) = overseas_voter_fixture("", "");
    let adapted = adapt(&sql, Some(&xlsx), &preserving()).unwrap();
    assert_eq!(adapted.summary.preserved_voters, 80);
    assert_eq!(adapted.summary.excluded_voters, 0);

    let templates = TemplateSet::builtin().unwrap();
    let built = build(
        &adapted.workbook,
        &templates,
        &BuildOptions::default(),
        &Sources::default(),
    )
    .unwrap();

    // Assert the final serialized census (what becomes export_voters-*.csv).
    let v = &built.voters;
    assert_eq!(v.rows.len(), 80, "every source voter is preserved");
    let col = |name: &str| {
        v.column(name)
            .unwrap_or_else(|| panic!("census column {name} is missing"))
    };
    // Canonical COMELEC attribute keys the enrollment matcher reads must be present
    // (not source_* variants), so imported voters match at enrollment.
    let (username, area_name, status, email) = (
        col("username"),
        col("area_name"),
        col("source_abroad_status"),
        col("email"),
    );
    // Canonical enrollment keys AND raw source codes both present.
    let (country, embassy) = (col("country"), col("embassy"));
    let (src_country, src_embassy, src_precinct) = (
        col("source_country"),
        col("source_embassy"),
        col("source_precinct"),
    );
    let (middle, suffix, dob) =
        (col("middleName"), col("suffix"), col("dateOfBirth"));

    // 80 unique ids, exactly the source ids.
    let ids: std::collections::BTreeSet<&str> =
        v.rows.iter().map(|r| r[username].as_str()).collect();
    assert_eq!(ids.len(), 80);
    assert!(ids.contains("v-0000") && ids.contains("v-0079"));

    // Full-tuple join: every source tuple has exactly 20 voters, all mapped to the
    // right area, with the canonical embassy/country derived from that area (the same
    // normalization the registration option lists use). Source precinct 0001A under
    // embassy 06 / embassy 02 / country 91 resolves to three different areas.
    // Count-based so a non-matching tuple fails loudly.
    for (c, e, p, area, emb, cty) in [
        (
            "90",
            "06",
            "0001A",
            "KINGDOM OF THAILAND",
            "Bangkok PE",
            "Kingdom Of Thailand/Bangkok PE",
        ),
        (
            "90",
            "06",
            "0002A",
            "KINGDOM OF THAILAND",
            "Bangkok PE",
            "Kingdom Of Thailand/Bangkok PE",
        ),
        (
            "90",
            "02",
            "0001A",
            "PEOPLES REPUBLIC OF BANGLADESH",
            "Dhaka PE",
            "Peoples Republic Of Bangladesh/Dhaka PE",
        ),
        (
            "91",
            "01",
            "0001A",
            "UNITED STATES",
            "Washington D.C.",
            "United States/Washington D.C.",
        ),
    ] {
        let matching: Vec<_> = v
            .rows
            .iter()
            .filter(|r| {
                r[src_country] == c
                    && r[src_embassy] == e
                    && r[src_precinct] == p
            })
            .collect();
        assert_eq!(
            matching.len(),
            20,
            "tuple {c}/{e}/{p} should have 20 voters"
        );
        assert!(
            matching
                .iter()
                .all(|r| r[area_name] == area && r[embassy] == emb && r[country] == cty),
            "tuple {c}/{e}/{p} must map to area {area} / embassy {emb} / country {cty}"
        );
    }

    // Optional attributes are preserved with fidelity under the canonical keys, and
    // the raw source components are kept alongside for audit.
    let first = v.rows.iter().find(|r| r[username] == "v-0000").unwrap();
    assert_eq!(first[middle], "Mid0");
    assert_eq!(first[suffix], "Jr");
    assert_eq!(first[dob], "1990-01-01", "DOB parts compose to yyyy-MM-dd");
    assert_eq!(first[col("source_maternal_name")], "Mid0");
    assert_eq!(first[col("source_suffix")], "Jr");
    assert_eq!(first[col("source_dob_year")], "1990");
    assert_eq!(first[col("source_dob_month")], "1");
    assert_eq!(first[col("source_dob_day")], "1");

    // Raw O/P/S/X preserved verbatim, 20 of each; status never reinterpreted.
    for s in ["O", "P", "S", "X"] {
        assert_eq!(v.rows.iter().filter(|r| r[status] == s).count(), 20);
    }
    // No invented contact or credentials: email blank, no mobile/password columns.
    assert!(v.rows.iter().all(|r| r[email].is_empty()));
    assert!(v.column("password").is_none());
    assert!(v.column("mobile").is_none());

    let provenance: Value = serde_json::from_str(
        built.export["election_event"]["annotations"]["import:provenance"]
            .as_str()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(provenance["preservedVoters"], json!(80));
    assert_eq!(
        provenance["decisions"]["census"],
        json!("preserve-source-voters")
    );

    // The serialized CSV artifact itself carries exactly header + 80 rows.
    let layout = archive::layout(&built);
    let csv = layout
        .importable
        .iter()
        .find(|f| f.name.starts_with("export_voters-"))
        .expect("census csv is emitted");
    let lines = std::str::from_utf8(&csv.bytes)
        .unwrap()
        .lines()
        .filter(|l| !l.is_empty())
        .count();
    assert_eq!(lines, 81, "one header row plus 80 voter rows");
}
#[test]
fn a_voter_with_no_in_scope_area_is_surfaced_not_dropped() {
    // An 81st voter whose precinct no pop row provides must raise a visible per-row
    // error rather than being silently excluded.
    let (sql, xlsx) = overseas_voter_fixture(
        "",
        "('v-9999','Orphan','Voter','Mid','','1990','1','1','90','06','9999','O')",
    );
    let refused = adapt(&sql, Some(&xlsx), &preserving()).unwrap_err();
    assert_eq!(refused.report.problems[0].code, Code::DanglingReference);
    assert!(refused.report.problems[0].path.contains("voters"));
}
#[test]
fn a_precinct_key_shared_by_two_areas_is_a_conflict_not_a_guess() {
    // A second pop row with the same (province, municipality, precinct) key but a
    // different barangay/area must be rejected, so voters are never misassigned.
    let (sql, xlsx) =
        overseas_voter_fixture(",('B02','P01','90','06','001','0001A')", "");
    let refused = adapt(&sql, Some(&xlsx), &preserving()).unwrap_err();
    assert_eq!(refused.report.problems[0].code, Code::DuplicateId);
    assert!(refused.report.problems[0].path.contains("pop"));
}
#[test]
fn a_duplicate_voter_id_is_rejected() {
    let (sql, xlsx) = overseas_voter_fixture(
        "",
        "('v-0000','Dup','Licate','Mid','','1990','1','1','90','06','0001A','O')",
    );
    let refused = adapt(&sql, Some(&xlsx), &preserving()).unwrap_err();
    assert_eq!(refused.report.problems[0].code, Code::DuplicateId);
    assert!(refused.report.problems[0].path.contains("voters"));
}
#[test]
fn an_incomplete_or_impossible_date_of_birth_is_surfaced() {
    // Month/day 99 parse as numbers but are not a real date: a visible problem, not
    // a silently blank canonical identity.
    let (sql, xlsx) = overseas_voter_fixture(
        "",
        "('v-9000','Bad','Date','Mid','','1990','99','99','90','06','0001A','O')",
    );
    let refused = adapt(&sql, Some(&xlsx), &preserving()).unwrap_err();
    assert_eq!(refused.report.problems[0].code, Code::InvalidValue);
    assert!(refused.report.problems[0].path.contains("voters"));
}
#[test]
fn every_failing_voter_row_is_reported_not_only_the_first() {
    // Three independent failures (orphan location, duplicate id, impossible date)
    // must each be identified by its own row, so an operator can fix the whole
    // source in one pass. Valid voters are never silently dropped either way.
    let (sql, xlsx) = overseas_voter_fixture(
        "",
        "('v-9999','Orphan','Voter','Mid','','1990','1','1','90','06','9999','O'),\
         ('v-0000','Dup','Licate','Mid','','1990','1','1','90','06','0001A','O'),\
         ('v-9000','Bad','Date','Mid','','1990','99','99','90','06','0001A','O')",
    );
    let refused = adapt(&sql, Some(&xlsx), &preserving()).unwrap_err();
    let found: Vec<(Code, &str)> = refused
        .report
        .problems
        .iter()
        .map(|p| (p.code, p.path.as_str()))
        .collect();
    assert_eq!(
        found,
        vec![
            (
                Code::DanglingReference,
                "SQL table voters row 81 column PRECINCT"
            ),
            (Code::DuplicateId, "SQL table voters row 82 column ID"),
            (Code::InvalidValue, "SQL table voters row 83 column DOBYEAR"),
        ]
    );
}
#[test]
fn the_voter_area_join_is_an_explicit_recorded_decision() {
    // The location join is a named contract, recorded in the event provenance so a
    // reviewer can see which source columns were equated.
    let (sql, xlsx) = overseas_voter_fixture("", "");
    let adapted = adapt(&sql, Some(&xlsx), &preserving()).unwrap();
    let event = &adapted.workbook.rows("electionevent")[0];
    let provenance: Value = serde_json::from_str(
        event
            .get("annotations.import:provenance")
            .and_then(Value::as_str)
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        provenance["decisions"]["voterAreaJoin"],
        json!("population-precinct-tuple")
    );
}
#[test]
fn preserving_voters_without_a_join_decision_is_not_approved() {
    // Control: preserving() succeeds; only the join decision is removed.
    let (sql, xlsx) = overseas_voter_fixture("", "");
    let options = HistoricalOptions {
        voter_area_join: None,
        ..preserving()
    };
    let refused = adapt(&sql, Some(&xlsx), &options).unwrap_err();
    assert_eq!(refused.report.problems[0].code, Code::NotApproved);
    assert_eq!(
        refused.report.problems[0].path,
        "historical voter area join"
    );
    assert!(refused.summary.generated_rows.is_empty());
}
#[test]
fn a_voter_with_no_date_of_birth_imports_with_it_left_blank() {
    let (sql, xlsx) = overseas_voter_fixture(
        "",
        "('v-9001','No','Dob','Mid','','','','','90','06','0001A','O')",
    );
    let adapted = adapt(&sql, Some(&xlsx), &preserving()).unwrap();
    assert_eq!(adapted.summary.preserved_voters, 81);
    let voters = adapted.workbook.rows("voters");
    let voter = voters
        .iter()
        .find(|r| r.text("username") == Some("v-9001"))
        .unwrap();
    assert_eq!(
        voter.get("dateOfBirth").and_then(Value::as_str),
        Some(""),
        "a fully-absent DOB is left unset, never guessed"
    );
}
#[test]
fn refuses_a_dangling_precinct_reference_before_building() {
    let (sql, xlsx) = fixture();
    let sql = String::from_utf8(sql)
        .unwrap()
        .replace("('B01','P01','001'", "('B01','MISSING','001'");
    let refused = adapt(sql.as_bytes(), Some(&xlsx), &reviewed()).unwrap_err();
    assert_eq!(refused.report.problems[0].code, Code::DanglingReference);
    assert!(!refused.report.problems[0].message.contains("MISSING"));
}
#[test]
fn outside_seat_candidates_require_explicit_scope_review() {
    let sql = with_second_seat(fixture().0, "('S02','Second Seat',1,'G01')")
        + "INSERT INTO `candidate` VALUES ('0008','Another Example','C02','S02',NULL);";
    let xlsx = fixture().1;
    let mut options = first_seat_only(sql.as_bytes(), &xlsx);
    options.scope = ScopePolicy::RequireCompleteCoverage;
    let refused = adapt(sql.as_bytes(), Some(&xlsx), &options).unwrap_err();
    assert_eq!(refused.report.problems[0].code, Code::NotApproved);
    assert_eq!(refused.summary.outside_scope_candidates, 1);
}
/// A second, valid contest `C02` with seat `S02`, so a candidate can be valid and
/// still outside the seats an election selects.
fn with_second_seat(sql: Vec<u8>, seat: &str) -> String {
    String::from_utf8(sql).unwrap()
        + &format!(
            "INSERT INTO `seat` VALUES {seat};\nINSERT INTO `contest` VALUES ('C02','Second Seat',2);\n"
        )
}
/// The workbook's only election takes seat row 1 (`S01`) and leaves `S02` out.
fn first_seat_only(sql: &[u8], xlsx: &[u8]) -> HistoricalOptions {
    HistoricalOptions {
        election_mappings: vec![ElectionMapping {
            workbook_row: 2,
            seat_rows: vec![1],
        }],
        ..bound_options(sql, xlsx)
    }
}
#[test]
fn a_valid_unselected_contest_is_excluded_and_counted_under_configured_seats_only(
) {
    let sql = with_second_seat(fixture().0, "('S02','Second Seat',1,'G01')")
        + "INSERT INTO `candidate` VALUES ('0008','Outside Example','C02','S02',NULL);";
    let xlsx = fixture().1;
    let adapted = adapt(
        sql.as_bytes(),
        Some(&xlsx),
        &first_seat_only(sql.as_bytes(), &xlsx),
    )
    .unwrap();
    assert_eq!(adapted.summary.selected_candidates, 1);
    assert_eq!(adapted.summary.outside_scope_candidates, 1);
    let candidates = adapted.workbook.rows("candidates");
    assert_eq!(candidates.len(), 1);
    assert_eq!(
        candidates[0].text("annotations.miru:candidate-id"),
        Some("0007")
    );
    assert_eq!(adapted.summary.generated_rows["Contests"], 1);
}
#[test]
fn every_candidate_reference_is_checked_against_the_whole_source_before_scope_filtering(
) {
    struct Case {
        name: &'static str,
        candidate: &'static str,
        second_seat: &'static str,
        code: Code,
        column: &'static str,
        offending: &'static str,
    }
    let cases = [
        Case {
            name: "missing contest",
            candidate: "('0008','Dangling Example','C99','S01',NULL)",
            second_seat: "('S02','Second Seat',1,'G01')",
            code: Code::DanglingReference,
            column: "candidate row 2 column POST_CODE",
            offending: "C99",
        },
        Case {
            name: "missing seat",
            candidate: "('0008','Dangling Example','C01','S99',NULL)",
            second_seat: "('S02','Second Seat',1,'G01')",
            code: Code::DanglingReference,
            column: "candidate row 2 column DISTRICTCODE",
            offending: "S99",
        },
        Case {
            name: "seat of another contest",
            candidate: "('0008','Mismatched Example','C01','S02',NULL)",
            second_seat: "('S02','Second Seat',1,'G01')",
            code: Code::ConflictingColumns,
            column: "candidate row 2 column DISTRICTCODE",
            offending: "S02",
        },
        Case {
            name: "contest of another seat",
            candidate: "('0008','Mismatched Example','C02','S01',NULL)",
            second_seat: "('S02','Second Seat',1,'G01')",
            code: Code::ConflictingColumns,
            column: "candidate row 2 column DISTRICTCODE",
            offending: "S01",
        },
        Case {
            name: "seat naming no contest",
            candidate: "('0008','Unresolved Example','C02','S02',NULL)",
            second_seat: "('S02','Unknown Seat',1,'G01')",
            code: Code::DanglingReference,
            column: "seat row 2 column CONTEST",
            offending: "Unknown Seat",
        },
    ];
    for case in cases {
        let sql = with_second_seat(fixture().0, case.second_seat)
            + &format!("INSERT INTO `candidate` VALUES {};", case.candidate);
        let xlsx = fixture().1;
        // Every scope: the narrowest selection, every seat, and complete coverage.
        let mut complete = reviewed();
        complete.scope = ScopePolicy::RequireCompleteCoverage;
        for options in
            [first_seat_only(sql.as_bytes(), &xlsx), reviewed(), complete]
        {
            let rejected =
                adapt(sql.as_bytes(), Some(&xlsx), &options).unwrap_err();
            let problem = &rejected.report.problems[0];
            assert_eq!(problem.code, case.code, "{}", case.name);
            assert!(
                problem.path.ends_with(case.column),
                "{}: {}",
                case.name,
                problem.path
            );
            assert!(
                problem.message.contains(case.offending),
                "{}: {}",
                case.name,
                problem.message
            );
            assert_eq!(
                problem.details.get("reference").map(String::as_str),
                Some(case.offending),
                "{}",
                case.name
            );
            assert_eq!(rejected.summary.outside_scope_candidates, 0);
            assert!(rejected.summary.generated_rows.is_empty());
            assert!(!format!("{:?}", rejected.report).contains("Example"));
        }
    }
}
#[test]
fn a_null_code_on_either_side_of_a_candidate_reference_is_refused_where_it_is()
{
    let (source, xlsx) = fixture();
    let source = String::from_utf8(source).unwrap();
    for (from, to, path) in [
        (
            "('C01','Example Seat',1)",
            "(NULL,'Example Seat',1)",
            "contest row 1 column POSTCODE",
        ),
        (
            "'Example Candidate','C01','S01'",
            "'Example Candidate',NULL,'S01'",
            "candidate row 1 column POST_CODE",
        ),
        (
            "'Example Candidate','C01','S01'",
            "'Example Candidate','C01',NULL",
            "candidate row 1 column DISTRICTCODE",
        ),
        (
            "('S01','Example Seat',1,'G01')",
            "('S01',NULL,1,'G01')",
            "seat row 1 column CONTEST",
        ),
    ] {
        assert!(source.contains(from), "fixture replacement {from}");
        let modified = source.replace(from, to);
        let problem = refused_code(
            modified.as_bytes(),
            &xlsx,
            &reviewed(),
            Code::MissingField,
        );
        assert!(problem.path.ends_with(path), "{path}: {}", problem.path);
    }
}
#[test]
fn malformed_sql_reports_hashes_and_location_without_any_workbook_read() {
    let refused = adapt(
        b"SELECT 'private-record' INTO OUTFILE '/tmp/data';",
        Some(b"invalid"),
        &reviewed(),
    )
    .unwrap_err();
    assert_eq!(refused.report.problems[0].code, Code::Unreadable);
    assert_eq!(refused.summary.sql_sha256.len(), 64);
    assert!(!refused.report.problems[0]
        .message
        .contains("private-record"));
}

#[test]
fn repeated_posts_with_distinct_aliases_require_associations_instead_of_discarding_contexts(
) {
    let (sql, xlsx) = fixture();
    let workbook = read_xlsx(&xlsx).unwrap();
    let mut sheets = workbook.sheets().to_vec();
    let elections = sheets.iter_mut().find(|s| s.key == "elections").unwrap();
    let mut duplicate = elections.rows[0].clone();
    duplicate.number = 3;
    set(&mut duplicate, "alias", json!("e2"));
    elections.rows.push(duplicate);
    let xlsx = write_xlsx(&Workbook::new(sheets).unwrap()).unwrap();
    let refused = adapt(&sql, Some(&xlsx), &reviewed()).unwrap_err();
    assert_eq!(refused.report.problems[0].code, Code::NotApproved);
    assert_eq!(refused.summary.election_contexts.len(), 2);
    let mut options = reviewed();
    options.source_digests = Some(SourceDigests {
        sql_sha256: refused.summary.sql_sha256,
        workbook_sha256: refused.summary.workbook_sha256,
    });
    options.election_mappings = vec![
        ElectionMapping {
            workbook_row: 2,
            seat_rows: vec![1],
        },
        ElectionMapping {
            workbook_row: 3,
            seat_rows: vec![1],
        },
    ];
    let accepted = adapt(&sql, Some(&xlsx), &options).unwrap();
    assert_eq!(accepted.workbook.rows("elections").len(), 2);
    assert_eq!(accepted.workbook.rows("contests").len(), 2);
    assert_eq!(accepted.workbook.rows("candidates").len(), 2);
    assert_eq!(accepted.workbook.rows("scheduledevents").len(), 2);
    assert!(accepted
        .workbook
        .rows("scheduledevents")
        .iter()
        .all(|r| r.text("election.external_id").unwrap().ends_with(":e1")));
}

#[test]
fn timezone_less_dates_require_an_explicit_reviewed_offset() {
    let (sql, xlsx) = fixture();
    let workbook = read_xlsx(&xlsx).unwrap();
    let mut sheets = workbook.sheets().to_vec();
    let schedule = sheets
        .iter_mut()
        .find(|s| s.key == "scheduledevents")
        .unwrap();
    for row in &mut schedule.rows {
        set(row, "date", json!("2027-01-01 10:00:00"));
    }
    let xlsx = write_xlsx(&Workbook::new(sheets).unwrap()).unwrap();
    let refused = adapt(&sql, Some(&xlsx), &reviewed()).unwrap_err();
    assert_eq!(refused.report.problems[0].code, Code::NotApproved);
    assert!(refused.report.problems[0].path.contains("column date"));
    let mut options = reviewed();
    options.schedule_offset_minutes = Some(480);
    let accepted = adapt(&sql, Some(&xlsx), &options).unwrap();
    assert_eq!(
        accepted.workbook.rows("scheduledevents")[0].text("scheduled_datetime"),
        Some("2027-01-01T10:00:00+08:00")
    );
}

#[test]
fn reviewed_associations_cannot_be_reused_after_source_bytes_change() {
    let (sql, xlsx) = fixture();
    let mut options = reviewed();
    options.election_mappings = vec![ElectionMapping {
        workbook_row: 2,
        seat_rows: vec![1],
    }];
    options.source_digests = Some(SourceDigests {
        sql_sha256: format!("{:x}", Sha256::digest(&sql)),
        workbook_sha256: Some(format!("{:x}", Sha256::digest(&xlsx))),
    });
    assert!(adapt(&sql, Some(&xlsx), &options).is_ok());
    let changed = [
        sql.as_slice(),
        b"
-- source changed
",
    ]
    .concat();
    let refused = adapt(&changed, Some(&xlsx), &options).unwrap_err();
    assert_eq!(refused.report.problems[0].code, Code::IntegrityMismatch);
}

#[test]
fn partial_associations_cannot_drop_an_election_context() {
    let (sql, xlsx) = fixture();
    let mut options = reviewed();
    options.election_mappings = vec![ElectionMapping {
        workbook_row: 999,
        seat_rows: vec![1],
    }];
    options.source_digests = Some(SourceDigests {
        sql_sha256: format!("{:x}", Sha256::digest(&sql)),
        workbook_sha256: Some(format!("{:x}", Sha256::digest(&xlsx))),
    });
    let refused = adapt(&sql, Some(&xlsx), &options).unwrap_err();
    assert_eq!(refused.report.problems[0].code, Code::InvalidValue);
}

#[test]
fn explicit_area_associations_are_hash_bound_and_preserve_the_workbook_row() {
    let (sql, xlsx) = fixture();
    let workbook = read_xlsx(&xlsx).unwrap();
    let mut sheets = workbook.sheets().to_vec();
    set(
        &mut sheets.iter_mut().find(|s| s.key == "areas").unwrap().rows[0],
        "name",
        json!("Reviewed area label"),
    );
    let xlsx = write_xlsx(&Workbook::new(sheets).unwrap()).unwrap();
    let mut options = reviewed();
    let rejected = adapt(&sql, Some(&xlsx), &options).unwrap_err();
    assert_eq!(rejected.report.problems[0].code, Code::DanglingReference);
    options.source_digests = Some(SourceDigests {
        sql_sha256: format!("{:x}", Sha256::digest(&sql)),
        workbook_sha256: Some(format!("{:x}", Sha256::digest(&xlsx))),
    });
    options.area_mappings = vec![AreaMapping {
        workbook_row: 2,
        precinct_rows: vec![1],
    }];
    let adapted = adapt(&sql, Some(&xlsx), &options).unwrap();
    assert_eq!(
        adapted.workbook.rows("areas")[0].text("name"),
        Some("Reviewed area label")
    );
    assert_eq!(adapted.workbook.rows("areacontests").len(), 1);
    options.source_digests = None;
    assert_eq!(
        adapt(&sql, Some(&xlsx), &options)
            .unwrap_err()
            .report
            .problems[0]
            .code,
        Code::NotApproved
    );
}

#[test]
fn incomplete_post_coverage_requires_an_explicit_bound_exclusion() {
    let (mut sql, xlsx) = fixture();
    sql.extend_from_slice(
        b"INSERT INTO `polling_centers` VALUES ('P02','Unconfigured post');",
    );
    let mut options = reviewed();
    assert_eq!(
        adapt(&sql, Some(&xlsx), &options)
            .unwrap_err()
            .report
            .problems[0]
            .code,
        Code::BallotCoverage
    );
    options.post_coverage = PostCoverage::WorkbookPostsOnly;
    options.source_digests = Some(SourceDigests {
        sql_sha256: format!("{:x}", Sha256::digest(&sql)),
        workbook_sha256: Some(format!("{:x}", Sha256::digest(&xlsx))),
    });
    let accepted = adapt(&sql, Some(&xlsx), &options).unwrap();
    assert_eq!(accepted.summary.excluded_polling_centers, 1);
    assert_eq!(accepted.summary.generated_rows["Elections"], 1);
}

#[test]
fn precincts_without_workbook_areas_are_counted_only_after_explicit_exclusion()
{
    let (mut sql, xlsx) = fixture();
    sql.extend_from_slice(b"INSERT INTO `allbgy` VALUES ('B02','Missing supplemental area'); INSERT INTO `trans_route` VALUES ('0010020004','DEST02'); INSERT INTO `pop` VALUES ('B02','P01','001','002','004');");
    let mut options = reviewed();
    assert_eq!(
        adapt(&sql, Some(&xlsx), &options)
            .unwrap_err()
            .report
            .problems[0]
            .code,
        Code::DanglingReference
    );
    options.precinct_coverage = PostCoverage::WorkbookPostsOnly;
    options.source_digests = Some(SourceDigests {
        sql_sha256: format!("{:x}", Sha256::digest(&sql)),
        workbook_sha256: Some(format!("{:x}", Sha256::digest(&xlsx))),
    });
    let adapted = adapt(&sql, Some(&xlsx), &options).unwrap();
    assert_eq!(adapted.summary.excluded_precinct_rows, 1);
    assert_eq!(adapted.summary.generated_rows["Areas"], 1);
}

#[test]
fn unresolved_affiliations_require_review_and_never_gain_invented_party_names()
{
    let (sql, xlsx) = fixture();
    let sql = String::from_utf8(sql)
        .unwrap()
        .replace("'S01','PR01'", "'S01','UNRESOLVED'");
    let mut options = reviewed();
    assert_eq!(
        adapt(sql.as_bytes(), Some(&xlsx), &options)
            .unwrap_err()
            .report
            .problems[0]
            .code,
        Code::DanglingReference
    );
    options.affiliation = AffiliationPolicy::PreserveUnresolvedCode;
    options.source_digests = Some(SourceDigests {
        sql_sha256: format!("{:x}", Sha256::digest(sql.as_bytes())),
        workbook_sha256: Some(format!("{:x}", Sha256::digest(&xlsx))),
    });
    let accepted = adapt(sql.as_bytes(), Some(&xlsx), &options).unwrap();
    assert_eq!(accepted.summary.unresolved_affiliations, 1);
    let candidate = &accepted.workbook.rows("candidates")[0];
    assert_eq!(
        candidate.text("annotations.miru:candidate-affiliation-id"),
        Some("UNRESOLVED")
    );
    assert!(candidate
        .get("annotations.miru:candidate-affiliation-registered-name")
        .is_none());
}

#[test]
fn multiple_centers_cannot_silently_share_one_schedule_alias() {
    let (mut sql, xlsx) = fixture();
    sql.extend_from_slice(
        b"INSERT INTO `polling_centers` VALUES ('P02','Post One');",
    );
    let refused = adapt(&sql, Some(&xlsx), &reviewed()).unwrap_err();
    assert_eq!(refused.report.problems[0].code, Code::NotApproved);
    assert!(refused.report.problems[0]
        .message
        .contains("multiple SQL centers"));
}

fn changed_workbook(
    bytes: &[u8],
    change: impl FnOnce(&mut Vec<Sheet>),
) -> Vec<u8> {
    let mut sheets = read_xlsx(bytes).unwrap().sheets().to_vec();
    change(&mut sheets);
    for sheet in &mut sheets {
        if !sheet.rows.is_empty() {
            let mut headers = Vec::new();
            for row in &sheet.rows {
                for (name, _) in &row.cells {
                    if !headers.contains(name) {
                        headers.push(name.clone());
                    }
                }
            }
            sheet.headers = headers;
        }
    }
    write_xlsx(&Workbook::new(sheets).unwrap()).unwrap()
}
fn bound_options(sql: &[u8], xlsx: &[u8]) -> HistoricalOptions {
    HistoricalOptions {
        source_digests: Some(SourceDigests {
            sql_sha256: format!("{:x}", Sha256::digest(sql)),
            workbook_sha256: Some(format!("{:x}", Sha256::digest(xlsx))),
        }),
        ..reviewed()
    }
}
fn refused_code(
    sql: &[u8],
    xlsx: &[u8],
    options: &HistoricalOptions,
    code: Code,
) -> Problem {
    let rejected = adapt(sql, Some(xlsx), options).unwrap_err();
    assert_eq!(rejected.report.problems[0].code, code);
    assert!(rejected.summary.generated_rows.is_empty());
    rejected.report.problems[0].clone()
}
#[test]
fn strict_json_decisions_roundtrip_all_reviewed_policy_and_association_fields()
{
    let defaults: HistoricalOptions =
        serde_json::from_value(json!({})).unwrap();
    assert_eq!(defaults.ballot, BallotPolicy::RequireReview);
    assert_eq!(defaults.census, CensusPolicy::RequireReviewedMapping);
    let source = json!({"census":"configuration-only","ballot":"seat-plurality-at-large","scope":"configured-seats-only","postCoverage":"workbook-posts-only","precinctCoverage":"workbook-posts-only","affiliation":"preserve-unresolved-code","scheduleOffsetMinutes":480,"sourceDigests":{"sqlSha256":"a","workbookSha256":"b"},"electionMappings":[{"workbookRow":2,"seatRows":[1]}],"areaMappings":[{"workbookRow":2,"precinctRows":[1]}]});
    let options: HistoricalOptions =
        serde_json::from_value(source.clone()).unwrap();
    assert_eq!(serde_json::to_value(options).unwrap(), source);
    for bad in [
        json!({"automaticEligibility":true}),
        json!({"census":"guess"}),
        json!({"sourceDigests":{"sqlSha256":"a","workbookSha256":"b","extra":1}}),
        json!({"electionMappings":[{"workbookRow":2,"seatRows":[1],"guess":true}]}),
        json!({"areaMappings":[{"workbookRow":2,"precinctRows":[1],"guess":true}]}),
    ] {
        assert!(serde_json::from_value::<HistoricalOptions>(bad).is_err());
    }
}
#[test]
fn missing_schema_lookups_and_required_source_values_are_refused_with_field_origins(
) {
    let (source, xlsx) = fixture();
    let source = String::from_utf8(source).unwrap();
    for (from, to, code) in [
        (
            "CREATE TABLE `allprov` (`ID` text);\n",
            "",
            Code::MissingField,
        ),
        ("`CAND_CODE` text", "`renamed` text", Code::MissingField),
        (
            "'0007','Example Candidate'",
            "'0007',NULL",
            Code::MissingField,
        ),
        (
            "'B01','P01','001'",
            "'missing','P01','001'",
            Code::DanglingReference,
        ),
        (
            "('001002','City One')",
            "('other','City One')",
            Code::DanglingReference,
        ),
        (
            "('0010020003','DEST01')",
            "('other','DEST01')",
            Code::DanglingReference,
        ),
        ("('G01')", "('other')", Code::DanglingReference),
        (
            "('C01','Example Seat',1)",
            "('C01','Other Seat',1)",
            Code::DanglingReference,
        ),
        (
            "'Example Seat',1,'G01'",
            "'Example Seat',0,'G01'",
            Code::InvalidValue,
        ),
        (
            "'Example Seat',1,'G01'",
            "'Example Seat','many','G01'",
            Code::InvalidValue,
        ),
        (
            "'Example Seat',1);",
            "'Example Seat','first');",
            Code::InvalidValue,
        ),
    ] {
        assert!(source.contains(from), "fixture replacement {from}");
        let modified = source.replace(from, to);
        let problem =
            refused_code(modified.as_bytes(), &xlsx, &reviewed(), code);
        assert!(!problem.message.contains("missing"));
    }
    let extra = format!("{source} CREATE TABLE unknown_table (id int);");
    refused_code(extra.as_bytes(), &xlsx, &reviewed(), Code::InvalidValue);
    let duplicate=format!("{source} INSERT INTO candidate VALUES ('0007','Other','C01','S01',NULL);");
    refused_code(duplicate.as_bytes(), &xlsx, &reviewed(), Code::DuplicateId);
    let mut unreviewed = reviewed();
    unreviewed.ballot = BallotPolicy::RequireReview;
    refused_code(source.as_bytes(), &xlsx, &unreviewed, Code::NotApproved);
}
#[test]
fn declared_primary_and_unique_keys_are_validated_before_any_mapping() {
    let (source, xlsx) = fixture();
    let source = String::from_utf8(source).unwrap();
    for (definition, tail, code) in [
        ("`ID` text, PRIMARY KEY (`absent`)", "", Code::MissingField),
        ("`ID` text, UNIQUE KEY u (`absent`)", "", Code::MissingField),
        (
            "`ID` text, PRIMARY KEY (`ID`)",
            "INSERT INTO allprov VALUES (NULL);",
            Code::MissingField,
        ),
        (
            "`ID` text, UNIQUE KEY u (`ID`)",
            "INSERT INTO allprov VALUES ('1'),('1');",
            Code::DuplicateId,
        ),
    ] {
        let sql = source.replace(
            "CREATE TABLE `allprov` (`ID` text)",
            &format!("CREATE TABLE `allprov` ({definition})"),
        ) + tail;
        refused_code(sql.as_bytes(), &xlsx, &reviewed(), code);
    }
    let sql = source.replace(
        "CREATE TABLE `allprov` (`ID` text)",
        "CREATE TABLE `allprov` (`ID` text, UNIQUE KEY u (`ID`))",
    ) + "INSERT INTO allprov VALUES (NULL),(NULL),('1');";
    let adapted = adapt(sql.as_bytes(), Some(&xlsx), &reviewed()).unwrap();
    assert_eq!(adapted.summary.source_rows["allprov"], 3);
}
#[test]
fn source_charsets_and_nullable_affiliations_preserve_text_without_invention() {
    let (source, xlsx) = fixture();
    let source = String::from_utf8(source).unwrap();
    let marker = "__BYTES__";
    let sql = source
        .replace("'Example Candidate'", &format!("'{marker}'"))
        .replace(
            "`NOMINATEDBY` text);",
            "`NOMINATEDBY` text) CHARSET=latin1;",
        )
        .replace("'S01','PR01'", "'S01',NULL");
    let start = sql.find(marker).unwrap();
    let mut sql = sql.into_bytes();
    sql.splice(start..start + marker.len(), [0x80, 0xe9]);
    let adapted = adapt(&sql, Some(&xlsx), &reviewed()).unwrap();
    assert_eq!(
        adapted.workbook.rows("candidates")[0]
            .text("presentation.i18n.en.name"),
        Some("€é")
    );
    assert!(adapted.workbook.rows("candidates")[0]
        .get("annotations.miru:candidate-affiliation-id")
        .is_none());
    let unsupported = source
        .replace("`NOMINATEDBY` text);", "`NOMINATEDBY` text) CHARSET=ucs2;");
    refused_code(
        unsupported.as_bytes(),
        &xlsx,
        &reviewed(),
        Code::InvalidValue,
    );
    let invalid = sql
        .windows(b"CHARSET=latin1".len())
        .position(|p| p == b"CHARSET=latin1")
        .unwrap();
    sql.splice(
        invalid..invalid + b"CHARSET=latin1".len(),
        b"CHARSET=utf8mb4".iter().copied(),
    );
    refused_code(&sql, &xlsx, &reviewed(), Code::InvalidValue);
    let override_sql = source
        .replace(
            "`NAMEONBALLOT` text",
            "`NAMEONBALLOT` text CHARACTER SET utf8mb4",
        )
        .replace(
            "`NOMINATEDBY` text);",
            "`NOMINATEDBY` text) CHARSET=latin1;",
        )
        .replace("Example Candidate", "Résumé");
    assert_eq!(
        adapt(override_sql.as_bytes(), Some(&xlsx), &reviewed())
            .unwrap()
            .workbook
            .rows("candidates")[0]
            .text("presentation.i18n.en.name"),
        Some("Résumé")
    );
}
#[test]
fn missing_and_ambiguous_workbook_metadata_is_not_silently_filled_in() {
    let (sql, xlsx) = fixture();
    for (sheet, column) in [
        ("electionevent", "annotations.miru_election_event_id"),
        ("elections", "alias"),
        ("elections", "election_post"),
        ("areas", "name"),
    ] {
        let changed = changed_workbook(&xlsx, |sheets| {
            let target = sheets.iter_mut().find(|s| s.key == sheet).unwrap();
            set(&mut target.rows[0], column, json!(""));
        });
        refused_code(&sql, &changed, &reviewed(), Code::MissingField);
    }
    for sheet in ["electionevent", "elections", "areas"] {
        let changed = changed_workbook(&xlsx, |sheets| {
            let target = sheets.iter_mut().find(|s| s.key == sheet).unwrap();
            let mut extra = target.rows[0].clone();
            extra.number += 1;
            target.rows.push(extra);
        });
        refused_code(
            &sql,
            &changed,
            &reviewed(),
            if sheet == "electionevent" {
                Code::InvalidValue
            } else {
                Code::DuplicateId
            },
        );
    }
    let unknown =
        changed_workbook(&xlsx, |s| s.push(rows_sheet("Unexpected", vec![])));
    refused_code(&sql, &unknown, &reviewed(), Code::InvalidValue);
    let no_event =
        changed_workbook(&xlsx, |s| s.retain(|s| s.key != "electionevent"));
    refused_code(&sql, &no_event, &reviewed(), Code::InvalidValue);
    let missing_context = changed_workbook(&xlsx, |s| {
        let target = s.iter_mut().find(|s| s.key == "elections").unwrap();
        let mut extra = target.rows[0].clone();
        extra.number = 3;
        set(&mut extra, "alias", json!("unused"));
        set(&mut extra, "election_post", json!("Unused post"));
        target.rows.push(extra)
    });
    refused_code(&sql, &missing_context, &reviewed(), Code::BallotCoverage);
}
#[test]
fn association_rows_must_be_present_unique_complete_and_hash_bound() {
    let (sql, xlsx) = fixture();
    for (rows, code) in [
        (vec![], Code::InvalidValue),
        (vec![0], Code::InvalidValue),
        (vec![2], Code::InvalidValue),
        (vec![1, 1], Code::InvalidValue),
    ] {
        let mut options = bound_options(&sql, &xlsx);
        options.election_mappings = vec![ElectionMapping {
            workbook_row: 2,
            seat_rows: rows,
        }];
        refused_code(&sql, &xlsx, &options, code);
    }
    let mut options = bound_options(&sql, &xlsx);
    options.election_mappings = vec![
        ElectionMapping {
            workbook_row: 2,
            seat_rows: vec![1]
        };
        2
    ];
    refused_code(&sql, &xlsx, &options, Code::DuplicateId);
    let repeated = changed_workbook(&xlsx, |s| {
        let target = s.iter_mut().find(|s| s.key == "elections").unwrap();
        let mut extra = target.rows[0].clone();
        extra.number = 3;
        set(&mut extra, "alias", json!("e2"));
        target.rows.push(extra)
    });
    let mut options = bound_options(&sql, &repeated);
    options.election_mappings = vec![ElectionMapping {
        workbook_row: 2,
        seat_rows: vec![1],
    }];
    refused_code(&sql, &repeated, &options, Code::BallotCoverage);
    for (row, precincts) in [
        (999, vec![1]),
        (2, vec![]),
        (2, vec![0]),
        (2, vec![2]),
        (2, vec![1, 1]),
    ] {
        let mut options = bound_options(&sql, &xlsx);
        options.area_mappings = vec![AreaMapping {
            workbook_row: row,
            precinct_rows: precincts,
        }];
        refused_code(&sql, &xlsx, &options, Code::InvalidValue);
    }
}
#[test]
fn exclusions_and_municipality_area_names_reconcile_without_losing_contexts() {
    let (sql, xlsx) = fixture();
    let source = String::from_utf8(sql).unwrap();
    let changed_sql =
        source.replace("('B01','Area One')", "('B01','Post One')");
    let changed = changed_workbook(&xlsx, |s| {
        set(
            &mut s.iter_mut().find(|s| s.key == "areas").unwrap().rows[0],
            "name",
            json!("City One"),
        )
    });
    let adapted =
        adapt(changed_sql.as_bytes(), Some(&changed), &reviewed()).unwrap();
    assert_eq!(adapted.summary.precincts[0].workbook_area_rows, [2]);
    assert_eq!(
        adapted.workbook.rows("areas")[0].text("name"),
        Some("City One")
    );
    let extra=format!("{source} INSERT INTO polling_centers VALUES ('P02','Unused post'); INSERT INTO pop VALUES ('B01','P02','001','002','003');");
    let mut options = bound_options(extra.as_bytes(), &xlsx);
    options.post_coverage = PostCoverage::WorkbookPostsOnly;
    let adapted = adapt(extra.as_bytes(), Some(&xlsx), &options).unwrap();
    assert_eq!(adapted.summary.excluded_precinct_rows, 1);
    assert_eq!(adapted.summary.excluded_polling_centers, 1);
    let conflict=format!("{source} INSERT INTO trans_route VALUES ('0010020004','DEST02'); INSERT INTO pop VALUES ('B01','P01','001','002','004');");
    refused_code(
        conflict.as_bytes(),
        &xlsx,
        &reviewed(),
        Code::ConflictingColumns,
    );
}

fn with_ccs(bytes: &[u8], tag: Value, send_logs: Value) -> Vec<u8> {
    changed_workbook(bytes, |sheets| {
        set(
            &mut sheets.iter_mut().find(|s| s.key == "areas").unwrap().rows[0],
            "annotations.miru_ccs_server_tags",
            tag.clone(),
        );
        *sheets.iter_mut().find(|s| s.key == "ccsservers").unwrap() =
            rows_sheet(
                "CcsServers",
                vec![row(
                    "CcsServers",
                    0,
                    vec![
                        ("tag", if tag.is_boolean() { json!(1) } else { tag }),
                        ("name", json!("Synthetic CCS")),
                        ("address", json!("https://ccs.example.invalid")),
                        ("public_key", json!("synthetic-public-key")),
                        ("send_logs", send_logs),
                    ],
                )],
            );
    })
}
#[test]
fn ccs_trustees_annotations_and_logo_are_preserved_as_canonical_metadata() {
    let (sql, xlsx) = fixture();
    for logs in [json!(true), json!("true"), json!("FALSE"), json!(false)] {
        let with_servers = with_ccs(&xlsx, json!(1), logs.clone());
        let changed = changed_workbook(&with_servers, |sheets| {
            let event = &mut sheets
                .iter_mut()
                .find(|s| s.key == "electionevent")
                .unwrap()
                .rows[0];
            set(event, "annotations.miru_election_event_id", json!(123));
            set(
                event,
                "annotations.miru_election_event_name",
                json!("Synthetic name"),
            );
            set(event, "logo_url", json!("/local/logo.svg"));
            let election = &mut sheets
                .iter_mut()
                .find(|s| s.key == "elections")
                .unwrap()
                .rows[0];
            set(
                election,
                "annotations.miru_election_id",
                json!("context-id"),
            );
            set(
                election,
                "annotations.miru_election_name",
                json!("Context name"),
            );
            let area =
                &mut sheets.iter_mut().find(|s| s.key == "areas").unwrap().rows
                    [0];
            set(
                area,
                "annotations.miru_trustee_servers",
                json!(" trustee-a,trustee-b "),
            );
            set(area, "annotations.synthetic_flag", json!(true));
        });
        let adapted = adapt(&sql, Some(&changed), &reviewed()).unwrap();
        let event = &adapted.workbook.rows("electionevent")[0];
        assert_eq!(
            event.text("presentation.logo_url"),
            Some("/local/logo.svg")
        );
        assert_eq!(
            event.text("annotations.miru:election-event-id"),
            Some("123")
        );
        let area = &adapted.workbook.rows("areas")[0];
        let servers: Value = serde_json::from_str(
            area.text("annotations.miru:area-ccs-servers").unwrap(),
        )
        .unwrap();
        assert_eq!(servers[0]["tag"], json!("1"));
        assert_eq!(servers[0]["public_key_pem"], json!("synthetic-public-key"));
        assert_eq!(
            servers[0]["send_logs"],
            json!(logs == json!(true) || logs == json!("true"))
        );
        assert_eq!(
            area.text("annotations.miru:area-trustee-users"),
            Some("[\"trustee-a\",\"trustee-b\"]")
        );
        assert_eq!(area.text("annotations.synthetic_flag"), Some("true"));
    }
}
#[test]
fn invalid_ccs_and_annotation_values_are_actionable_local_errors() {
    let (sql, xlsx) = fixture();
    for (tag, logs) in [
        (json!(true), json!(true)),
        (json!("non-integer"), json!(true)),
        (json!(1), json!(42)),
    ] {
        let changed = with_ccs(&xlsx, tag, logs);
        refused_code(&sql, &changed, &reviewed(), Code::InvalidValue);
    }
    let changed = changed_workbook(&xlsx, |s| {
        set(
            &mut s.iter_mut().find(|s| s.key == "areas").unwrap().rows[0],
            "annotations.miru_ccs_server_tags",
            json!("999"),
        )
    });
    refused_code(&sql, &changed, &reviewed(), Code::DanglingReference);
    let changed = changed_workbook(&xlsx, |s| {
        set(
            &mut s
                .iter_mut()
                .find(|s| s.key == "electionevent")
                .unwrap()
                .rows[0],
            "annotations.miru_election_event_name",
            json!(true),
        )
    });
    refused_code(&sql, &changed, &reviewed(), Code::MissingField);
}
#[test]
fn source_text_helpers_reject_missing_cells_columns_and_duplicate_lookup_keys()
{
    let mut dump = SqlDump::default();
    dump.tables.insert(
        "sample".into(),
        SqlTable {
            columns: vec!["id".into(), "optional".into()],
            rows: vec![
                vec![SqlValue::Bytes(b"key".to_vec()), SqlValue::Null],
                vec![
                    SqlValue::Bytes(b"key".to_vec()),
                    SqlValue::Bytes(b"value".to_vec()),
                ],
            ],
            ..SqlTable::default()
        },
    );
    let data = Data::new(&dump, "sample", &["id"]).unwrap();
    assert_eq!(data.optional(0, "optional").unwrap(), None);
    assert_eq!(data.optional(1, "optional").unwrap(), Some("value".into()));
    assert_eq!(
        data.text(0, "unknown").unwrap_err().code,
        Code::MissingField
    );
    assert_eq!(
        data.optional(0, "unknown").unwrap_err().code,
        Code::MissingField
    );
    assert_eq!(data.text(99, "id").unwrap_err().code, Code::MissingField);
    assert_eq!(data.index("id").unwrap_err().code, Code::DuplicateId);
    let (mut sql, xlsx) = fixture();
    sql.extend_from_slice(b"INSERT INTO allprov VALUES (x'ff');");
    let adapted = adapt(&sql, Some(&xlsx), &reviewed()).unwrap();
    assert_eq!(adapted.summary.opaque_binary_values, 1);
}
#[test]
fn schedules_require_real_dates_aliases_and_an_explicit_bounded_offset() {
    let (sql, xlsx) = fixture();
    for date in [json!("tomorrow"), json!(123), json!(true)] {
        let changed = changed_workbook(&xlsx, |s| {
            set(
                &mut s
                    .iter_mut()
                    .find(|s| s.key == "scheduledevents")
                    .unwrap()
                    .rows[0],
                "date",
                date,
            )
        });
        refused_code(&sql, &changed, &reviewed(), Code::InvalidValue);
    }
    let alias = changed_workbook(&xlsx, |s| {
        set(
            &mut s
                .iter_mut()
                .find(|s| s.key == "scheduledevents")
                .unwrap()
                .rows[0],
            "election_alias",
            json!("unknown"),
        )
    });
    refused_code(&sql, &alias, &reviewed(), Code::DanglingReference);
    let blank = changed_workbook(&xlsx, |s| {
        s.iter_mut()
            .find(|s| s.key == "scheduledevents")
            .unwrap()
            .rows[0]
            .cells
            .retain(|(column, _)| column != "date");
    });
    refused_code(&sql, &blank, &reviewed(), Code::MissingField);
    let no_sheet =
        changed_workbook(&xlsx, |s| s.retain(|s| s.key != "scheduledevents"));
    refused_code(&sql, &no_sheet, &reviewed(), Code::MissingField);
    let no_column = changed_workbook(&xlsx, |s| {
        let schedule =
            s.iter_mut().find(|s| s.key == "scheduledevents").unwrap();
        for row in &mut schedule.rows {
            row.cells.retain(|(name, _)| name != "date");
        }
    });
    refused_code(&sql, &no_column, &reviewed(), Code::MissingField);
    let naive = changed_workbook(&xlsx, |s| {
        set(
            &mut s
                .iter_mut()
                .find(|s| s.key == "scheduledevents")
                .unwrap()
                .rows[0],
            "date",
            json!("2027-01-01T10:00:00"),
        )
    });
    let mut options = reviewed();
    options.schedule_offset_minutes = Some(841);
    refused_code(&sql, &naive, &options, Code::NotApproved);
    options.schedule_offset_minutes = Some(-840);
    assert_eq!(
        adapt(&sql, Some(&naive), &options)
            .unwrap()
            .workbook
            .rows("scheduledevents")[0]
            .text("scheduled_datetime"),
        Some("2027-01-01T10:00:00-14:00")
    );
    assert_eq!(
        historical_dates(b"not an xlsx", None).unwrap_err().code,
        Code::Unreadable
    );
}

fn rewrite_zip(bytes: &[u8], name: &str, replacement: &[u8]) -> Vec<u8> {
    use std::io::{Cursor, Read, Write};
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut output = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for i in 0..archive.len() {
        let mut member = archive.by_index(i).unwrap();
        let mut data = Vec::new();
        member.read_to_end(&mut data).unwrap();
        output
            .start_file(member.name(), zip::write::SimpleFileOptions::default())
            .unwrap();
        output
            .write_all(if member.name() == name {
                replacement
            } else {
                &data
            })
            .unwrap();
    }
    output.finish().unwrap().into_inner()
}
#[test]
fn actual_excel_date_cells_preserve_the_reviewed_instant() {
    let (_, xlsx) = fixture();
    // Independent XLSX wire cells: dates, empty cells and short rows cannot be
    // generated by the JSON workbook writer used by ordinary adapter fixtures.
    let xml=br#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData><row r="1"><c r="A1" t="inlineStr"><is><t>date</t></is></c></row><row r="2"><c r="A2" t="d"><v>2027-01-01T10:00:00+08:00</v></c></row><row r="3"><c r="A3" t="n" s="1"><v>46388.5</v></c></row><row r="4"><c r="B4" t="inlineStr"><is><t>other field</t></is></c></row></sheetData></worksheet>"#;
    let styles=br#"<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><cellXfs count="2"><xf numFmtId="0"/><xf numFmtId="14"/></cellXfs></styleSheet>"#;
    let bytes = rewrite_zip(
        &rewrite_zip(&xlsx, "xl/styles.xml", styles),
        "xl/worksheets/sheet5.xml",
        xml,
    );
    let dates = historical_dates(&bytes, Some(480)).unwrap();
    assert_eq!(dates[&2], "2027-01-01T10:00:00+08:00");
    assert!(dates[&3].ends_with("12:00:00+08:00"));
    assert_eq!(dates.len(), 2);
    let broken = rewrite_zip(&xlsx, "xl/worksheets/sheet5.xml", br#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData><row r="1"><c r="A1" t="n"><v>not-a-number</v></c></row></sheetData></worksheet>"#);
    assert_eq!(
        historical_dates(&broken, None).unwrap_err().code,
        Code::Unreadable
    );
}
#[test]
fn workbook_zip_limits_and_malformed_containers_are_refused_before_parsing_data(
) {
    use std::io::{Cursor, Write};
    let (sql, _) = fixture();
    refused_code(&sql, b"not a ZIP", &reviewed(), Code::Unreadable);
    refused_code(
        &sql,
        &vec![0; 32 * 1024 * 1024 + 1],
        &reviewed(),
        Code::Unreadable,
    );
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for i in 0..1025 {
        zip.start_file(
            format!("{i}"),
            zip::write::SimpleFileOptions::default(),
        )
        .unwrap();
    }
    let bytes = zip.finish().unwrap().into_inner();
    refused_code(&sql, &bytes, &reviewed(), Code::Unreadable);
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    zip.start_file("small", zip::write::SimpleFileOptions::default())
        .unwrap();
    zip.write_all(b"x").unwrap();
    let small = zip.finish().unwrap().into_inner();
    // ZIP declarations are checked before allocating/decompressing their payload.
    let mut oversized = small.clone();
    for (signature, offset) in [(b"PK\x03\x04", 22), (b"PK\x01\x02", 24)] {
        let at = oversized.windows(4).position(|p| p == signature).unwrap();
        oversized[at + offset..at + offset + 4]
            .copy_from_slice(&(64u32 * 1024 * 1024 + 1).to_le_bytes());
    }
    refused_code(&sql, &oversized, &reviewed(), Code::Unreadable);
    let mut encrypted = small.clone();
    for (signature, offset) in [(b"PK\x03\x04", 6), (b"PK\x01\x02", 8)] {
        let at = encrypted.windows(4).position(|p| p == signature).unwrap();
        encrypted[at + offset] |= 1;
    }
    refused_code(&sql, &encrypted, &reviewed(), Code::Unreadable);
    assert!(adapt(&sql, Some(&small), &reviewed()).is_err());
}

#[test]
fn review_binding_checks_workbook_bytes_even_when_sql_is_unchanged() {
    let (sql, xlsx) = fixture();
    let mut options = bound_options(&sql, &xlsx);
    options.election_mappings = vec![ElectionMapping {
        workbook_row: 2,
        seat_rows: vec![1],
    }];
    let changed = changed_workbook(&xlsx, |sheets| {
        set(
            &mut sheets
                .iter_mut()
                .find(|sheet| sheet.key == "electionevent")
                .unwrap()
                .rows[0],
            "description",
            json!("Updated supplemental context"),
        );
    });
    let problem =
        refused_code(&sql, &changed, &options, Code::IntegrityMismatch);
    assert!(problem.path.contains("sourceDigests"));
}
#[test]
fn explicit_candidate_scope_records_exclusions_without_inventing_affiliations()
{
    let (sql, xlsx) = fixture();
    let sql = String::from_utf8(sql)
        .unwrap()
        .replace("'S01','PR01'", "'S01',''");
    let sql = with_second_seat(sql.into_bytes(), "('S02','Second Seat',1,'G01')")
        + "INSERT INTO candidate VALUES ('0008','Outside example','C02','S02',NULL);";
    let options = first_seat_only(sql.as_bytes(), &xlsx);
    let adapted = adapt(sql.as_bytes(), Some(&xlsx), &options).unwrap();
    assert_eq!(adapted.summary.selected_candidates, 1);
    assert_eq!(adapted.summary.outside_scope_candidates, 1);
    assert_eq!(adapted.summary.unresolved_affiliations, 0);
    assert!(adapted.workbook.rows("candidates")[0]
        .get("annotations.miru:candidate-affiliation-id")
        .is_none());
    let mut strict = options;
    strict.scope = ScopePolicy::RequireCompleteCoverage;
    refused_code(sql.as_bytes(), &xlsx, &strict, Code::NotApproved);
}
#[test]
fn an_empty_source_census_never_requires_invented_voter_credentials() {
    let (sql, xlsx) = fixture();
    let sql = String::from_utf8(sql)
        .unwrap()
        .replace("INSERT INTO `voters` VALUES ('private-voter-id');", "");
    let mut options = reviewed();
    options.census = CensusPolicy::RequireReviewedMapping;
    options.scope = ScopePolicy::RequireCompleteCoverage;
    let adapted = adapt(sql.as_bytes(), Some(&xlsx), &options).unwrap();
    assert_eq!(adapted.summary.excluded_voters, 0);
    assert_eq!(adapted.summary.outside_scope_candidates, 0);
    assert!(adapted.workbook.rows("voters").is_empty());
}
#[test]
fn repeated_precinct_rows_with_identical_metadata_share_one_canonical_area() {
    let (mut sql, xlsx) = fixture();
    sql.extend_from_slice(
        b"INSERT INTO pop VALUES ('B01','P01','001','002','003');",
    );
    let adapted = adapt(&sql, Some(&xlsx), &reviewed()).unwrap();
    assert_eq!(adapted.summary.source_rows["pop"], 2);
    assert_eq!(adapted.summary.generated_rows["Areas"], 1);
    assert_eq!(adapted.summary.generated_rows["AreaContests"], 1);
    assert_eq!(adapted.summary.generated_rows["Candidates"], 1);
}

#[test]
fn each_required_source_table_reports_its_missing_column_before_conversion() {
    let (source, xlsx) = fixture();
    let source = String::from_utf8(source).unwrap();
    for (table, column) in [
        ("pop", "POLLCENTER_CODE"),
        ("polling_centers", "POLLING_PLACE"),
        ("allbgy", "ID_BARANGAY"),
        ("allmun", "ID_CITY"),
        ("trans_route", "TRANS_DEST_ID"),
        ("seat", "ELIGIBLEAMOUNT"),
        ("contest", "SORT_ORDER"),
        ("candidate", "CAND_CODE"),
        ("party_list", "CODE_PARTY"),
        ("geographical", "GEOCODE"),
    ] {
        let prefix = format!("CREATE TABLE `{table}` ");
        let declaration = source
            .lines()
            .find(|line| line.starts_with(&prefix))
            .unwrap();
        let marker = format!("`{column}`");
        assert!(declaration.contains(&marker));
        let changed = source.replace(
            declaration,
            &declaration.replacen(&marker, "`unavailable_column`", 1),
        );
        let problem = refused_code(
            changed.as_bytes(),
            &xlsx,
            &reviewed(),
            Code::MissingField,
        );
        assert!(problem.path.contains(table), "{table}: {}", problem.path);
        assert!(problem.path.contains(column), "{column}: {}", problem.path);
    }
}
#[test]
fn every_name_lands_in_the_localized_presentation_the_importer_keeps() {
    let (sql, xlsx) = fixture();
    let adapted = adapt(&sql, Some(&xlsx), &reviewed()).unwrap();
    let built = build(
        &adapted.workbook,
        &TemplateSet::builtin().unwrap(),
        &BuildOptions::default(),
        &Sources::default(),
    )
    .unwrap();
    let export = &built.export;
    for (entity, expected) in [
        (&export["election_event"], "Synthetic Event"),
        (&export["elections"][0], "Example Election"),
        (&export["contests"][0], "Example Seat"),
        (&export["candidates"][0], "Example Candidate"),
    ] {
        assert_eq!(
            entity["presentation"]["i18n"]["en"]["name"],
            json!(expected)
        );
        assert!(entity.get("name").is_none(), "{entity}");
    }
}
#[test]
fn a_plain_workbook_name_must_agree_with_its_localized_name() {
    let (sql, xlsx) = fixture();
    for sheet in ["electionevent", "elections"] {
        let localized = changed_workbook(&xlsx, |sheets| {
            let target = sheets.iter_mut().find(|s| s.key == sheet).unwrap();
            target.rows[0].cells.retain(|(c, _)| c != "name");
            set(
                &mut target.rows[0],
                "presentation.i18n.en.name",
                json!("Localized"),
            );
        });
        let adapted = adapt(&sql, Some(&localized), &reviewed()).unwrap();
        assert_eq!(
            adapted.workbook.rows(sheet)[0].text("presentation.i18n.en.name"),
            Some("Localized")
        );
        assert!(adapted.workbook.rows(sheet)[0].get("name").is_none());

        let agreeing = changed_workbook(&xlsx, |sheets| {
            let target = sheets.iter_mut().find(|s| s.key == sheet).unwrap();
            let plain = target.rows[0].text("name").unwrap().to_string();
            set(
                &mut target.rows[0],
                "presentation.i18n.en.name",
                json!(plain),
            );
        });
        adapt(&sql, Some(&agreeing), &reviewed()).unwrap();

        let conflicting = changed_workbook(&xlsx, |sheets| {
            let target = sheets.iter_mut().find(|s| s.key == sheet).unwrap();
            set(
                &mut target.rows[0],
                "presentation.i18n.en.name",
                json!("Another name"),
            );
        });
        refused_code(&sql, &conflicting, &reviewed(), Code::ConflictingColumns);
    }
}
fn built_export(sql: &[u8], xlsx: &[u8]) -> Value {
    let adapted = adapt(sql, Some(xlsx), &reviewed()).unwrap();
    build(
        &adapted.workbook,
        &TemplateSet::builtin().unwrap(),
        &BuildOptions::default(),
        &Sources::default(),
    )
    .unwrap()
    .export
}
fn localized<'a>(entity: &'a Value, language: &str, field: &str) -> &'a Value {
    &entity["presentation"]["i18n"][language][field]
}
#[test]
fn workbook_descriptions_land_in_the_localized_presentation_the_admin_portal_edits(
) {
    let (sql, xlsx) = fixture();
    let export = built_export(&sql, &xlsx);
    for entity in [&export["election_event"], &export["elections"][0]] {
        assert_eq!(localized(entity, "en", "description"), &json!("Test"));
        assert_eq!(entity["description"], json!("Test"), "{entity}");
    }
}
#[test]
fn a_plain_workbook_description_must_agree_with_its_localized_description() {
    let (sql, xlsx) = fixture();
    let description = "presentation.i18n.en.description";
    for sheet in ["electionevent", "elections"] {
        let localized_only = changed_workbook(&xlsx, |sheets| {
            let target = sheets.iter_mut().find(|s| s.key == sheet).unwrap();
            target.rows[0].cells.retain(|(c, _)| c != "description");
            set(&mut target.rows[0], description, json!("Localized"));
        });
        let adapted = adapt(&sql, Some(&localized_only), &reviewed()).unwrap();
        assert_eq!(
            adapted.workbook.rows(sheet)[0].text(description),
            Some("Localized")
        );

        let agreeing = changed_workbook(&xlsx, |sheets| {
            let target = sheets.iter_mut().find(|s| s.key == sheet).unwrap();
            set(&mut target.rows[0], description, json!("Test"));
        });
        adapt(&sql, Some(&agreeing), &reviewed()).unwrap();

        let conflicting = changed_workbook(&xlsx, |sheets| {
            let target = sheets.iter_mut().find(|s| s.key == sheet).unwrap();
            set(&mut target.rows[0], description, json!("Another text"));
        });
        let problem = refused_code(
            &sql,
            &conflicting,
            &reviewed(),
            Code::ConflictingColumns,
        );
        assert!(
            problem.path.ends_with("column 'description'"),
            "{}",
            problem.path
        );
    }
}
#[test]
fn names_go_to_each_rows_default_language_beside_its_existing_translations() {
    let (sql, xlsx) = fixture();
    let xlsx = changed_workbook(&xlsx, |sheets| {
        for sheet in sheets.iter_mut().filter(|s| {
            ["electionevent", "elections"].contains(&s.key.as_str())
        }) {
            let row = &mut sheet.rows[0];
            set(
                row,
                "presentation.language_conf.default_language_code",
                json!("es"),
            );
            set(row, "presentation.i18n.tl.name", json!("Pangalan"));
        }
    });
    let export = built_export(&sql, &xlsx);
    for (entity, name, description) in [
        (&export["election_event"], "Synthetic Event", Some("Test")),
        (&export["elections"][0], "Example Election", Some("Test")),
        (&export["contests"][0], "Example Seat", None),
        (&export["candidates"][0], "Example Candidate", None),
    ] {
        assert_eq!(localized(entity, "es", "name"), &json!(name), "{entity}");
        if let Some(description) = description {
            assert_eq!(
                localized(entity, "es", "description"),
                &json!(description)
            );
            assert_eq!(localized(entity, "tl", "name"), &json!("Pangalan"));
        }
        assert!(localized(entity, "en", "name").is_null(), "{entity}");
        assert!(localized(entity, "en", "description").is_null(), "{entity}");
    }
}
#[test]
fn an_entity_left_without_a_name_in_its_default_language_is_refused_where_it_is(
) {
    let (sql, xlsx) = fixture();
    for sheet in ["electionevent", "elections"] {
        let nameless = changed_workbook(&xlsx, |sheets| {
            let target = sheets.iter_mut().find(|s| s.key == sheet).unwrap();
            target.rows[0].cells.retain(|(c, _)| c != "name");
        });
        let problem =
            refused_code(&sql, &nameless, &reviewed(), Code::MissingField);
        assert!(problem.path.ends_with("column 'name'"), "{}", problem.path);

        let blank = changed_workbook(&xlsx, |sheets| {
            let target = sheets.iter_mut().find(|s| s.key == sheet).unwrap();
            set(&mut target.rows[0], "name", json!("  "));
        });
        refused_code(&sql, &blank, &reviewed(), Code::MissingField);

        let other_language_only = changed_workbook(&xlsx, |sheets| {
            let target = sheets.iter_mut().find(|s| s.key == sheet).unwrap();
            target.rows[0].cells.retain(|(c, _)| c != "name");
            set(&mut target.rows[0], "presentation.i18n.tl.name", json!("X"));
        });
        let problem = refused_code(
            &sql,
            &other_language_only,
            &reviewed(),
            Code::MissingField,
        );
        assert!(problem.message.contains("en"), "{}", problem.message);
    }
    let source = String::from_utf8(sql).unwrap();
    for (replacements, path) in [
        (
            vec![
                ("('S01','Example Seat',1,'G01')", "('S01',' ',1,'G01')"),
                ("('C01','Example Seat',1)", "('C01',' ',1)"),
            ],
            "seat row 1 column CONTEST",
        ),
        (
            vec![("'Example Candidate','C01'", "'','C01'")],
            "candidate row 1 column NAMEONBALLOT",
        ),
    ] {
        let mut modified = source.clone();
        for (from, to) in replacements {
            assert!(modified.contains(from), "fixture replacement {from}");
            modified = modified.replace(from, to);
        }
        let problem = refused_code(
            modified.as_bytes(),
            &xlsx,
            &reviewed(),
            Code::MissingField,
        );
        assert!(problem.path.ends_with(path), "{path}: {}", problem.path);
    }
}
#[test]
fn names_and_descriptions_survive_the_native_schema_and_a_reopened_archive() {
    let (sql, xlsx) = fixture();
    let adapted = adapt(&sql, Some(&xlsx), &reviewed()).unwrap();
    let built = build(
        &adapted.workbook,
        &TemplateSet::builtin().unwrap(),
        &BuildOptions::default(),
        &Sources::default(),
    )
    .unwrap();
    let native: crate::election_config::ImportElectionEventSchema =
        serde_json::from_value(built.export.clone()).unwrap();
    let reserialized = serde_json::to_value(&native).unwrap();
    for (collection, name) in [
        ("elections", "Example Election"),
        ("contests", "Example Seat"),
        ("candidates", "Example Candidate"),
    ] {
        assert_eq!(
            localized(&reserialized[collection][0], "en", "name"),
            &json!(name)
        );
    }
    assert_eq!(
        localized(&reserialized["election_event"], "en", "name"),
        &json!("Synthetic Event")
    );

    let zip = archive::zip(&archive::layout(&built).importable).unwrap();
    let opened = crate::election_config::open::open(&zip).unwrap();
    let plan = &opened.plan;
    let en = |translated: &crate::election_config::architect::Translated| {
        translated.by_language.get("en").cloned()
    };
    assert_eq!(en(&plan.name).as_deref(), Some("Synthetic Event"));
    assert_eq!(en(&plan.description).as_deref(), Some("Test"));
    let election = &plan.elections[0];
    assert_eq!(en(&election.name).as_deref(), Some("Example Election"));
    assert_eq!(en(&election.description).as_deref(), Some("Test"));
    let contest = &election.contests[0];
    assert_eq!(en(&contest.name).as_deref(), Some("Example Seat"));
    assert_eq!(
        en(&contest.candidates[0].name).as_deref(),
        Some("Example Candidate")
    );
    assert_eq!(
        contest.candidates[0].external_id,
        "center:P01:context:e1:seat:S01:candidate:0007"
    );
}

#[test]
fn embassy_labels_are_formatted_as_the_registration_options_are() {
    // Parenthetical segments drop, a short last word is upper-cased, and the
    // special abbreviations stay upper-case wherever they appear.
    assert_eq!(
        format_embassy("washington (embassy) d.c. usa"),
        "Washington D.C. USA"
    );
    assert_eq!(format_embassy("hong kong sar china"), "Hong Kong SAR China");
    assert_eq!(
        format_embassy("taipei roc (teco) office"),
        "Taipei ROC Office"
    );
    assert_eq!(format_embassy("  ((nested) note)  "), "");
    assert_eq!(format_embassy(""), "");
}

#[test]
fn a_date_of_birth_is_composed_only_from_a_valid_complete_date() {
    assert_eq!(compose_date_of_birth("1981", "2", "29"), Err(()));
    assert_eq!(
        compose_date_of_birth(" 1984 ", "02", "29"),
        Ok(Some("1984-02-29".to_string()))
    );
    assert_eq!(compose_date_of_birth("", " ", ""), Ok(None));
    assert_eq!(compose_date_of_birth("1984", "", "1"), Err(()));
    assert_eq!(compose_date_of_birth("19x4", "1", "1"), Err(()));
}

/// The overseas fixture's SQL with its `voters` statement replaced by `voters`.
fn with_voters(sql: Vec<u8>, voters: &str) -> Vec<u8> {
    let sql = String::from_utf8(sql).unwrap();
    let kept: Vec<&str> = sql
        .lines()
        .filter(|line| !line.starts_with("CREATE TABLE `voters`"))
        .collect();
    (kept.join("\n") + "\n" + voters + "\n").into_bytes()
}

#[test]
fn a_source_without_the_optional_identity_columns_imports_them_blank() {
    // Another client's census carries no maternal name, suffix or date of birth
    // columns at all: the voters import, those attributes left blank.
    let (sql, xlsx) = overseas_voter_fixture("", "");
    let values: Vec<String> = (0..4)
        .map(|i| {
            format!("('w-{i}','First{i}','Last{i}','90','06','0001A','O')")
        })
        .collect();
    let sql = with_voters(
        sql,
        &format!(
            "CREATE TABLE `voters` (`ID` text, `FIRSTNAME` text, `LASTNAME` text, `COUNTRY` text, `EMBASSY` text, `PRECINCT` text, `ABROADSTATUS` text); INSERT INTO `voters` VALUES {};",
            values.join(",")
        ),
    );
    let adapted = adapt(&sql, Some(&xlsx), &preserving()).unwrap();
    assert_eq!(adapted.summary.preserved_voters, 4);
    let voters = adapted.workbook.rows("voters");
    let voter = voters
        .iter()
        .find(|r| r.text("username") == Some("w-0"))
        .unwrap();
    for attribute in ["middleName", "suffix", "dateOfBirth"] {
        assert_eq!(
            voter.get(attribute).and_then(Value::as_str),
            Some(""),
            "{attribute}"
        );
    }
}

#[test]
fn preserving_voters_needs_the_population_precinct_column() {
    // The voter area join reads pop's PRECINCT; a source without it is refused
    // by name rather than joined on a guess.
    let (sql, xlsx) = overseas_voter_fixture("", "");
    let sql = String::from_utf8(sql).unwrap();
    let start = sql.find("CREATE TABLE `pop`").unwrap();
    let end = start + sql[start..].find('\n').unwrap();
    let pop = "CREATE TABLE `pop` (`CLUSTERPOLLCENTER` text, `POLLCENTER_CODE` text, `PROV_CODE` text, `MUN_CODE` text, `BRGY_CODE` text); INSERT INTO `pop` VALUES ('B06','P01','90','06','001');";
    let sql = format!("{}{}{}", &sql[..start], pop, &sql[end..]);
    let refused =
        adapt(sql.as_bytes(), Some(&xlsx), &preserving()).unwrap_err();
    let problem = &refused.report.problems[0];
    assert!(
        problem.path.contains("pop") && problem.path.contains("PRECINCT"),
        "{problem:?}"
    );
}

#[test]
fn a_report_lists_each_problem_on_its_own_line() {
    let mut report =
        Report::from(Problem::error(Code::MissingField, "a.b", "first"));
    report.push(Problem::error(Code::InvalidValue, "c", "second"));
    let text = report.to_string();
    assert_eq!(text.lines().count(), 2);
    assert!(text.lines().all(|line| line.starts_with("  ")));
    assert!(text.contains("first") && text.contains("second"));
}

#[test]
fn a_voter_whose_area_has_no_embassy_label_is_refused() {
    // Enrollment matches the voter's embassy against the registration options;
    // an area without a label would match against a blank, so it is refused.
    let (sql, xlsx) = overseas_voter_fixture_with("", "", "");
    let refused = adapt(&sql, Some(&xlsx), &preserving()).unwrap_err();
    let problems = &refused.report.problems;
    assert!(!problems.is_empty());
    assert!(problems
        .iter()
        .all(|p| p.code == Code::MissingField
            && p.path.ends_with("column EMBASSY")));
    // Only the 20 voters of the unlabelled area, each by its own row.
    assert_eq!(problems.len(), 20);
}

/// The legacy SQL rewritten in the documented post-qualification names.
fn post_qualification(sql: &[u8]) -> String {
    let mut sql = String::from_utf8(sql.to_vec()).unwrap();
    for (legacy, post) in [
        ("candidate", "candidates"),
        ("contest", "contests"),
        ("seat", "seats"),
        ("party_list", "political_party"),
        ("canvass_center", "canvass_centers"),
        ("trans_route", "trans_routes"),
        ("geographical", "jurisdictions"),
        ("ELIGIBLEAMOUNT", "NUM_SEAT"),
    ] {
        sql = sql.replace(&format!("`{legacy}`"), &format!("`{post}`"));
    }
    sql + "CREATE TABLE `trans_servers` (`ID` int, `CODE_SERVER` text); INSERT INTO `trans_servers` VALUES (1,'S1');\n"
}

#[test]
fn a_post_qualification_dump_converts_as_its_legacy_equivalent() {
    let (sql, xlsx) = fixture();
    let post = post_qualification(&sql);
    let legacy = adapt(&sql, Some(&xlsx), &reviewed()).unwrap();
    let renamed = adapt(post.as_bytes(), Some(&xlsx), &reviewed()).unwrap();
    assert_eq!(legacy.summary.source_schema, SourceSchema::Legacy);
    assert_eq!(
        renamed.summary.source_schema,
        SourceSchema::PostQualification
    );
    // The same rows; only the provenance differs, recording the source's own names.
    let cells = |workbook: &Workbook| -> Vec<Vec<(String, Value)>> {
        workbook
            .sheets()
            .iter()
            .flat_map(|sheet| sheet.rows.iter())
            .map(|row| row.without(&["annotations.import:provenance"]))
            .collect()
    };
    assert_eq!(cells(&renamed.workbook), cells(&legacy.workbook));
    let provenance: Value = serde_json::from_str(
        renamed.workbook.rows("electionevent")[0]
            .text("annotations.import:provenance")
            .unwrap(),
    )
    .unwrap();
    assert_eq!(provenance["sourceSchema"], json!("post-qualification"));
    assert_eq!(provenance["sourceRows"]["seats"], json!(1));
}

#[test]
fn a_dump_mixing_both_schemas_is_refused() {
    let (sql, xlsx) = fixture();
    let mixed = post_qualification(&sql).replace("`contests`", "`contest`");
    let refused =
        adapt(mixed.as_bytes(), Some(&xlsx), &reviewed()).unwrap_err();
    assert_eq!(refused.report.problems[0].code, Code::InvalidValue);
    assert_eq!(refused.report.problems[0].path, "SQL schema");
}

#[test]
fn post_qualification_problems_name_the_source_tables_and_columns() {
    let (sql, xlsx) = fixture();
    let post = post_qualification(&sql).replace(
        "('S01','Example Seat',1,'G01')",
        "('S01','Example Seat',NULL,'G01')",
    );
    let refused = adapt(post.as_bytes(), Some(&xlsx), &reviewed()).unwrap_err();
    let path = &refused.report.problems[0].path;
    assert!(
        path.starts_with("SQL table seats ")
            && path.ends_with("column NUM_SEAT"),
        "{path}"
    );

    let report = schema::source_names(
        Report::from(Problem::error(
            Code::InvalidValue,
            "SQL table pop row 2 column MACHINE_TYPE",
            "x",
        )),
        SourceSchema::PostQualification,
    );
    assert_eq!(
        report.problems[0].path,
        "SQL table pop row 2 column VOTING_MODE"
    );
}

/// The overseas source in the post-qualification schema: voters carry their mail
/// country and embassy, and no precinct or abroad status. Each cluster has its own
/// polling center, whose post is the area's embassy; `UNITED STATES` is named after
/// its own post, so its area takes the municipality's name.
fn post_qualification_overseas() -> Vec<u8> {
    let (sql, _) = overseas_voter_fixture("", "");
    let mut sql = post_qualification(&sql)
        .replace(
            "('P01','Post One');",
            "('P01','Post One'),('B06','BANGKOK PE'),('B02','DHAKA PE'),('B91','WASHINGTON D.C.');",
        )
        .replace("('B91','UNITED STATES')", "('B91','WASHINGTON D.C.')");
    let start = sql.find("CREATE TABLE `voters`").unwrap();
    let end = start + sql[start..].find(";\n").unwrap() + 2;
    let voters: Vec<String> = (0..80usize)
        .map(|i| {
            let (country, embassy) =
                [("90", "06"), ("90", "06"), ("90", "02"), ("91", "01")][i / 20];
            format!("('v-{i:04}','Last{i}','First{i}','Mid{i}','','F','1990','1','2',NULL,'000','x@example.org','{country}','{embassy}',NULL)")
        })
        .collect();
    sql.replace_range(start..end, &format!("CREATE TABLE `voters` (`ID` text, `LASTNAME` text, `FIRSTNAME` text, `MATERNALNAME` text, `SUFFIX` text, `SEX` text, `DOBYEAR` text, `DOBMONTH` text, `DOBDAY` text, `VOTING_HIST1` text, `MOBILE` text, `EMAIL` text, `MAILCOUNTRY` text, `MAILEMBASSY` text, `GEOLOGY` text); INSERT INTO `voters` VALUES {};\n", voters.join(",")));
    sql.into_bytes()
}

fn supplemented() -> HistoricalOptions {
    HistoricalOptions {
        voter_area_join: Some(VoterAreaJoin::MailCountryEmbassy),
        schedule_offset_minutes: Some(480),
        supplement: Some(Supplement {
            event: SupplementEvent {
                name: "Synthetic Event".into(),
                election_event_id: "evt-001".into(),
                description: None,
                logo_url: None,
                css: None,
            },
            schedules: vec![
                SupplementSchedule {
                    election: None,
                    event_type: "START_VOTING_PERIOD".into(),
                    date: "2027-01-01T10:00".into(),
                },
                SupplementSchedule {
                    election: Some("Post One".into()),
                    event_type: "END_VOTING_PERIOD".into(),
                    date: "2027-01-02T10:00:00+08:00".into(),
                },
            ],
            ccs_servers: vec![SupplementCcsServer {
                name: "Central".into(),
                tag: "9701".into(),
                address: "https://ccs.example.org".into(),
                public_key: "key".into(),
                send_logs: false,
            }],
            area_ccs_server_tags: Some("9701".into()),
            area_trustee_servers: None,
        }),
        ..preserving()
    }
}

#[test]
fn a_supplement_replaces_the_workbook_and_the_mail_location_joins_voters() {
    let sql = post_qualification_overseas();
    let adapted = adapt(&sql, None, &supplemented()).unwrap();
    assert_eq!(adapted.summary.workbook_sha256, None);
    assert_eq!(adapted.summary.preserved_voters, 80);

    let templates = TemplateSet::builtin().unwrap();
    let built = build(
        &adapted.workbook,
        &templates,
        &BuildOptions::default(),
        &Sources::default(),
    )
    .unwrap();
    let event = &built.export["election_event"];
    assert_eq!(
        event["presentation"]["i18n"]["en"]["name"],
        json!("Synthetic Event")
    );
    let elections = built.export["elections"].as_array().unwrap();
    assert_eq!(elections.len(), 1, "one election per post with voters");
    // A schedule without an election is every election's; a local time takes the
    // reviewed offset.
    let dates: Vec<&str> = adapted
        .workbook
        .rows("scheduledevents")
        .iter()
        .map(|row| row.text("scheduled_datetime").unwrap())
        .collect();
    assert_eq!(
        dates,
        ["2027-01-01T10:00:00+08:00", "2027-01-02T10:00:00+08:00"]
    );

    let v = &built.voters;
    let col = |name: &str| v.column(name).unwrap();
    let (area, embassy) = (col("area_name"), col("embassy"));
    for (prefix, expected_area, expected_embassy) in [
        ("90/06", "KINGDOM OF THAILAND", "Bangkok PE"),
        ("90/02", "PEOPLES REPUBLIC OF BANGLADESH", "Dhaka PE"),
        ("91/01", "City C", "Washington D.C."),
    ] {
        let (c, e) = prefix.split_once('/').unwrap();
        let matching: Vec<_> = v
            .rows
            .iter()
            .filter(|r| {
                r[col("source_country")] == c && r[col("source_embassy")] == e
            })
            .collect();
        assert!(!matching.is_empty(), "{prefix}");
        assert!(
            matching
                .iter()
                .all(|r| r[area] == expected_area
                    && r[embassy] == expected_embassy),
            "{prefix} must map to {expected_area} / {expected_embassy}"
        );
    }
    // Contact and sex stay in the source: nothing beyond the reviewed census.
    assert!(v.rows.iter().all(|r| r[col("email")].is_empty()));
    assert!(v.column("mobile").is_none());
}

#[test]
fn a_supplement_needs_an_event_name_and_identity() {
    let sql = post_qualification_overseas();
    let mut options = supplemented();
    options.supplement.as_mut().unwrap().event.election_event_id = " ".into();
    let refused = adapt(&sql, None, &options).unwrap_err();
    assert_eq!(refused.report.problems[0].code, Code::MissingField);
    assert!(refused.report.problems[0]
        .path
        .ends_with("event.electionEventId"));
}

#[test]
fn the_workbook_and_the_supplement_are_one_or_the_other() {
    let (sql, xlsx) = fixture();
    let both = adapt(&sql, Some(&xlsx), &supplemented()).unwrap_err();
    assert_eq!(both.report.problems[0].code, Code::ConflictingColumns);
    let neither = adapt(&sql, None, &reviewed()).unwrap_err();
    assert_eq!(neither.report.problems[0].code, Code::NotApproved);
}

#[test]
fn supplement_decisions_roundtrip_as_strict_json() {
    let json = serde_json::to_value(&supplemented()).unwrap();
    assert_eq!(json["voterAreaJoin"], json!("mail-country-embassy"));
    assert_eq!(
        json["supplement"]["event"]["electionEventId"],
        json!("evt-001")
    );
    let back: HistoricalOptions = serde_json::from_value(json.clone()).unwrap();
    assert_eq!(serde_json::to_value(&back).unwrap(), json);
    let mut unknown = json;
    unknown["supplement"]["event"]["extra"] = json!(1);
    assert!(serde_json::from_value::<HistoricalOptions>(unknown).is_err());
}

#[test]
fn a_supplement_carries_the_optional_event_and_area_metadata() {
    let sql = post_qualification_overseas();
    let mut options = supplemented();
    let supplement = options.supplement.as_mut().unwrap();
    supplement.event.description = Some("Synthetic description".into());
    supplement.event.logo_url = Some("https://example.org/logo.png".into());
    supplement.event.css = Some("body {}".into());
    supplement.area_trustee_servers = Some("trustee1,trustee2".into());
    let adapted = adapt(&sql, None, &options).unwrap();
    let event = &adapted.workbook.rows("electionevent")[0];
    assert_eq!(event.text("description"), Some("Synthetic description"));
    assert_eq!(
        event.text("presentation.logo_url"),
        Some("https://example.org/logo.png")
    );
    assert_eq!(event.text("presentation.css"), Some("body {}"));
    for area in adapted.workbook.rows("areas") {
        assert_eq!(
            area.text("annotations.miru_trustee_servers"),
            Some("trustee1,trustee2")
        );
    }
}

#[test]
fn a_supplement_refuses_a_source_it_cannot_derive_elections_and_areas_from() {
    let sql = String::from_utf8(post_qualification_overseas()).unwrap();
    for (from, to, expected) in [
        ("`pop`", "`pop_missing`", Code::MissingField),
        ("`polling_centers`", "`centers_missing`", Code::MissingField),
        ("`allbgy`", "`allbgy_missing`", Code::MissingField),
        ("`allmun`", "`allmun_missing`", Code::MissingField),
        (
            "('B06','P01','90','06'",
            "('B06','P99','90','06'",
            Code::DanglingReference,
        ),
        (
            "('B06','KINGDOM OF THAILAND')",
            "('B07','KINGDOM OF THAILAND')",
            Code::DanglingReference,
        ),
        (
            "('9101','City C')",
            "('9102','City C')",
            Code::DanglingReference,
        ),
    ] {
        assert!(sql.contains(from), "{from}");
        let changed = sql.replace(from, to);
        let refused =
            adapt(changed.as_bytes(), None, &supplemented()).unwrap_err();
        assert_eq!(refused.report.problems[0].code, expected, "{from}");
    }
}

#[test]
fn post_qualification_renames_reach_keys_charsets_and_bare_table_paths() {
    let dump = crate::election_config::sql_dump::read_mysql_dump(
        b"CREATE TABLE `seats` (`DISTRICTCODE` text, `NUM_SEAT` int CHARACTER SET latin1, PRIMARY KEY (`NUM_SEAT`), UNIQUE KEY u (`NUM_SEAT`));\n",
    )
    .unwrap();
    let (dump, schema) = schema::normalize(dump).unwrap();
    assert_eq!(schema, SourceSchema::PostQualification);
    let seat = &dump.tables["seat"];
    assert_eq!(seat.columns, ["DISTRICTCODE", "ELIGIBLEAMOUNT"]);
    assert_eq!(seat.primary_key, ["ELIGIBLEAMOUNT"]);
    assert!(seat
        .unique_keys
        .iter()
        .all(|key| key == &["ELIGIBLEAMOUNT"]));
    assert!(seat.column_charsets.contains_key("ELIGIBLEAMOUNT"));

    let report = schema::source_names(
        Report::from(Problem::error(Code::MissingField, "SQL table seat", "x")),
        SourceSchema::PostQualification,
    );
    assert_eq!(report.problems[0].path, "SQL table seats");
}

#[test]
fn a_supplement_refuses_workbook_associations_and_unreadable_dates() {
    let sql = post_qualification_overseas();
    let mut options = supplemented();
    options.election_mappings = vec![ElectionMapping {
        workbook_row: 2,
        seat_rows: vec![1],
    }];
    let refused = adapt(&sql, None, &options).unwrap_err();
    assert_eq!(refused.report.problems[0].code, Code::InvalidValue);
    assert!(refused.report.problems[0].path.ends_with("areaMappings"));

    let mut options = supplemented();
    options.supplement.as_mut().unwrap().schedules[0].date =
        "next tuesday".into();
    let refused = adapt(&sql, None, &options).unwrap_err();
    assert_eq!(refused.report.problems[0].code, Code::InvalidValue);
    assert!(refused.report.problems[0]
        .message
        .contains("an instant or a local date and time"));
}
