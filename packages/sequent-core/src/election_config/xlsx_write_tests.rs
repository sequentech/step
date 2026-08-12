// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The writer, tested against the reader.
//!
//! No fixture files: a workbook is built in memory, written, read back and
//! compared. That is the strongest available proof for this pair — the two are
//! inverses or they are not — and it means there is no binary in the repository to
//! keep in step, and no chance of a client's data ending up in one.

use serde_json::{json, Value};

use super::*;
use crate::election_config::sheet::Sheet;
use crate::election_config::xlsx::read_xlsx;

/// A sheet from a grid of text, the way an author's file arrives.
fn sheet(name: &str, grid: &[&[&str]]) -> Sheet {
    let cells: Vec<Vec<crate::election_config::paths::Cell>> = grid
        .iter()
        .map(|row| {
            row.iter()
                .map(|text| crate::election_config::paths::Cell::text(*text))
                .collect()
        })
        .collect();
    Sheet::from_grid(name, &cells).unwrap()
}

fn book(sheets: Vec<Sheet>) -> Workbook {
    Workbook::new(sheets).unwrap()
}

/// One part of the written file, as text.
///
/// The styling tests read the XML directly because there is nothing else to read
/// it with: the reader in `xlsx.rs` throws styles away, quite correctly — a width
/// is not data. So a column that lost its colour or its frozen header would round
/// trip perfectly and look wrong, and this is what notices.
fn part(bytes: &[u8], name: &str) -> String {
    let mut archive =
        zip::ZipArchive::new(std::io::Cursor::new(bytes)).expect("it is a zip");
    let mut text = String::new();
    std::io::Read::read_to_string(
        &mut archive
            .by_name(name)
            .unwrap_or_else(|_| panic!("no {name}")),
        &mut text,
    )
    .expect("it is text");
    text
}

/// Write it, read it, and it is the same workbook.
fn round_trip(workbook: &Workbook) -> Workbook {
    let bytes = write_xlsx(workbook).expect("this workbook writes");
    read_xlsx(&bytes).expect("what we wrote reads")
}

#[test]
fn a_workbook_survives_a_round_trip_through_a_file() {
    // Every shape a cell can hold, because the round trip is only worth
    // something if the awkward ones are in it.
    let workbook = book(vec![sheet(
        "Contests",
        &[
            &["external_id", "max_votes", "is_encrypted", "description"],
            &["president", "3", "true", "One seat, three years."],
            &["board", "-12.000", "false", "null"],
        ],
    )]);

    assert_eq!(round_trip(&workbook), workbook);
}

