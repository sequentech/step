// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Writing a [`Workbook`] out as an `.xlsx`.
//!
//! The other half of [`super::xlsx`], and the reason a delivery can carry the
//! spreadsheet somebody actually works in rather than only the plan the wizard
//! reads. One format, two directions, and the reader's own regression tests are
//! pointed at this writer — a file it produces has to satisfy every case the
//! reader was taught to handle.
//!
//! **Hand-written rather than `rust_xlsxwriter`, and the reason is not taste.**
//! That crate compiles for `wasm32-unknown-unknown` and then panics at run time:
//! `Workbook::new` eagerly builds `DocProperties`, which calls
//! `ExcelDateTime::utc_now` → `SystemTime::now`, and there is no way to construct
//! a workbook without going through it. `cargo check` cannot see that, so it would
//! have shipped. Its `wasm` feature swaps the clock, but only through a
//! target-specific dependency arm that no CI job exercises — and it would drag a
//! second copy of the zip crate into a bundle that is already 4.7 MB and goes to a
//! browser. What is needed here is a header row and typed cells; that is this file.
//!
//! Only what the format needs: inline strings, numbers, booleans. No shared-string
//! table, no styles, no column widths. A [`Workbook`] holds `serde_json::Value`
//! and nothing else, so there is nothing else to write.

use std::io::{Cursor, Write};

use serde_json::Value;
use zip::write::SimpleFileOptions;

use crate::election_config::paths::cell_text;
use crate::election_config::problem::{Code, Problem};
use crate::election_config::sheet::{multi_value_columns, Origin, Workbook};

/// Excel's own limit on a tab name.
const NAME_LIMIT: usize = 31;

/// The characters Excel refuses in a tab name.
const NAME_REFUSES: [char; 7] = ['[', ']', ':', '*', '?', '/', '\\'];

/// The spreadsheet column at `index`: `A`, `Z`, `AA`, `AB`, … `ZZ`, `AAA`.
///
/// A function rather than `(b'A' + index)`, which is what the reader's test helper
/// did and which silently produces `[` for the 27th column. A Voters sheet carrying
/// a client's own columns passes 27 routinely.
pub(crate) fn column_name(index: usize) -> String {
    let mut name = String::new();
    let mut at = index;
    loop {
        name.insert(0, char::from(b'A' + (at % 26) as u8));
        if at < 26 {
            return name;
        }
        at = at / 26 - 1;
    }
}

/// XML text, escaped.
///
/// All five, including the apostrophe. A candidate called `Smith & Jones` is
/// ordinary and produced a file nothing could open.
fn escaped(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            // A control character other than tab, newline or return is not legal
            // in XML at all, and a spreadsheet somebody pasted into carries them.
            other
                if (other as u32) < 0x20
                    && other != '\t'
                    && other != '\n'
                    && other != '\r' =>
            {
                out.push(' ')
            }
            other => out.push(other),
        }
    }
    out
}

/// A tab name Excel will open, or a problem saying why not.
///
/// Refused rather than quietly truncated: two sheets whose names differ past the
/// 31st character would become one tab, and `Workbook` guarantees they are
/// distinct. Silently merging them is worse than not writing the file.
fn checked_name(name: &str) -> Result<&str, Problem> {
    if let Some(bad) = name.chars().find(|each| NAME_REFUSES.contains(each)) {
        return Err(Problem::error(
            Code::InvalidValue,
            format!("sheet '{name}'"),
            format!(
                "a spreadsheet tab cannot contain '{bad}', so this workbook \
                 cannot be written as a file"
            ),
        )
        .at(&Origin::sheet(name)));
    }
    if name.chars().count() > NAME_LIMIT {
        return Err(Problem::error(
            Code::InvalidValue,
            format!("sheet '{name}'"),
            format!(
                "a spreadsheet tab cannot be longer than {NAME_LIMIT} characters, \
                 and this one is {}",
                name.chars().count()
            ),
        )
        .at(&Origin::sheet(name)));
    }
    Ok(name)
}

