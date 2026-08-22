// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! One door, three formats — and the reason the telling-apart is in Rust.

use super::*;
use crate::election_config::architect::{
    compile_plan, to_workbook, BLUEPRINT_VERSION,
};
use crate::election_config::build::BuildOptions;
use crate::election_config::render::TemplateSet;
use crate::election_config::sources::Sources;
use crate::election_config::xlsx_write::write_xlsx;
use crate::election_config::Workbook;

/// `to_workbook` against the census and files the plan is still carrying.
///
/// One function rather than every call site: when the census leaves `Blueprint`,
/// this is where a test says where its voters come from.
fn workbook_of(plan: &Blueprint) -> Result<Workbook, Problem> {
    to_workbook(plan, &Sources::from_plan(plan))
}

fn sound() -> Blueprint {
    serde_json::from_value(serde_json::json!({
        "version": BLUEPRINT_VERSION,
        "external_id": "union-2027",
        "name": {"en": "Union Election 2027"},
        "languages": ["en"],
        "trustees": [
            {"name": "Ada Lovelace", "email": "ada@example.org"},
            {"name": "Grace Hopper", "email": "grace@example.org"}
        ],
        "elections": [{
            "external_id": "officers",
            "name": {"en": "Officers"},
            "contests": [{
                "external_id": "president",
                "name": {"en": "President"},
                "max_votes": 1,
                "winners": 1,
                "candidates": [
                    {"external_id": "alice", "name": {"en": "Alice Okonjo"}},
                    {"external_id": "bob", "name": {"en": "Bob Iyer"}}
                ]
            }]
        }]
    }))
    .unwrap()
}

/// Whether a report says something, by the words a reader would see.
fn says(report: &Report, needle: &str) -> bool {
    report
        .problems
        .iter()
        .any(|problem| problem.message.contains(needle))
}

fn delivery_of(plan: &Blueprint) -> Vec<u8> {
    let compiled = compile_plan(
        plan,
        &TemplateSet::builtin().unwrap(),
        &BuildOptions::default(),
        None,
    )
    .expect("the sample plan compiles");
    super::super::archive::delivery(&compiled.layout)
        .expect("and packs")
        .bytes
}

#[test]
fn a_delivery_opens_as_the_plan_that_built_it() {
    let plan = sound();
    let opened = open(&delivery_of(&plan)).expect("a delivery is a plan");

    assert_eq!(opened.source, Source::Delivery);
    assert_eq!(opened.plan.external_id, plan.external_id);
    assert_eq!(opened.plan.trustees.len(), 2);
}

#[test]
fn a_bare_plan_file_opens() {
    let plan = sound();
    let text = serde_json::to_vec(&plan).unwrap();

    let opened = open(&text).expect("a plan is a plan");
    assert_eq!(opened.source, Source::Plan);
    assert_eq!(opened.plan.external_id, "union-2027");
}

/// The case that cannot be told apart outside Rust.
///
/// A spreadsheet has the same `PK` magic as a delivery, so a front end sniffing
/// bytes hands it to the delivery reader, which looks for `blueprint.json`, does
/// not find it, and reports a broken delivery about a perfectly good workbook.
#[test]
fn a_workbook_opens_as_a_plan_rather_than_as_a_broken_delivery() {
    let plan = sound();
    let bytes = write_xlsx(&workbook_of(&plan).unwrap()).unwrap();

    assert!(
        is_zip(&bytes),
        "an .xlsx is a zip, which is the whole problem"
    );

    let opened = open(&bytes).expect("and it is a workbook");
    assert_eq!(opened.source, Source::Workbook);
    assert_eq!(opened.plan.external_id, "union-2027");
    assert_eq!(opened.plan.elections[0].contests[0].candidates.len(), 2);
}

/// The workbook inside a delivery opens as one.
///
/// The round trip a client actually does: build, unzip, edit the spreadsheet, open
/// it again.
#[test]
fn the_workbook_inside_a_delivery_opens_on_its_own() {
    let plan = sound();
    let bytes = delivery_of(&plan);

    let mut archive =
        zip::ZipArchive::new(std::io::Cursor::new(&bytes)).unwrap();
    let mut workbook = Vec::new();
    {
        use std::io::Read;
        let mut member = archive
            .by_name(super::super::archive::WORKBOOK_MEMBER)
            .expect("a delivery carries the spreadsheet");
        member.read_to_end(&mut workbook).unwrap();
    }

    let opened = open(&workbook).expect("and it opens on its own");
    assert_eq!(opened.source, Source::Workbook);
    assert_eq!(opened.plan.external_id, plan.external_id);
}

