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
            workbook_sha256: format!("{:x}", Sha256::digest(xlsx)),
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
    let rejected = adapt(sql, xlsx, options).unwrap_err();
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
    let adapted = adapt(sql.as_bytes(), &xlsx, &reviewed()).unwrap();
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
    let adapted = adapt(&sql, &xlsx, &reviewed()).unwrap();
    assert_eq!(
        adapted.workbook.rows("candidates")[0].text("name"),
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
        adapt(override_sql.as_bytes(), &xlsx, &reviewed())
            .unwrap()
            .workbook
            .rows("candidates")[0]
            .text("name"),
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
    let adapted = adapt(changed_sql.as_bytes(), &changed, &reviewed()).unwrap();
    assert_eq!(adapted.summary.precincts[0].workbook_area_rows, [2]);
    assert_eq!(
        adapted.workbook.rows("areas")[0].text("name"),
        Some("City One")
    );
    let extra=format!("{source} INSERT INTO polling_centers VALUES ('P02','Unused post'); INSERT INTO pop VALUES ('B01','P02','001','002','003');");
    let mut options = bound_options(extra.as_bytes(), &xlsx);
    options.post_coverage = PostCoverage::WorkbookPostsOnly;
    let adapted = adapt(extra.as_bytes(), &xlsx, &options).unwrap();
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
        let adapted = adapt(&sql, &changed, &reviewed()).unwrap();
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
    let adapted = adapt(&sql, &xlsx, &reviewed()).unwrap();
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
        adapt(&sql, &naive, &options)
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
    assert!(adapt(&sql, &small, &reviewed()).is_err());
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
    let mut sql = String::from_utf8(sql)
        .unwrap()
        .replace("'S01','PR01'", "'S01',''");
    sql.push_str("INSERT INTO candidate VALUES ('0008','Outside example','C01','S02',NULL);");
    let adapted = adapt(sql.as_bytes(), &xlsx, &reviewed()).unwrap();
    assert_eq!(adapted.summary.selected_candidates, 1);
    assert_eq!(adapted.summary.outside_scope_candidates, 1);
    assert_eq!(adapted.summary.unresolved_affiliations, 0);
    assert!(adapted.workbook.rows("candidates")[0]
        .get("annotations.miru:candidate-affiliation-id")
        .is_none());
    let mut strict = reviewed();
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
    let adapted = adapt(sql.as_bytes(), &xlsx, &options).unwrap();
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
    let adapted = adapt(&sql, &xlsx, &reviewed()).unwrap();
    assert_eq!(adapted.summary.source_rows["pop"], 2);
    assert_eq!(adapted.summary.generated_rows["Areas"], 1);
    assert_eq!(adapted.summary.generated_rows["AreaContests"], 1);
    assert_eq!(adapted.summary.generated_rows["Candidates"], 1);
}
