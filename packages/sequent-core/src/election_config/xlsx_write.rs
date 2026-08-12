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
/// One cell, optionally carrying a `cellXfs` index.
///
/// `style` is `None` for the overwhelming majority — a census is a hundred
/// thousand rows of plain cells, and an `s="0"` on each of them is bytes that say
/// nothing.
fn cell_xml(
    reference: &str,
    value: &Value,
    multi_value: bool,
    style: Option<u32>,
) -> String {
    let s_attr = match style {
        Some(index) => format!(r#" s="{index}""#),
        None => String::new(),
    };
    match value {
        Value::Bool(flag) => {
            let bit = u8::from(*flag);
            format!(r#"<c r="{reference}"{s_attr} t="b"><v>{bit}</v></c>"#)
        }
        Value::Number(number) => {
            format!(r#"<c r="{reference}"{s_attr}><v>{number}</v></c>"#)
        }
        other => {
            let text = escaped(&cell_text(other, multi_value));
            format!(
                r#"<c r="{reference}"{s_attr} t="inlineStr"><is><t xml:space="preserve">{text}</t></is></c>"#
            )
        }
    }
}

/// Write a workbook as the bytes of an `.xlsx`.
///
/// Reproducible: the same workbook gives the same bytes, because every member
/// carries the fixed timestamp [`super::archive::zip`] already uses for the same
/// reason. A delivery somebody diffs against last week's is the point.
/// The three cell formats this writer uses, as a whole `xl/styles.xml`.
///
/// Hand-written, like the rest of the part: the format wants a fixed order —
/// `numFmts`, `fonts`, `fills`, `borders`, `cellStyleXfs`, `cellXfs` — and every
/// index below is a position in one of those lists, not a name.
///
/// `cellXfs` is what a cell's `s=` points at:
///
///   - **0** — the default. No `s=` written, which keeps a hundred thousand census
///     cells free of an attribute that says nothing. It is not empty, though: it
///     carries `vertical="top"`, and that is what a cell with no `s=` resolves to.
///     Without it a row whose description wrapped to three lines showed the id and
///     the name **at the bottom** of it, floating away from the sentence they
///     belong to — the spreadsheet default is bottom, and it only shows once a row
///     is taller than one line. Putting it on `0` rather than on every cell is the
///     whole reason the census stays free of the attribute.
///   - **1** — the header: white, bold, on the brand navy, and vertically centred
///     so a wrapped header sits with its neighbours.
///   - **2** — wrapped body text, for the prose columns.
///
/// The navy is `#0F054C`, the same one the wizard is built from. `FF` in front is
/// the alpha the format requires — ARGB, not RGB, and a six-digit value here is
/// silently ignored by LibreOffice, which is a header that renders white-on-white.
const STYLES: &str = concat!(
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
    r#"<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">"#,
    r#"<fonts count="2">"#,
    r#"<font><sz val="11"/><name val="Calibri"/></font>"#,
    r#"<font><b/><color rgb="FFFFFFFF"/><sz val="11"/><name val="Calibri"/></font>"#,
    r#"</fonts>"#,
    // The first two fills are reserved by the format — `none` and `gray125` — and
    // a file that omits them renders every fill one position out.
    r#"<fills count="3">"#,
    r#"<fill><patternFill patternType="none"/></fill>"#,
    r#"<fill><patternFill patternType="gray125"/></fill>"#,
    r#"<fill><patternFill patternType="solid"><fgColor rgb="FF0F054C"/><bgColor indexed="64"/></patternFill></fill>"#,
    r#"</fills>"#,
    r#"<borders count="1"><border><left/><right/><top/><bottom/><diagonal/></border></borders>"#,
    // The alignment is repeated here, on the `Normal` named style, and that is not
    // belt and braces. A cell with no `s=` is supposed to resolve to `cellXfs[0]`;
    // LibreOffice resolves it through `Normal` instead, so putting it in only one
    // of the two places renders correctly in one program and wrongly in the other.
    r#"<cellStyleXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0" applyAlignment="1"><alignment vertical="top"/></xf></cellStyleXfs>"#,
    r#"<cellXfs count="3">"#,
    r#"<xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0" applyAlignment="1"><alignment vertical="top"/></xf>"#,
    r#"<xf numFmtId="0" fontId="1" fillId="2" borderId="0" xfId="0" applyFont="1" applyFill="1" applyAlignment="1"><alignment vertical="center" wrapText="1"/></xf>"#,
    r#"<xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0" applyAlignment="1"><alignment vertical="top" wrapText="1"/></xf>"#,
    r#"</cellXfs>"#,
    // The `Normal` named style. Nothing here points at it, but a reader that finds
    // no default style says so — `openpyxl` warns and substitutes its own — and a
    // reader substituting its own defaults is exactly how the alignment above stops
    // being what this file says it is.
    r#"<cellStyles count="1"><cellStyle name="Normal" xfId="0" builtinId="0"/></cellStyles>"#,
    r#"</styleSheet>"#,
);

/// The `cellXfs` index for a header cell.
const HEADER_STYLE: u32 = 1;
/// The `cellXfs` index for a wrapped body cell.
const WRAPPED_STYLE: u32 = 2;

/// How wide a column should be, and whether its text may wrap.
///
/// Decided from what is *in* the column rather than from a table of known names,
/// because the columns belong to the client: a Voters sheet carries whatever
/// reporting breakouts they keep, and a hand-kept list would be stale the first
/// time somebody added one.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Shape {
    /// Width in Excel's character units.
    pub width: f64,
    /// Whether the cells wrap onto a second line.
    pub wrap: bool,
}

/// Never narrower than this, so a one-letter header is still clickable.
const NARROWEST: f64 = 9.0;

/// Never wider than this when the text can wrap — beyond it, wrapping is better
/// than a column somebody has to scroll sideways past.
const WIDEST_PROSE: f64 = 52.0;

/// The cap for anything that must stay on one line.
///
/// Markup and identifiers are not read left-to-right like a sentence; they are
/// scanned, and the useful thing is to see the start and be able to widen the
/// column by hand. A template pasted into one cell can be two thousand characters,
/// and a column that honoured that would push every other column off the screen.
const WIDEST_FIXED: f64 = 60.0;

/// How many rows to look at before deciding. Enough to be representative, few
/// enough that a hundred-thousand-row census does not pay for a cosmetic choice.
const SAMPLED: usize = 200;

/// The widest a column may be made by its *header* alone.
///
/// Deliberately much narrower than [`WIDEST_PROSE`], and this is what the sample
/// plan showed the need for: these headers are field paths, so
/// `presentation.language_conf.enabled_language_codes` was making a column 51 wide
/// to hold a two-letter value. Half the sheet was header. Past this the header
/// wraps onto a second line instead — which is what the header format's `wrapText`
/// is for, and why [`header_lines`] exists to give row 1 the height to show it.
const HEADER_WIDE: f64 = 26.0;

/// A row of default-size text, in points. Excel's own default for Calibri 11.
const LINE_HEIGHT: f64 = 15.0;

/// How many lines row 1 needs, given each column's header and final width.
///
/// Computed rather than left to the viewer. `wrapText` with no explicit height is
/// supposed to auto-fit, and Excel does — but a height nobody wrote is a height
/// every reader is free to guess, and the guess that costs is the one that clips
/// the second line. One number, written down, renders the same everywhere.
///
/// The width less one, because a character's worth of the cell is padding and a
/// header that exactly fills its column wraps its last character anyway.
pub(crate) fn header_lines(headers: &[String], shape: &[Shape]) -> usize {
    headers
        .iter()
        .zip(shape)
        .map(|(header, each)| {
            let per_line = (each.width - 1.0).max(1.0);
            (header.chars().count() as f64 / per_line).ceil() as usize
        })
        .max()
        .unwrap_or(1)
        .max(1)
}

/// Whether a value is the sort of thing that must not be wrapped.
///
/// Markup, a handlebars template, JSON, a URL, or a long run with no spaces at
/// all. Wrapping any of these makes them *less* readable, which is the opposite of
/// the point: somebody opening the Templates sheet is there to read the markup.
fn is_markup(text: &str) -> bool {
    text.contains('<')
        || text.contains("{{")
        || text.starts_with('{')
        || text.starts_with('[')
        || text.starts_with("http://")
        || text.starts_with("https://")
        // A long token with no break in it — an id, a base64 blob, a path.
        || (text.len() > 40 && !text.contains(' '))
}

/// The shape of every column on a sheet, in column order.
pub(crate) fn shapes(headers: &[String], values: &[Vec<String>]) -> Vec<Shape> {
    headers
        .iter()
        .enumerate()
        .map(|(at, header)| {
            let seen: Vec<&String> = values
                .iter()
                .take(SAMPLED)
                .filter_map(|row| row.get(at))
                .filter(|text| !text.is_empty())
                .collect();

            let markup = seen.iter().any(|text| is_markup(text));
            let longest = seen
                .iter()
                .map(|text| text.chars().count())
                .max()
                .unwrap_or(0);
            // The header has a say, but only up to a point: past `HEADER_WIDE` it
            // wraps rather than widening a column whose values are two characters
            // long.
            let needed = (longest as f64)
                .max((header.chars().count() as f64).min(HEADER_WIDE))
                + 2.0;

            if markup {
                Shape {
                    width: needed.clamp(NARROWEST, WIDEST_FIXED),
                    wrap: false,
                }
            } else if needed > WIDEST_PROSE {
                // Prose past the cap: fixed at the cap and wrapped, so the whole
                // value is readable in two or three lines rather than running off
                // the sheet.
                Shape {
                    width: WIDEST_PROSE,
                    wrap: true,
                }
            } else {
                Shape {
                    width: needed.max(NARROWEST),
                    wrap: false,
                }
            }
        })
        .collect()
}

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
        // After the sheets, so the ids stay `rId1..n` for them — the workbook part
        // refers to sheets by that id and renumbering would repoint them.
        rels.push_str(&format!(
            r#"<Relationship Id="rId{}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/>"#,
            sheets.len() + 1
        ));
        rels.push_str("</Relationships>");
        zip.write_all(rels.as_bytes())?;

        zip.start_file("xl/styles.xml", options)?;
        zip.write_all(STYLES.as_bytes())?;

        for (index, sheet) in sheets.iter().enumerate() {
            zip.start_file(
                format!("xl/worksheets/sheet{}.xml", index + 1),
                options,
            )?;
            let multi = multi_value_columns(&sheet.key);

            let mut xml = String::from(
                r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">"#,
            );

            // What each column holds, as text, for deciding how wide it should be
            // and whether it may wrap. Read once per sheet rather than per cell.
            let sampled: Vec<Vec<String>> = sheet
                .rows
                .iter()
                .take(SAMPLED)
                .map(|row| {
                    sheet
                        .headers
                        .iter()
                        .map(|header| {
                            row.get(header)
                                .map(|value| {
                                    cell_text(
                                        value,
                                        multi_value_columns(&sheet.key)
                                            .contains(&header.as_str()),
                                    )
                                })
                                .unwrap_or_default()
                        })
                        .collect()
                })
                .collect();
            let shape = shapes(&sheet.headers, &sampled);

            // The header row stays put when somebody scrolls a census. Written
            // before `<cols>`, which the format requires.
            xml.push_str(
                r#"<sheetViews><sheetView workbookViewId="0"><pane ySplit="1" topLeftCell="A2" activePane="bottomLeft" state="frozen"/></sheetView></sheetViews>"#,
            );

            if !shape.is_empty() {
                xml.push_str("<cols>");
                for (at, each) in shape.iter().enumerate() {
                    let column = at + 1;
                    xml.push_str(&format!(
                        r#"<col min="{column}" max="{column}" width="{:.1}" customWidth="1"/>"#,
                        each.width
                    ));
                }
                xml.push_str("</cols>");
            }
            xml.push_str("<sheetData>");

            // Row 1 is the headers, which is what makes the file readable by the
            // reader: `Sheet::from_grid` takes the first row as the column names.
            xml.push_str(&format!(
                r#"<row r="1" ht="{:.1}" customHeight="1">"#,
                header_lines(&sheet.headers, &shape) as f64 * LINE_HEIGHT
            ));
            for (at, header) in sheet.headers.iter().enumerate() {
                if header.is_empty() {
                    continue;
                }
                xml.push_str(&cell_xml(
                    &format!("{}1", column_name(at)),
                    &Value::String(header.clone()),
                    false,
                    Some(HEADER_STYLE),
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
                            // Only the wrapping columns carry a style. Everything
                            // else uses the default, which costs no attribute on
                            // the hundred thousand cells of a census.
                            shape
                                .get(at)
                                .filter(|each| each.wrap)
                                .map(|_| WRAPPED_STYLE),
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
