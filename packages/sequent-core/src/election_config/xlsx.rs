// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Reading an `.xlsx` file into a [`Workbook`].
//!
//! The only part of this module that knows a file format. It converts `calamine`
//! cells into [`Cell`]s and hands the grid to [`Sheet::from_grid`]; every
//! decision about headers, blank rows and coercion is there, where it can be
//! tested without a fixture file.
//!
//! Behind the `election_config_xlsx` feature so the front ends that have no
//! workbook to read do not carry a spreadsheet library into their WASM bundle.
//! Reads from bytes rather than a path, so the same call serves `step-cli` and a
//! browser holding the result of a file input.

use crate::election_config::paths::Cell;
use crate::election_config::problem::{Code, Problem};
use crate::election_config::sheet::{Origin, Sheet, Workbook};
use calamine::{Data, Reader, Xlsx};
use std::io::Cursor;

/// Read an `.xlsx` from bytes.
///
/// Formulas are read as their cached result, which is what a spreadsheet stores
/// alongside them. A file written by a tool that does not compute formulas has
/// no cached result, so the cell reads as blank and turns up as a missing
/// required value — better than a formula string reaching the output.
pub fn read_xlsx(bytes: &[u8]) -> Result<Workbook, Problem> {
    let mut book: Xlsx<_> = Xlsx::new(Cursor::new(bytes)).map_err(|error| {
        Problem::error(
            Code::InvalidValue,
            "workbook",
            format!("this does not read as an .xlsx file: {error}"),
        )
    })?;

    let names = book.sheet_names().to_vec();
    let mut sheets = Vec::with_capacity(names.len());

    for name in names {
        let range = book.worksheet_range(&name).map_err(|error| {
            Problem::error(
                Code::InvalidValue,
                format!("sheet '{name}'"),
                format!("could not be read: {error}"),
            )
            .at(&Origin::sheet(&name))
        })?;

        let grid: Vec<Vec<Cell>> = range
            .rows()
            .map(|row| row.iter().map(cell_from_data).collect())
            .collect();

        sheets.push(Sheet::from_grid(name, &grid)?);
    }

    Workbook::new(sheets)
}