#[test]
fn an_empty_zip_says_what_it_contained_rather_than_guessing() {
    // The end-of-central-directory record and nothing else — the signature that
    // used to reach `JSON.parse` and produce "Unexpected token 'P'".
    let empty: Vec<u8> = vec![
        0x50, 0x4b, 0x05, 0x06, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0,
    ];

    let refused =
        open(&empty).expect_err("an empty zip is not a configuration");
    // Every kind it knows how to open, so somebody holding the wrong file learns
    // what the right ones are. The list grew when election event exports were
    // added, which is why this asserts the whole clause rather than a prefix.
    assert!(refused.problems.iter().any(|problem| problem
        .message
        .contains("neither a delivery, a workbook nor an election event")));

    // And it says *empty* rather than trailing off after a colon with nothing
    // listed, which is what naming the members produces when there are none.
    assert!(
        refused
            .problems
            .iter()
            .any(|problem| problem.message.ends_with("it is empty")),
        "an empty archive should say so: {:?}",
        refused.problems
    );
}

#[test]
fn text_that_is_not_a_plan_is_refused_readably() {
    let refused =
        open(b"hello, world").expect_err("that is not a configuration");
    assert!(refused
        .problems
        .iter()
        .any(|problem| problem.message.contains("not a plan file")));
}

#[test]
fn bytes_that_are_nothing_at_all_are_refused() {
    let refused = open(&[0xff, 0xfe, 0x00, 0x01])
        .expect_err("not a zip and not text either");
    assert!(refused.has_errors());
}