#[test]
fn a_json_array_in_one_cell_comes_back_as_an_array() {
    // How a whole `voting_channels` or a language list fits in a spreadsheet.
    let workbook = book(vec![sheet(
        "ElectionEvent",
        &[
            &[
                "external_id",
                "presentation.language_conf.enabled_language_codes",
            ],
            &["union-2027", r#"["en","es"]"#],
        ],
    )]);

    let back = round_trip(&workbook);
    assert_eq!(back, workbook);
    assert_eq!(
        back.rows("electionevent")[0]
            .get("presentation.language_conf.enabled_language_codes"),
        Some(&json!(["en", "es"]))
    );
}

#[test]
fn a_multi_valued_column_keeps_its_separator() {
    // `authorized-election-ids` is multi-valued *by column*, so the same array
    // that is bracketed JSON anywhere else is `a || b` here. Getting this the
    // wrong way round writes a file the reader turns into one long string.
    let workbook = book(vec![sheet(
        "Voters",
        &[
            &["username", "authorized-election-ids"],
            &["ada", "board || president"],
        ],
    )]);

    let back = round_trip(&workbook);
    assert_eq!(
        back.rows("voters")[0].get("authorized-election-ids"),
        Some(&json!(["board", "president"]))
    );
    assert_eq!(back, workbook);
}

#[test]
fn a_column_past_z_gets_its_real_name() {
    // The reader's own test helper wrote `(b'A' + index)`, which produces `[` for
    // the 27th column and a file nothing opens. A Voters sheet carrying a
    // client's own columns passes 27 routinely, so this is not a curiosity.
    assert_eq!(column_name(0), "A");
    assert_eq!(column_name(25), "Z");
    assert_eq!(column_name(26), "AA");
    assert_eq!(column_name(27), "AB");
    assert_eq!(column_name(51), "AZ");
    assert_eq!(column_name(52), "BA");
    assert_eq!(column_name(701), "ZZ");
    assert_eq!(column_name(702), "AAA");

    let headers: Vec<String> =
        (0..30).map(|at| format!("column_{at}")).collect();
    let values: Vec<String> = (0..30).map(|at| format!("value {at}")).collect();
    let header_refs: Vec<&str> = headers.iter().map(String::as_str).collect();
    let value_refs: Vec<&str> = values.iter().map(String::as_str).collect();

    let workbook =
        book(vec![sheet("Voters", &[&header_refs[..], &value_refs[..]])]);
    let back = round_trip(&workbook);

    assert_eq!(
        back.rows("voters")[0].get("column_29"),
        Some(&json!("value 29"))
    );
    assert_eq!(back, workbook);
}

#[test]
fn text_that_is_xml_is_escaped() {
    // `Smith & Jones` is an ordinary name and produced a file nothing could open.
    // The apostrophe is in here too: it is legal unescaped in text, and escaping
    // it consistently is cheaper than remembering which contexts differ.
    let workbook = book(vec![sheet(
        "Candidates",
        &[
            &["external_id", "name"],
            &["smith-jones", r#"Smith & Jones <"the firm"> O'Brien"#],
        ],
    )]);

    let back = round_trip(&workbook);
    assert_eq!(
        back.rows("candidates")[0].get("name"),
        Some(&json!(r#"Smith & Jones <"the firm"> O'Brien"#))
    );
}

#[test]
fn a_control_character_does_not_produce_an_unopenable_file() {
    // A spreadsheet somebody pasted into carries these, and they are not legal in
    // XML at any escape. A space is a lie the file can survive; a vertical tab in
    // the middle of a name is a file that will not open at all.
    let workbook = book(vec![sheet(
        "Candidates",
        &[&["external_id", "name"], &["ada", "Ada\u{000b}Lovelace"]],
    )]);

    let bytes = write_xlsx(&workbook).expect("it writes");
    let back = read_xlsx(&bytes).expect("and it reads");
    assert_eq!(
        back.rows("candidates")[0].get("name"),
        Some(&json!("Ada Lovelace"))
    );
}

#[test]
fn a_sheet_name_excel_refuses_is_refused_here() {
    for bad in ["Voters:2027", "Voters/2027", "Voters[2027]", "Voters?"] {
        let workbook = book(vec![sheet(bad, &[&["a"], &["1"]])]);
        let refused = write_xlsx(&workbook)
            .expect_err(&format!("'{bad}' is not a tab name"));
        assert_eq!(refused.code, Code::InvalidValue);
        assert!(refused.at.is_some(), "it names the sheet");
    }
}

#[test]
fn a_sheet_name_too_long_for_excel_is_refused_rather_than_truncated() {
    // Truncating is worse than refusing: two sheets differing past the 31st
    // character would become one tab, and `Workbook` guarantees they are
    // distinct. Silently merging them loses rows.
    let long = "V".repeat(32);
    let workbook = book(vec![sheet(&long, &[&["a"], &["1"]])]);
    let refused = write_xlsx(&workbook).expect_err("32 characters is too many");
    assert!(refused.message.contains("31"));
}

#[test]
fn the_bytes_do_not_change_between_runs() {
    // A delivery is a file somebody diffs against last week's. A clock anywhere
    // in here would make two builds of one plan different files, which is the
    // same argument `archive::zip` already makes.
    let workbook = book(vec![sheet(
        "ElectionEvent",
        &[&["external_id"], &["union-2027"]],
    )]);

    assert_eq!(
        write_xlsx(&workbook).unwrap(),
        write_xlsx(&workbook).unwrap()
    );
}

#[test]
fn a_blank_cell_stays_blank_rather_than_becoming_null() {
    // The format's own distinction: a blank cell means "the author said nothing,
    // keep the template default", and the literal `null` means `Value::Null`.
    // Writing the first as the second would change what a build produces.
    let workbook = book(vec![sheet(
        "Contests",
        &[
            &["external_id", "description", "max_votes"],
            &["president", "", "1"],
        ],
    )]);

    let back = round_trip(&workbook);
    assert!(back.rows("contests")[0].get("description").is_none());
    assert_eq!(back, workbook);
}

#[test]
fn a_header_with_no_name_keeps_the_columns_after_it_in_place() {
    // The reader keeps a blank header as a placeholder so column positions still
    // line up. If the writer skipped the position rather than the value, every
    // column after it would shift by one.
    let cells: Vec<Vec<crate::election_config::paths::Cell>> = vec![
        vec![
            crate::election_config::paths::Cell::text("external_id"),
            crate::election_config::paths::Cell::Blank,
            crate::election_config::paths::Cell::text("max_votes"),
        ],
        vec![
            crate::election_config::paths::Cell::text("president"),
            crate::election_config::paths::Cell::text("ignored"),
            crate::election_config::paths::Cell::text("2"),
        ],
    ];
    let workbook = book(vec![Sheet::from_grid("Contests", &cells).unwrap()]);

    let back = round_trip(&workbook);
    // A string, not a number: the author typed text, and `coerce_scalar` only
    // numifies a trailing-zero float. That it stays a string is the round trip
    // working, not failing.
    assert_eq!(back.rows("contests")[0].get("max_votes"), Some(&json!("2")));
    assert_eq!(back, workbook);
}

/// The four the format cannot carry, stated so nobody discovers them later.
///
/// A `Value::String` spelling `true`, `false` or `null`, one shaped like `-12.000`,
/// and one that is bracketed JSON all read back as the thing they spell — that is
/// what those characters mean in a spreadsheet cell. There is no escape the reader
/// would understand, and inventing one would mean a file Excel shows differently
/// from what the platform reads. This test exists so the loss is on the record.
///
/// A plain `"3"` is **not** in that set, which is worth knowing: `coerce_scalar`
/// numifies only a trailing-zero float, so a digit string survives.
#[test]
fn the_four_strings_the_format_cannot_carry() {
    for (spelled, becomes) in [
        ("true", json!(true)),
        ("false", json!(false)),
        ("null", Value::Null),
        ("-12.000", json!(-12)),
        (r#"["a"]"#, json!(["a"])),
    ] {
        let written = crate::election_config::paths::cell_text(
            &Value::String(spelled.to_string()),
            false,
        );
        assert_eq!(written, spelled, "it writes the characters it was given");
        assert_eq!(
            crate::election_config::paths::coerce_scalar(&written),
            becomes,
            "'{spelled}' cannot come back as a string"
        );
    }

    // And the one that does survive, which is the common case.
    assert_eq!(
        crate::election_config::paths::coerce_scalar("3"),
        json!("3"),
        "a digit string is still a string"
    );
}

/// Every kind, through the real file rather than through the coercion alone.
///
/// Numbers and booleans are written as *typed cells*, so checking them against
/// `coerce_scalar` would be checking a path the writer does not take.
#[test]
fn every_value_kind_survives_the_file() {
    let cases: Vec<(&str, Value)> = vec![
        ("null", Value::Null),
        ("yes", json!(true)),
        ("no", json!(false)),
        ("count", json!(7)),
        ("fraction", json!(1.5)),
        ("plain", json!("plain")),
        ("object", json!({"a": 1})),
        ("array", json!(["x", "y"])),
    ];

    let headers: Vec<&str> = cases.iter().map(|(name, _)| *name).collect();
    let mut cells = vec![headers
        .iter()
        .map(|name| crate::election_config::paths::Cell::text(*name))
        .collect::<Vec<_>>()];
    cells.push(
        cases
            .iter()
            .map(|(_, value)| match value {
                Value::Bool(flag) => {
                    crate::election_config::paths::Cell::Bool(*flag)
                }
                Value::Number(number) if number.is_i64() => {
                    crate::election_config::paths::Cell::Int(
                        number.as_i64().unwrap(),
                    )
                }
                Value::Number(number) => {
                    crate::election_config::paths::Cell::Float(
                        number.as_f64().unwrap(),
                    )
                }
                other => crate::election_config::paths::Cell::text(
                    crate::election_config::paths::cell_text(other, false),
                ),
            })
            .collect::<Vec<_>>(),
    );

    let workbook = book(vec![Sheet::from_grid("Contests", &cells).unwrap()]);
    let back = round_trip(&workbook);

    for (name, expected) in &cases {
        assert_eq!(
            back.rows("contests")[0].get(name),
            Some(expected),
            "{name} did not survive"
        );
    }
    assert_eq!(back, workbook);
}

/// A column of prose is capped and wrapped; one of identifiers is neither.
#[test]
fn a_long_description_wraps_and_an_id_stays_narrow() {
    let headers = vec!["external_id".to_string(), "description".to_string()];
    let rows = vec![vec![
        "okonjo".to_string(),
        "Alice has served on the executive committee since 2019 and chairs the \
         grievance panel. She is standing on a platform of shorter shifts."
            .to_string(),
    ]];
    let shape = shapes(&headers, &rows);

    assert_eq!(
        shape[0],
        Shape {
            width: 13.0,
            wrap: false
        }
    );
    assert_eq!(
        shape[1],
        Shape {
            width: WIDEST_PROSE,
            wrap: true
        }
    );
}

/// Markup is never wrapped, however long it is.
///
/// The request was explicit about this, and the reason is that a template is read
/// by scanning it: `<html>` broken across six lines of a 52-wide column is harder
/// to read than one line running off the edge, which a person can widen or click
/// into. Every form of it — a tag, a handlebars expression, JSON, a URL, a long
/// unbroken token — because the Templates sheet carries all five.
#[test]
fn nothing_that_is_markup_is_ever_wrapped() {
    for value in [
        "<html><body><h1>Vote</h1></body></html>",
        "Dear {{voter.name}}, voting closes on {{voting_closes}}.",
        r#"{"policy": "plurality-at-large", "seats": 3}"#,
        "https://example.org/elections/union-2027/ballot?lang=en&area=north",
        "d5f3a1b9c7e24a6f8b0d2e4a6c8f0b2d4e6a8c0f2b4d6e8a0c2e4f6a8b0d2e4f",
    ] {
        let headers = vec!["body".to_string()];
        let shape = shapes(&headers, &vec![vec![value.to_string()]]);
        assert!(!shape[0].wrap, "wrapped markup: {value}");
        assert!(shape[0].width <= WIDEST_FIXED);
    }
}

/// Prose long enough to wrap, next to markup, in the same column: markup wins.
///
/// One templated row among a hundred plain ones is the realistic case — a Messages
/// sheet whose first message is plain text — and it is the one where wrapping would
/// mangle the row that matters most.
#[test]
fn one_templated_row_stops_the_whole_column_wrapping() {
    let headers = vec!["body".to_string()];
    let rows = vec![
        vec!["A plain sentence, long enough on its own to pass the cap and then some more words after it to be sure.".to_string()],
        vec!["<p>{{voter.name}}</p>".to_string()],
    ];
    assert!(!shapes(&headers, &rows)[0].wrap);
}

/// A long header does not widen a column full of two-letter values.
///
/// `presentation.language_conf.enabled_language_codes` over a column of `en` made
/// that column 51 characters wide, which is what the sample plan looked like before
/// this: half the sheet was header. It wraps onto a second line instead.
#[test]
fn a_field_path_header_wraps_rather_than_widening_its_column() {
    let headers =
        vec!["presentation.language_conf.enabled_language_codes".to_string()];
    let shape = shapes(&headers, &vec![vec!["en".to_string()]]);

    assert_eq!(shape[0].width, HEADER_WIDE + 2.0);
    assert_eq!(header_lines(&headers, &shape), 2);
}

/// The header row is tall enough for the tallest header on the sheet.
#[test]
fn the_header_row_is_as_tall_as_its_deepest_header() {
    let one = vec!["name".to_string(), "email".to_string()];
    let shape =
        shapes(&one, &vec![vec!["Alice".to_string(), "a@b.c".to_string()]]);
    assert_eq!(header_lines(&one, &shape), 1);

    // One deep header makes the whole row two lines, not just its own cell.
    let two = vec![
        "name".to_string(),
        "tally_configuration.tie_breaking_policy".to_string(),
    ];
    let shape =
        shapes(&two, &vec![vec!["Alice".to_string(), "random".to_string()]]);
    assert_eq!(header_lines(&two, &shape), 2);
}

/// An empty column is still wide enough to click on.
#[test]
fn a_column_with_nothing_in_it_keeps_a_usable_width() {
    let headers = vec!["id".to_string()];
    assert_eq!(shapes(&headers, &[])[0].width, NARROWEST);
}

/// The header cells carry the header format, and ordinary cells carry nothing.
///
/// The second half is the one worth a test: a census is a hundred thousand rows,
/// and `s="0"` on every cell of it is bytes that say exactly nothing. Asserted on
/// the XML rather than through the reader, because the reader is indifferent to
/// styles — which is the whole reason a regression here would otherwise be silent.
#[test]
fn only_the_cells_that_need_a_format_carry_one() {
    let workbook = book(vec![sheet(
        "Voters",
        &[&["username", "email"], &["alice", "alice@example.org"]],
    )]);
    let sheet_xml =
        part(&write_xlsx(&workbook).unwrap(), "xl/worksheets/sheet1.xml");

    assert!(sheet_xml
        .contains(&format!(r#"<c r="A1" s="{HEADER_STYLE}" t="inlineStr">"#)));
    assert!(sheet_xml.contains(r#"<c r="A2" t="inlineStr">"#));
    assert!(!sheet_xml.contains(r#"s="0""#));
}

/// The navy is written the way the format reads colour, and the header row freezes.
///
/// `FF` in front of the six digits is not decoration: a `rgb` of `0F054C` is four
/// bytes short, and LibreOffice reads the missing alpha as transparent — a header
/// whose white bold text lands on white. It renders correctly in Excel, so this is
/// the kind of thing that ships.
#[test]
fn the_header_is_navy_and_the_top_row_stays_put() {
    let bytes =
        write_xlsx(&book(vec![sheet("Voters", &[&["username"], &["alice"]])]))
            .unwrap();

    assert!(
        part(&bytes, "xl/styles.xml").contains(r#"<fgColor rgb="FF0F054C"/>"#)
    );
    assert!(
        part(&bytes, "xl/worksheets/sheet1.xml").contains(r#"state="frozen""#)
    );
}

/// Write a workbook with prose and markup in it, for a person to open.
///
/// Ignored, and not a duplicate of `architect_tests::emit_a_workbook_to_look_at`:
/// that one writes the *sample plan*, whose every value is an id or an enum, so it
/// shows the header band and nothing else. Wrapping — the half of `EA-F5-006` that
/// needed the most judgement — only happens past fifty-two characters of prose, and
/// nothing in the sample plan is that long. This carries all three shapes: short
/// identifiers, a description that must wrap, and a template that must not.
///
/// `cargo test -p sequent-core --features election_config_archive,election_config_xlsx
///  dump_a_workbook -- --ignored --nocapture`
#[test]
#[ignore]
fn dump_a_workbook() {
    let candidates = sheet(
        "Candidates",
        &[
            &["external_id", "name", "description", "winners"],
            &[
                "okonjo",
                "Alice Okonjo",
                "Alice has served on the executive committee since 2019 and chairs \
                 the grievance panel. She is standing on a platform of shorter \
                 shifts and a rebuilt apprenticeship programme.",
                "1",
            ],
            &[
                "iyer",
                "Bob Iyer",
                "Bob is a shop steward at the Northgate depot.",
                "1",
            ],
        ],
    );

    let templates = sheet(
        "Messages",
        &[
            &["name", "body"],
            &[
                "invitation",
                "<html><body><h1>{{election.name}}</h1><p>Voting is open until \
                 {{voting_closes}}. <a href=\"{{link}}\">Cast your ballot</a>.\
                 </p></body></html>",
            ],
        ],
    );

    let bytes =
        write_xlsx(&book(vec![candidates, templates])).expect("it writes");
    let at = std::env::temp_dir().join("sample-workbook.xlsx");
    std::fs::write(&at, &bytes).expect("written");
    println!("wrote {} bytes to {}", bytes.len(), at.display());
}