/// One cell, typed the way a spreadsheet stores it.
///
/// Booleans and numbers are written as booleans and numbers so a person opening
/// the file sees what the platform sees — and so the reader's own coercion is
/// exercised rather than bypassed. Everything else is an inline string, which is
/// what makes a shared-string table unnecessary.
fn cell_xml(reference: &str, value: &Value, multi_value: bool) -> String {
    match value {
        Value::Bool(flag) => {
            let bit = u8::from(*flag);
            format!(r#"<c r="{reference}" t="b"><v>{bit}</v></c>"#)
        }
        Value::Number(number) => {
            format!(r#"<c r="{reference}"><v>{number}</v></c>"#)
        }
        other => {
            let text = escaped(&cell_text(other, multi_value));
            format!(
                r#"<c r="{reference}" t="inlineStr"><is><t xml:space="preserve">{text}</t></is></c>"#
            )
        }
    }
}

/// Write a workbook as the bytes of an `.xlsx`.
///
/// Reproducible: the same workbook gives the same bytes, because every member
/// carries the fixed timestamp [`super::archive::zip`] already uses for the same
/// reason. A delivery somebody diffs against last week's is the point.
pub fn write_xlsx(workbook: &Workbook) -> Result<Vec<u8>, Problem> {
    let sheets = workbook.sheets();
    for sheet in sheets {
        checked_name(&sheet.name)?;
    }

    let mut buffer = Vec::new();
    let outcome = (|| -> Result<(), zip::result::ZipError> {
        let mut zip = zip::ZipWriter::new(Cursor::new(&mut buffer));
        // The same fixed instant `archive::zip` uses, and for the same reason: a
        // clock in here would make two builds of one plan different files.
        let options = SimpleFileOptions::default()
            .last_modified_time(
                zip::DateTime::from_date_and_time(2026, 1, 1, 0, 0, 0)
                    .unwrap_or_default(),
            )
            .unix_permissions(0o644);

        zip.start_file("[Content_Types].xml", options)?;
        let mut types = String::from(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>"#,
        );
        for index in 1..=sheets.len() {
            types.push_str(&format!(
                r#"<Override PartName="/xl/worksheets/sheet{index}.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>"#
            ));
        }
        types.push_str("</Types>");
        zip.write_all(types.as_bytes())?;

        zip.start_file("_rels/.rels", options)?;
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
        )?;

        zip.start_file("xl/workbook.xml", options)?;
        let mut book = String::from(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets>"#,
        );
        for (index, sheet) in sheets.iter().enumerate() {
            let id = index + 1;
            book.push_str(&format!(
                r#"<sheet name="{}" sheetId="{id}" r:id="rId{id}"/>"#,
                escaped(&sheet.name)
            ));
        }
        book.push_str("</sheets></workbook>");
        zip.write_all(book.as_bytes())?;

        zip.start_file("xl/_rels/workbook.xml.rels", options)?;
        let mut rels = String::from(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">"#,
        );
        for index in 1..=sheets.len() {
            rels.push_str(&format!(
                r#"<Relationship Id="rId{index}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet{index}.xml"/>"#
            ));
        }
        rels.push_str("</Relationships>");
        zip.write_all(rels.as_bytes())?;

        for (index, sheet) in sheets.iter().enumerate() {
            zip.start_file(
                format!("xl/worksheets/sheet{}.xml", index + 1),
                options,
            )?;
            let multi = multi_value_columns(&sheet.key);

            let mut xml = String::from(
                r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData>"#,
            );

            // Row 1 is the headers, which is what makes the file readable by the
            // reader: `Sheet::from_grid` takes the first row as the column names.
            xml.push_str(r#"<row r="1">"#);
            for (at, header) in sheet.headers.iter().enumerate() {
                if header.is_empty() {
                    continue;
                }
                xml.push_str(&cell_xml(
                    &format!("{}1", column_name(at)),
                    &Value::String(header.clone()),
                    false,
                ));
            }
            xml.push_str("</row>");

            for (offset, row) in sheet.rows.iter().enumerate() {
                let number = offset + 2;
                xml.push_str(&format!(r#"<row r="{number}">"#));
                for (at, header) in sheet.headers.iter().enumerate() {
                    if header.is_empty() {
                        continue;
                    }
                    // Absent rather than null: a blank cell is how the format
                    // says "the author said nothing", which is not the same as
                    // the literal `null`.
                    if let Some(value) = row.get(header) {
                        xml.push_str(&cell_xml(
                            &format!("{}{number}", column_name(at)),
                            value,
                            multi.contains(&header.as_str()),
                        ));
                    }
                }
                xml.push_str("</row>");
            }

            xml.push_str("</sheetData></worksheet>");
            zip.write_all(xml.as_bytes())?;
        }

        zip.finish()?;
        Ok(())
    })();

    outcome.map_err(|error| {
        Problem::error(
            Code::InvalidValue,
            "workbook",
            format!("this workbook could not be written as a file: {error}"),
        )
    })?;

    Ok(buffer)
}

#[cfg(test)]
#[path = "xlsx_write_tests.rs"]
mod xlsx_write_tests;
