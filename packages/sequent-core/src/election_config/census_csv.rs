// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Reading a census CSV a batch at a time.
//!
//! A union census is six figures of members, and the wizard's own path built three
//! whole copies of one before a single row was usable — the file's text, an array
//! of string arrays from a JavaScript CSV parser, and an array of objects. All
//! alive at once, all before anything had been stored.
//!
//! So the file is parsed here, and handed over in batches the caller consumes and
//! drops. The high-water mark becomes the text plus one batch.
//!
//! **Why this lives in the core rather than in the wizard.** A CSV parser written
//! in TypeScript beside the one here is a second answer to "what is in this file",
//! and the census is the list where that costs most: a quoted comma read one way
//! by the loader and another by the saver turns one member into two on the way
//! out. There is exactly one census CSV dialect and it is this one — the same
//! `csv` crate the rest of the core reads sheets with.
//!
//! This deliberately does **not** produce SQL. The wizard holds its census in
//! SQLite and the temptation was to emit `VALUES` tuples straight from here, which
//! would have been faster still and would have put the wizard's schema inside the
//! core. What crosses the boundary is rows of strings; what they become is the
//! caller's business.

use serde::Serialize;

/// Columns the platform derives or regenerates, so a value in the file is dropped.
///
/// Mirrors `VOTER_LEADING_COLUMNS` in `build_tables.rs` for the ones a census file
/// should not be dictating: an `id` the importer assigns, the elections a voter is
/// authorised for, and the two flags the platform sets itself.
const DERIVED: &[&str] =
    &["id", "authorized-election-ids", "enabled", "email_verified"];

/// The one column a census cannot do without.
const REQUIRED: &str = "username";

/// What a census file says about itself, before any rows are read.
#[derive(Debug, Serialize)]
pub struct CensusHeader {
    /// The columns, trimmed, in file order.
    pub columns: Vec<String>,
    /// What was odd about the file without being wrong with it.
    pub notes: Vec<String>,
}

/// A census file, read a batch at a time.
pub struct CensusCsv {
    reader: csv::Reader<std::io::Cursor<Vec<u8>>>,
    header: CensusHeader,
    /// Which of the header's columns survive into a voter, by position.
    kept: Vec<usize>,
}

impl CensusCsv {
    /// Read the header and get ready for the rows.
    ///
    /// Refuses two ways, and both are refusals rather than notes: a file nothing
    /// can be read from, and one with no `username`. The wizard used to surface the
    /// second as an amber "about that file" beside a census it had just emptied,
    /// which reads as a warning rather than as "your file was not loaded".
    pub fn new(text: &str) -> Result<Self, String> {
        let mut reader = csv::ReaderBuilder::new()
            .flexible(true)
            .from_reader(std::io::Cursor::new(text.as_bytes().to_vec()));

        let headers = reader
            .headers()
            .map_err(|e| format!("That file could not be read: {e}"))?;
        if headers.is_empty() {
            return Err("That file has nothing in it.".to_owned());
        }

        let columns: Vec<String> =
            headers.iter().map(|each| each.trim().to_owned()).collect();
        if !columns.iter().any(|each| each == REQUIRED) {
            return Err(
                "No `username` column. It is the one column a census cannot do \
                 without — it is what a voter signs in as."
                    .to_owned(),
            );
        }

        let mut notes = Vec::new();
        let derived: Vec<&str> = columns
            .iter()
            .filter(|each| DERIVED.contains(&each.as_str()))
            .map(|each| each.as_str())
            .collect();
        if !derived.is_empty() {
            // Said out loud rather than ignored: somebody who exported from the
            // platform and is loading it back should know these are regenerated.
            notes.push(format!(
                "Ignoring {} — the platform fills these in itself, so a value \
                 here would be overwritten.",
                derived.join(", ")
            ));
        }

        let kept: Vec<usize> = columns
            .iter()
            .enumerate()
            .filter(|(_, name)| {
                !name.is_empty() && !DERIVED.contains(&name.as_str())
            })
            .map(|(at, _)| at)
            .collect();

        let header = CensusHeader {
            columns: kept.iter().map(|at| columns[*at].clone()).collect(),
            notes,
        };

        Ok(Self {
            reader,
            header,
            kept,
        })
    }

    pub fn header(&self) -> &CensusHeader {
        &self.header
    }

