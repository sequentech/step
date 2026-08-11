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
