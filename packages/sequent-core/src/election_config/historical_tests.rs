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
        adapt(&sql, &xlsx, &HistoricalOptions::default()).unwrap_err();
    assert_eq!(refused.report.problems[0].code, Code::NotApproved);
    assert!(refused.report.problems[0].path.contains("voters"));
    assert!(!format!("{:?}", refused.report).contains("private-voter-id"));
    assert_eq!(refused.summary.source_rows["voters"], 1);
}
#[test]
fn preserves_codes_reconciles_explicit_exclusions_and_builds_repeatable_native_archive(
) {
    let (sql, xlsx) = fixture();
    let adapted = adapt(&sql, &xlsx, &reviewed()).unwrap();
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
#[test]
fn refuses_a_dangling_precinct_reference_before_building() {
    let (sql, xlsx) = fixture();
    let sql = String::from_utf8(sql)
        .unwrap()
        .replace("('B01','P01','001'", "('B01','MISSING','001'");
    let refused = adapt(sql.as_bytes(), &xlsx, &reviewed()).unwrap_err();
    assert_eq!(refused.report.problems[0].code, Code::DanglingReference);
    assert!(!refused.report.problems[0].message.contains("MISSING"));
}
#[test]
fn outside_seat_candidates_require_explicit_scope_review() {
    let (mut sql, xlsx) = fixture();
    sql.extend_from_slice(
        b"INSERT INTO `candidate` VALUES ('0008','Another Example','C01','S02',NULL);",
    );
    let mut options = reviewed();
    options.scope = ScopePolicy::RequireCompleteCoverage;
    let refused = adapt(&sql, &xlsx, &options).unwrap_err();
    assert_eq!(refused.report.problems[0].code, Code::NotApproved);
    assert_eq!(refused.summary.outside_scope_candidates, 1);
}
#[test]
fn malformed_sql_reports_hashes_and_location_without_any_workbook_read() {
    let refused = adapt(
        b"SELECT 'private-record' INTO OUTFILE '/tmp/data';",
        b"invalid",
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
    let refused = adapt(&sql, &xlsx, &reviewed()).unwrap_err();
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
    let accepted = adapt(&sql, &xlsx, &options).unwrap();
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
    let refused = adapt(&sql, &xlsx, &reviewed()).unwrap_err();
    assert_eq!(refused.report.problems[0].code, Code::NotApproved);
    assert!(refused.report.problems[0].path.contains("column date"));
    let mut options = reviewed();
    options.schedule_offset_minutes = Some(480);
    let accepted = adapt(&sql, &xlsx, &options).unwrap();
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
        workbook_sha256: format!("{:x}", Sha256::digest(&xlsx)),
    });
    assert!(adapt(&sql, &xlsx, &options).is_ok());
    let changed = [
        sql.as_slice(),
        b"
-- source changed
",
    ]
    .concat();
    let refused = adapt(&changed, &xlsx, &options).unwrap_err();
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
        workbook_sha256: format!("{:x}", Sha256::digest(&xlsx)),
    });
    let refused = adapt(&sql, &xlsx, &options).unwrap_err();
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
    let rejected = adapt(&sql, &xlsx, &options).unwrap_err();
    assert_eq!(rejected.report.problems[0].code, Code::DanglingReference);
    options.source_digests = Some(SourceDigests {
        sql_sha256: format!("{:x}", Sha256::digest(&sql)),
        workbook_sha256: format!("{:x}", Sha256::digest(&xlsx)),
    });
    options.area_mappings = vec![AreaMapping {
        workbook_row: 2,
        precinct_rows: vec![1],
    }];
    let adapted = adapt(&sql, &xlsx, &options).unwrap();
    assert_eq!(
        adapted.workbook.rows("areas")[0].text("name"),
        Some("Reviewed area label")
    );
    assert_eq!(adapted.workbook.rows("areacontests").len(), 1);
    options.source_digests = None;
    assert_eq!(
        adapt(&sql, &xlsx, &options).unwrap_err().report.problems[0].code,
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
        adapt(&sql, &xlsx, &options).unwrap_err().report.problems[0].code,
        Code::BallotCoverage
    );
    options.post_coverage = PostCoverage::WorkbookPostsOnly;
    options.source_digests = Some(SourceDigests {
        sql_sha256: format!("{:x}", Sha256::digest(&sql)),
        workbook_sha256: format!("{:x}", Sha256::digest(&xlsx)),
    });
    let accepted = adapt(&sql, &xlsx, &options).unwrap();
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
        adapt(&sql, &xlsx, &options).unwrap_err().report.problems[0].code,
        Code::DanglingReference
    );
    options.precinct_coverage = PostCoverage::WorkbookPostsOnly;
    options.source_digests = Some(SourceDigests {
        sql_sha256: format!("{:x}", Sha256::digest(&sql)),
        workbook_sha256: format!("{:x}", Sha256::digest(&xlsx)),
    });
    let adapted = adapt(&sql, &xlsx, &options).unwrap();
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
        adapt(sql.as_bytes(), &xlsx, &options)
            .unwrap_err()
            .report
            .problems[0]
            .code,
        Code::DanglingReference
    );
    options.affiliation = AffiliationPolicy::PreserveUnresolvedCode;
    options.source_digests = Some(SourceDigests {
        sql_sha256: format!("{:x}", Sha256::digest(sql.as_bytes())),
        workbook_sha256: format!("{:x}", Sha256::digest(&xlsx)),
    });
    let accepted = adapt(sql.as_bytes(), &xlsx, &options).unwrap();
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
    let refused = adapt(&sql, &xlsx, &reviewed()).unwrap_err();
    assert_eq!(refused.report.problems[0].code, Code::NotApproved);
    assert!(refused.report.problems[0]
        .message
        .contains("multiple SQL centers"));
}