    /// The next `size` rows, or fewer at the end. Empty when the file is done.
    ///
    /// Values are trimmed and returned in the order [`CensusHeader::columns`] gives
    /// — the derived columns are already dropped, so a caller can zip the two
    /// without knowing which were skipped.
    ///
    /// A short row is padded rather than refused. A spreadsheet that omits trailing
    /// empty cells is not a broken file, and the wizard's own reader has always
    /// treated a missing cell as blank.
    pub fn next_batch(
        &mut self,
        size: usize,
    ) -> Result<Vec<Vec<String>>, String> {
        let mut batch = Vec::with_capacity(size.min(8_192));
        let mut record = csv::StringRecord::new();

        while batch.len() < size {
            match self.reader.read_record(&mut record) {
                Ok(true) => {}
                Ok(false) => break,
                Err(e) => {
                    return Err(format!("That file could not be read: {e}"))
                }
            }

            // A wholly empty line is not a member. A file ending in a newline is
            // the commonest thing in the world and must not load a blank voter.
            if record.iter().all(|value| value.trim().is_empty()) {
                continue;
            }

            batch.push(
                self.kept
                    .iter()
                    .map(|at| record.get(*at).unwrap_or("").trim().to_owned())
                    .collect(),
            );
        }

        Ok(batch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all(text: &str) -> Vec<Vec<String>> {
        let mut reader = CensusCsv::new(text).expect("header");
        let mut rows = Vec::new();
        loop {
            let batch = reader.next_batch(2).expect("batch");
            if batch.is_empty() {
                break;
            }
            rows.extend(batch);
        }
        rows
    }

    #[test]
    fn reads_a_plain_file() {
        let rows =
            all("username,email\nada,ada@example.org\ngrace,g@example.org\n");
        assert_eq!(
            rows,
            vec![
                vec!["ada".to_owned(), "ada@example.org".to_owned()],
                vec!["grace".to_owned(), "g@example.org".to_owned()],
            ]
        );
    }

    #[test]
    fn keeps_a_quoted_comma_in_one_field() {
        // "Lovelace, Ada" is one value. Split wrongly it becomes two columns and
        // everything after it lands under the wrong header.
        let rows = all("username,last_name\nada,\"Lovelace, Ada\"\n");
        assert_eq!(rows[0][1], "Lovelace, Ada");
    }

    #[test]
    fn keeps_a_doubled_quote() {
        let rows = all("username,note\nada,\"she said \"\"yes\"\"\"\n");
        assert_eq!(rows[0][1], "she said \"yes\"");
    }

    #[test]
    fn keeps_a_newline_inside_quotes() {
        // A pasted address with a line break. A reader that treats every newline
        // as a row boundary turns one member into two, and the second has no
        // username.
        let rows = all("username,address\nada,\"12 King St\nCambridge\"\n");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0][1], "12 King St\nCambridge");
    }

    #[test]
    fn reads_what_a_windows_spreadsheet_writes() {
        let rows = all("username,email\r\nada,a@b.org\r\n");
        assert_eq!(rows[0], vec!["ada".to_owned(), "a@b.org".to_owned()]);
    }

    #[test]
    fn a_trailing_newline_is_not_a_voter() {
        let rows = all("username\nada\n\n");
        assert_eq!(rows.len(), 1);
    }

    #[test]
    fn pads_a_row_that_stops_early() {
        // A spreadsheet that omits trailing empty cells is not a broken file.
        let rows = all("username,email,area.external_id\nada\n");
        assert_eq!(
            rows[0],
            vec!["ada".to_owned(), String::new(), String::new()]
        );
    }

    #[test]
    fn trims_the_header_and_the_values() {
        let rows = all(" username , email \n ada , a@b.org \n");
        let reader = CensusCsv::new(" username , email \n").expect("header");
        assert_eq!(reader.header().columns, vec!["username", "email"]);
        assert_eq!(rows[0], vec!["ada".to_owned(), "a@b.org".to_owned()]);
    }

    #[test]
    fn drops_the_columns_the_platform_regenerates() {
        let reader =
            CensusCsv::new("username,id,enabled,email\nada,7,true,a@b.org\n")
                .expect("header");
        assert_eq!(reader.header().columns, vec!["username", "email"]);
        assert_eq!(reader.header().notes.len(), 1);
        assert!(reader.header().notes[0].contains("id, enabled"));

        let rows = all("username,id,enabled,email\nada,7,true,a@b.org\n");
        assert_eq!(rows[0], vec!["ada".to_owned(), "a@b.org".to_owned()]);
    }

    #[test]
    fn refuses_a_file_with_no_username() {
        let refused = match CensusCsv::new("email\na@b.org\n") {
            Ok(_) => panic!("a census with no username column must be refused"),
            Err(message) => message,
        };
        assert!(refused.contains("username"));
    }

    #[test]
    fn refuses_an_empty_file() {
        assert!(CensusCsv::new("").is_err());
    }

    #[test]
    fn hands_over_exactly_the_batch_size_asked_for() {
        // The property the streaming depends on: a caller asking for five rows at
        // a time never has more than five in hand.
        let text = format!(
            "username\n{}\n",
            (0..25)
                .map(|at| format!("voter-{at}"))
                .collect::<Vec<_>>()
                .join("\n")
        );
        let mut reader = CensusCsv::new(&text).expect("header");
        assert_eq!(reader.next_batch(5).expect("batch").len(), 5);
        assert_eq!(reader.next_batch(5).expect("batch").len(), 5);
        assert_eq!(reader.next_batch(100).expect("batch").len(), 15);
        assert!(reader.next_batch(5).expect("batch").is_empty());
    }
}