/// Narrow one spreadsheet cell to the neutral vocabulary.
fn cell_from_data(data: &Data) -> Cell {
    match data {
        Data::Empty => Cell::Blank,
        Data::String(text) => Cell::text(text.as_str()),
        Data::Float(value) => Cell::Float(*value),
        Data::Int(value) => Cell::Int(*value),
        Data::Bool(value) => Cell::Bool(*value),

        // A duration is asked about before being converted, because
        // `as_datetime` will cheerfully turn one into an instant near the 1900
        // epoch — 0.5 becomes 1899-12-31T12:00 — which is a plausible-looking
        // timestamp and completely wrong. No schema field wants a duration, so
        // the number is handed over for validation to object to instead.
        Data::DateTime(excel) if excel.is_duration() => {
            Cell::Float(excel.as_f64())
        }
        Data::DateTime(excel) => match excel.as_datetime() {
            Some(naive) => Cell::DateTime(naive),
            // A serial number outside the calendar. Same reasoning: visible
            // rather than dropped.
            None => Cell::Float(excel.as_f64()),
        },

        // Already text in the file. Left as text on purpose: these are written
        // in ISO 8601, which is what the platform wants anyway, and reparsing
        // them only adds a way to get the timezone wrong.
        Data::DateTimeIso(text) => Cell::text(text.as_str()),
        Data::DurationIso(text) => Cell::text(text.as_str()),

        // A formula that evaluated to an error — #REF!, #DIV/0!. Blank would
        // hide it; the text makes it show up as an invalid value naming the
        // cell.
        Data::Error(error) => Cell::text(format!("{error}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::election_config::sheet::{SHEET_ELECTIONS, SHEET_REPORTS};
    use calamine::{CellErrorType, ExcelDateTime, ExcelDateTimeType};
    use serde_json::{json, Value};

    #[test]
    fn an_empty_cell_is_blank() {
        assert_eq!(cell_from_data(&Data::Empty), Cell::Blank);
    }

    #[test]
    fn a_cell_holding_only_spaces_is_blank_too() {
        // Someone typed a space; the intent is still nothing.
        assert_eq!(cell_from_data(&Data::String("   ".into())), Cell::Blank);
    }

    #[test]
    fn the_native_types_pass_straight_through() {
        assert_eq!(cell_from_data(&Data::Int(7)), Cell::Int(7));
        assert_eq!(cell_from_data(&Data::Float(1.5)), Cell::Float(1.5));
        assert_eq!(cell_from_data(&Data::Bool(true)), Cell::Bool(true));
        assert_eq!(
            cell_from_data(&Data::String("board".into())),
            Cell::Text("board".to_string())
        );
    }

    #[test]
    fn a_date_cell_becomes_an_instant_with_its_time_intact() {
        // The time matters as much as the date: it is what opens and closes a
        // voting window.
        let excel = ExcelDateTime::new(
            46318.677_083_333_336,
            ExcelDateTimeType::DateTime,
            false,
        );
        let Cell::DateTime(naive) = cell_from_data(&Data::DateTime(excel))
        else {
            panic!("expected a datetime");
        };
        assert_eq!(naive.to_string(), "2026-10-23 16:15:00");
    }

    #[test]
    fn a_duration_keeps_its_number_rather_than_becoming_a_bogus_1899_instant() {
        // `as_datetime` turns 0.5 into 1899-12-31T12:00 — plausible-looking and
        // entirely wrong — so the kind is asked about first.
        let excel =
            ExcelDateTime::new(0.5, ExcelDateTimeType::TimeDelta, false);
        assert_eq!(cell_from_data(&Data::DateTime(excel)), Cell::Float(0.5));
    }

    #[test]
    fn an_iso_timestamp_stays_the_text_it_already_is() {
        // ISO 8601 is what the platform wants; reparsing only risks the zone.
        assert_eq!(
            cell_from_data(&Data::DateTimeIso("2026-10-24T16:15:00Z".into())),
            Cell::Text("2026-10-24T16:15:00Z".to_string())
        );
    }

    #[test]
    fn a_formula_error_is_visible_rather_than_blank() {
        // Blank would hide a broken formula and produce a bundle missing a field
        // nobody meant to leave out.
        assert_eq!(
            cell_from_data(&Data::Error(CellErrorType::Ref)),
            Cell::Text("#REF!".to_string())
        );
    }

    #[test]
    fn bytes_that_are_not_a_spreadsheet_are_refused_with_a_readable_message() {
        let problem = read_xlsx(b"this is not a spreadsheet").unwrap_err();
        assert_eq!(problem.code, Code::InvalidValue);
        assert!(problem.message.contains("does not read as an .xlsx"));
    }

    /// The smallest real `.xlsx` this can be tested against: written here rather
    /// than committed, so there is no binary fixture to keep in step with the
    /// code, and no chance of a client's data ending up in the repository.
    fn tiny_xlsx(sheets: &[(&str, &[&[&str]])]) -> Vec<u8> {
        use std::io::Write;
        use zip::write::SimpleFileOptions;

        let mut buffer = Vec::new();
        {
            let mut zip = zip::ZipWriter::new(Cursor::new(&mut buffer));
            let options = SimpleFileOptions::default();

            zip.start_file("[Content_Types].xml", options).unwrap();
            let mut content_types = String::from(
                r#"<?xml version="1.0"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>"#,
            );
            for index in 1..=sheets.len() {
                content_types.push_str(&format!(
                    r#"<Override PartName="/xl/worksheets/sheet{index}.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>"#
                ));
            }
            content_types.push_str("</Types>");
            zip.write_all(content_types.as_bytes()).unwrap();

            zip.start_file("_rels/.rels", options).unwrap();
            zip.write_all(
                br#"<?xml version="1.0"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
            )
            .unwrap();

            zip.start_file("xl/workbook.xml", options).unwrap();
            let mut workbook = String::from(
                r#"<?xml version="1.0"?><workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets>"#,
            );
            for (index, (name, _)) in sheets.iter().enumerate() {
                let id = index + 1;
                workbook.push_str(&format!(
                    r#"<sheet name="{name}" sheetId="{id}" r:id="rId{id}"/>"#
                ));
            }
            workbook.push_str("</sheets></workbook>");
            zip.write_all(workbook.as_bytes()).unwrap();

            zip.start_file("xl/_rels/workbook.xml.rels", options)
                .unwrap();
            let mut rels = String::from(
                r#"<?xml version="1.0"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">"#,
            );
            for index in 1..=sheets.len() {
                rels.push_str(&format!(
                    r#"<Relationship Id="rId{index}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet{index}.xml"/>"#
                ));
            }
            rels.push_str("</Relationships>");
            zip.write_all(rels.as_bytes()).unwrap();

            for (index, (_, rows)) in sheets.iter().enumerate() {
                zip.start_file(
                    format!("xl/worksheets/sheet{}.xml", index + 1),
                    options,
                )
                .unwrap();
                let mut xml = String::from(
                    r#"<?xml version="1.0"?><worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData>"#,
                );
                for (row_index, row) in rows.iter().enumerate() {
                    xml.push_str(&format!(r#"<row r="{}">"#, row_index + 1));
                    for (column_index, value) in row.iter().enumerate() {
                        let reference = format!(
                            "{}{}",
                            (b'A' + column_index as u8) as char,
                            row_index + 1
                        );
                        if value.is_empty() {
                            continue;
                        }
                        // Numbers are written as numbers, the way a spreadsheet
                        // stores them, so the float-to-integer coercion is
                        // actually exercised. Anything else is an inline string,
                        // which avoids needing a shared-string table. A value
                        // wrapped in single quotes is forced to text, for the
                        // codes an author formats as text on purpose, and one
                        // opening with `=` becomes a formula whose cached result
                        // is a string — the shape real workbooks use for a phone
                        // number, and the one that used to lose its leading plus.
                        if let Some(result) = value.strip_prefix('=') {
                            xml.push_str(&format!(
                                r#"<c r="{reference}" t="str"><f>"{result}"</f><v>{result}</v></c>"#
                            ));
                        } else if let Some(literal) = value.strip_prefix('\'') {
                            xml.push_str(&format!(
                                r#"<c r="{reference}" t="inlineStr"><is><t>{literal}</t></is></c>"#
                            ));
                        } else if value.parse::<f64>().is_ok() {
                            xml.push_str(&format!(
                                r#"<c r="{reference}"><v>{value}</v></c>"#
                            ));
                        } else {
                            xml.push_str(&format!(
                                r#"<c r="{reference}" t="inlineStr"><is><t>{value}</t></is></c>"#
                            ));
                        }
                    }
                    xml.push_str("</row>");
                }
                xml.push_str("</sheetData></worksheet>");
                zip.write_all(xml.as_bytes()).unwrap();
            }

            zip.finish().unwrap();
        }
        buffer
    }

    #[test]
    fn a_real_xlsx_reads_into_normalised_sheets() {
        let bytes = tiny_xlsx(&[
            (
                "Elections",
                &[
                    &["external_id", "presentation.i18n.en.name", "max_votes"],
                    &["board", "Board", "3"],
                    &["", "", ""],
                    &["council", "Council", "1"],
                ],
            ),
            ("Read Me", &[&["This tab is documentation"]]),
        ]);

        let workbook = read_xlsx(&bytes).unwrap();

        // Tab names normalise, so lookup does not depend on how a tab is spelled.
        assert_eq!(workbook.rows(SHEET_ELECTIONS).len(), 2);
        assert_eq!(workbook.rows(SHEET_REPORTS).len(), 0);

        // The blank row in the middle is dropped without shifting the numbering:
        // "council" is on line 4 of the spreadsheet.
        let council = &workbook.rows(SHEET_ELECTIONS)[1];
        assert_eq!(council.number, 4);
        assert_eq!(council.text("external_id"), Some("council"));

        // Coercion has happened: "3" is a number by the time it is a bundle.
        let board = &workbook.rows(SHEET_ELECTIONS)[0];
        assert_eq!(board.get("max_votes"), Some(&json!(3)));
        assert_eq!(
            Value::Object(board.overrides(&["external_id"]).unwrap()),
            json!({
                "presentation": {"i18n": {"en": {"name": "Board"}}},
                "max_votes": 3,
            })
        );

        // And the documentation tab is reported rather than silently ignored.
        assert_eq!(workbook.unread_sheets(), vec!["Read Me"]);
    }

    #[test]
    fn an_international_phone_number_keeps_its_leading_plus() {
        // Real workbooks compute contact details with a formula, and its cached
        // result is a string in the file. calamine before 0.36 tried a float parse
        // on that string first — and Rust's float parser accepts a leading plus —
        // so "+33645312453" arrived as the number 33645312453. A voter's phone
        // number silently lost its country prefix, and nothing said so.
        let bytes = tiny_xlsx(&[(
            "Admin Users",
            &[
                &["username", "sequent.read-only.mobile-number"],
                &["admin1", "=+33645312453"],
            ],
        )]);
        let workbook = read_xlsx(&bytes).unwrap();
        assert_eq!(
            workbook.rows("adminusers")[0]
                .get("sequent.read-only.mobile-number"),
            Some(&json!("+33645312453"))
        );
    }

    #[test]
    fn a_formula_whose_result_is_text_stays_text_even_when_it_looks_numeric() {
        // What `t="str"` means in the file: the cached result *is* a string. A
        // formula that computes a number is written as a numeric cell instead, so
        // trusting the marker is right — and it is what keeps a member id computed
        // by a formula from losing its leading zeros.
        let bytes = tiny_xlsx(&[(
            "Voters",
            &[&["username", "member_id"], &["v1", "=007"]],
        )]);
        let workbook = read_xlsx(&bytes).unwrap();
        assert_eq!(
            workbook.rows("voters")[0].get("member_id"),
            Some(&json!("007"))
        );
    }

    #[test]
    fn a_code_formatted_as_text_keeps_its_leading_zeros() {
        // The reason a text cell is never reparsed as a number: member id 007 is
        // not 7, and turning it into one would silently fail to match a voter.
        let bytes = tiny_xlsx(&[(
            "Voters",
            &[&["external_id", "member_id"], &["v1", "'007"]],
        )]);
        let workbook = read_xlsx(&bytes).unwrap();
        assert_eq!(
            workbook.rows("voters")[0].get("member_id"),
            Some(&json!("007"))
        );
    }
}