/// A broken workbook comes back as a report, not as an exception.
///
/// The rule the whole module follows: a report renders on a screen and an
/// exception does not.
#[test]
fn a_workbook_full_of_problems_comes_back_as_a_report() {
    let broken = crate::election_config::sheet::Workbook::new(vec![
        crate::election_config::sheet::Sheet::from_grid(
            "ElectionEvent",
            &[
                vec![crate::election_config::paths::Cell::text("external_id")],
                vec![crate::election_config::paths::Cell::text("union-2027")],
            ],
        )
        .unwrap(),
        crate::election_config::sheet::Sheet::from_grid(
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
        .unwrap(),
    ])
    .unwrap();

    let bytes = write_xlsx(&broken).unwrap();
    let refused = open(&bytes).expect_err("a contest with no election");

    let problem = refused
        .problems
        .iter()
        .find(|problem| problem.code == Code::DanglingReference)
        .expect("it says which reference dangles");
    let at = problem.at.as_ref().expect("and points at the cell");
    assert_eq!(at.sheet, "Contests");
}

/// Warnings load; they do not stop anything.
#[test]
fn a_workbook_with_only_warnings_opens_and_says_so() {
    let mut plan = sound();
    plan.logo = Some(crate::election_config::architect::CandidateImage {
        file_name: "logo.png".to_string(),
        bytes: vec![1, 2, 3],
    });
    let bytes = write_xlsx(&workbook_of(&plan).unwrap()).unwrap();

    let opened = open(&bytes).expect("a lost image does not stop a load");
    assert!(!opened.report.has_errors());
    assert!(
        opened
            .report
            .warnings()
            .any(|problem| problem.message.contains("logo.png")),
        "and it names what could not travel"
    );
}

/// The importable zip inside a delivery, which is what the Admin Portal exports.
///
/// Taken out of a real delivery rather than assembled by hand: the point of these
/// tests is that the *actual* export is recognised, and a zip built here to look
/// like one would only prove that the detection agrees with this author.
fn election_event_archive(plan: &Blueprint) -> Vec<u8> {
    let delivery = delivery_of(plan);
    let mut reader =
        zip::ZipArchive::new(std::io::Cursor::new(delivery)).expect("a zip");
    let mut inner = reader
        .by_name(super::super::archive::IMPORTABLE_MEMBER)
        .expect("the importable zip inside the delivery");
    let mut bytes = Vec::new();
    std::io::Read::read_to_end(&mut inner, &mut bytes).expect("read");
    bytes
}

/// The bare `export_election_event-<id>.json`, on its own.
fn election_event_json(plan: &Blueprint) -> Vec<u8> {
    let archive = election_event_archive(plan);
    let mut reader =
        zip::ZipArchive::new(std::io::Cursor::new(archive)).expect("a zip");
    let name = reader
        .file_names()
        .find(|name| {
            name.contains("export_election_event") && name.ends_with(".json")
        })
        .expect("the export document")
        .to_string();
    let mut entry = reader.by_name(&name).expect("by name");
    let mut bytes = Vec::new();
    std::io::Read::read_to_end(&mut entry, &mut bytes).expect("read");
    bytes
}

#[test]
fn an_election_event_export_opens_as_a_plan() {
    let plan = sound();
    let opened = open(&election_event_json(&plan)).expect("an export opens");

    assert_eq!(opened.source, Source::ElectionEvent);
    assert_eq!(opened.plan.external_id, plan.external_id);
    assert_eq!(opened.plan.elections.len(), 1);
    assert_eq!(opened.plan.elections[0].contests[0].candidates.len(), 2);
    // Warnings, because an export is not a whole plan — but not errors, because
    // it is a perfectly good election event.
    assert!(!opened.report.has_errors(), "{}", opened.report);
    assert!(!opened.report.is_empty(), "it says what it could not carry");
}

#[test]
fn an_election_event_archive_opens_as_a_plan() {
    let plan = sound();
    let opened =
        open(&election_event_archive(&plan)).expect("an export zip opens");

    assert_eq!(opened.source, Source::ElectionEventArchive);
    assert_eq!(opened.plan.external_id, plan.external_id);
    assert_eq!(opened.plan.elections.len(), 1);
}

#[test]
fn an_election_event_is_not_mistaken_for_a_plan() {
    // **The ordering test.** `Blueprint` requires only `version` and
    // `external_id`, so a document with neither fails — but the failure would be
    // an opaque serde message about a plan, when what somebody handed over was a
    // perfectly good export. The fingerprint has to be checked first.
    let json = election_event_json(&sound());
    let opened = open(&json).expect("opens");
    assert_ne!(
        opened.source,
        Source::Plan,
        "an export must not be read as a plan"
    );
}

#[test]
fn a_plan_is_not_mistaken_for_an_election_event() {
    // And the other direction, which is the one a loose fingerprint would break.
    let plan = sound();
    let bytes = serde_json::to_vec(&plan).unwrap();
    let opened = open(&bytes).expect("opens");
    assert_eq!(opened.source, Source::Plan);
}

#[test]
fn an_encrypted_export_says_that_it_is_encrypted() {
    // An `.ezip` is AES-CBC from its first byte, so it has no `PK` magic and no
    // valid UTF-8 — it would otherwise come back as "neither a .zip, an .xlsx,
    // nor text", which sends somebody looking for a corrupted download instead
    // of asking for the unencrypted export.
    //
    // Recognised by the name, because that is the only thing about an encrypted
    // file that is not encrypted.
    let refused = open_named(
        &[0x53, 0x61, 0x6c, 0x74, 0x65, 0x64, 0x5f, 0x5f, 9, 9],
        Some("election-event-1-export-sha256-abc.ezip"),
    )
    .expect_err("an encrypted export is refused");
    assert!(
        says(&refused, "encrypted"),
        "it should name the encryption: {refused}"
    );
}

#[test]
fn a_zip_that_is_nothing_recognisable_still_lists_what_it_held() {
    let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    zip.start_file::<_, ()>(
        "notes.txt",
        zip::write::SimpleFileOptions::default(),
    )
    .unwrap();
    std::io::Write::write_all(&mut zip, b"hello").unwrap();
    let bytes = zip.finish().unwrap().into_inner();

    let refused = open(&bytes).expect_err("not a configuration");
    assert!(says(&refused, "notes.txt"), "{refused}");
}

/// A plan with a census and a candidate's photograph, so the archive has both.
fn peopled() -> Blueprint {
    let mut plan = sound();
    // Declared rather than left to the builder: validation runs before the
    // synthesised area exists, so a voter naming one the plan does not declare is
    // refused before there is anything to match.
    plan.areas = vec![serde_json::from_value(serde_json::json!({
        "external_id": "all-voters",
        "name": "All voters"
    }))
    .unwrap()];
    plan.voters = vec![
        serde_json::from_value(serde_json::json!({
            "username": "ada",
            "email": "ada@example.org",
            "first_name": "Ada",
            "last_name": "Lovelace",
            "area_external_id": "all-voters",
            "department": "Engineering"
        }))
        .unwrap(),
        serde_json::from_value(serde_json::json!({
            "username": "grace",
            "email": "grace@example.org",
            "first_name": "Grace",
            "last_name": "Hopper",
            "area_external_id": "all-voters"
        }))
        .unwrap(),
    ];
    if let Some(candidate) = plan
        .elections
        .first_mut()
        .and_then(|election| election.contests.first_mut())
        .and_then(|contest| contest.candidates.first_mut())
    {
        candidate.image =
            Some(crate::election_config::architect::CandidateImage {
                file_name: "alice.png".to_string(),
                bytes: vec![0x89, b'P', b'N', b'G', 1, 2, 3],
            });
    }
    plan
}

#[test]
fn an_election_event_archive_brings_the_census_with_it() {
    // "Import all the data it contains" — the census is in the archive and not in
    // the document, so this is the assertion that separates the two doors.
    let plan = peopled();
    let opened =
        open(&election_event_archive(&plan)).expect("an export zip opens");

    assert_eq!(opened.source, Source::ElectionEventArchive);
    assert_eq!(opened.plan.voters.len(), 2, "{}", opened.report);
    let ada = &opened.plan.voters[0];
    assert_eq!(ada.username, "ada");
    assert_eq!(ada.email, "ada@example.org");
    assert_eq!(ada.first_name, "Ada");
    // A column the wizard has no field for rides in `extra`, which is how a
    // client keeps a reporting breakout without a code change.
    assert_eq!(
        ada.extra.get("department").map(String::as_str),
        Some("Engineering")
    );
}

#[test]
fn an_election_event_archive_brings_the_photographs_with_it() {
    let plan = peopled();
    let opened = open(&election_event_archive(&plan)).expect("opens");

    let alice = &opened.plan.elections[0].contests[0].candidates[0];
    let image = alice.image.as_ref().expect("Alice's photograph came back");
    assert_eq!(image.file_name, "alice.png");
    assert_eq!(image.bytes, vec![0x89, b'P', b'N', b'G', 1, 2, 3]);
    // And nothing complains about a photograph it did find.
    assert!(
        !says(&opened.report, "does not contain it"),
        "{}",
        opened.report
    );
}

#[test]
fn the_bare_document_says_the_census_is_not_in_it() {
    // The bare JSON cannot carry a census, and the difference between the two
    // doors is only useful if the wizard can say which one you used.
    let plan = peopled();
    let opened = open(&election_event_json(&plan)).expect("opens");

    assert_eq!(opened.source, Source::ElectionEvent);
    assert!(
        opened.plan.voters.is_empty(),
        "a bare document has no census in it"
    );
}

/// The same archive, with its entries renamed the way the platform names them.
///
/// **The case a delivery built here cannot produce.** The platform's own exporter
/// prefixes each `images/` and `export_S3_files/` entry with twelve characters of
/// tempfile name — `images/enGgihs9azd5document_<uuid>_alice.png` — and
/// `extract_document_uuid` on the import side matches unanchored because of it. A
/// test that only ever sees our own naming would pass against an anchored match
/// and drop every photograph from a real export.
fn with_tempfile_prefixes(archive: Vec<u8>) -> Vec<u8> {
    let mut reader =
        zip::ZipArchive::new(std::io::Cursor::new(archive)).expect("a zip");
    let mut out = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));

    for index in 0..reader.len() {
        let mut entry = reader.by_index(index).expect("an entry");
        let name = entry.name().to_string();
        let mut bytes = Vec::new();
        std::io::Read::read_to_end(&mut entry, &mut bytes).expect("read");

        let renamed = match name.split_once('/') {
            Some((folder, file))
                if folder == "images" || folder == "export_S3_files" =>
            {
                format!("{folder}/enGgihs9azd5{file}")
            }
            _ => name,
        };
        out.start_file::<_, ()>(
            renamed,
            zip::write::SimpleFileOptions::default(),
        )
        .expect("start");
        std::io::Write::write_all(&mut out, &bytes).expect("write");
    }

    out.finish().expect("finish").into_inner()
}

#[test]
fn a_photograph_is_found_through_the_platforms_own_tempfile_prefix() {
    let plan = peopled();
    let archive = with_tempfile_prefixes(election_event_archive(&plan));
    let opened = open(&archive).expect("a real-shaped export opens");

    let alice = &opened.plan.elections[0].contests[0].candidates[0];
    let image = alice.image.as_ref().expect("Alice's photograph came back");
    // And the *file's* name, not the entry's: a plan rebuilt from this must
    // produce `images/document_<id>_alice.png` again rather than baking somebody
    // else's tempfile prefix into the next export.
    assert_eq!(image.file_name, "alice.png");
    assert_eq!(image.bytes, vec![0x89, b'P', b'N', b'G', 1, 2, 3]);
}
