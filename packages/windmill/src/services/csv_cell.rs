// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Values in voters CSV cells that a spreadsheet would run as formulas.

use std::borrow::Cow;

/// What a cell can start with for a spreadsheet to run it as a formula, as
/// OWASP lists them.
const FORMULA_PREFIXES: [char; 11] = [
    '=', '+', '-', '@', '\t', '\r', '\n',
    // Full-width `=`, `+`, `-` and `@`, which some locales read as those.
    '\u{ff1d}', '\u{ff0b}', '\u{ff0d}', '\u{ff20}',
];

/// Makes a spreadsheet show the rest of a cell as text.
const TEXT_PREFIX: char = '\'';

/// Whether a spreadsheet would run `value` as a formula once the quotes it
/// starts with are left out. A prefix alone, like the `-` export writes for a
/// voter without an area, is not one.
fn is_formula(value: &str) -> bool {
    let value = value.trim_start_matches(TEXT_PREFIX);
    value.starts_with(FORMULA_PREFIXES) && value.chars().nth(1).is_some()
}

/// `value` as export writes it in a cell: after a `'` if a spreadsheet would
/// run it as a formula, so that it shows it as text. A value that already
/// starts with quotes before a formula gets one more, so that
/// [`unescape_formula`] gives every value back unchanged.
pub(crate) fn escape_formula(value: &str) -> Cow<'_, str> {
    if is_formula(value) {
        Cow::Owned(format!("{TEXT_PREFIX}{value}"))
    } else {
        Cow::Borrowed(value)
    }
}

/// The value that [`escape_formula`] wrote as `cell`, without the `'` it put
/// before a formula. A spreadsheet that saves the file drops it, which leaves
/// the value itself.
pub(crate) fn unescape_formula(cell: &str) -> &str {
    match cell.strip_prefix(TEXT_PREFIX) {
        Some(value) if is_formula(value) => value,
        _ => cell,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FORMULAS: [&str; 7] = [
        "=HYPERLINK(\"https://example.com\",\"x\")",
        "+34600000000",
        "-2+3",
        "@SUM(A1)",
        "\t=1+1",
        "\n=1+1",
        "\u{ff1d}1+1",
    ];

    #[test]
    fn formulas_are_written_after_a_quote_and_read_back() {
        for formula in FORMULAS {
            let cell = escape_formula(formula);

            assert_eq!(cell, format!("'{formula}"));
            assert_eq!(unescape_formula(&cell), formula);
        }
    }

    /// A spreadsheet that saves the file drops the quote.
    #[test]
    fn formulas_without_the_quote_read_back_as_they_are() {
        for formula in FORMULAS {
            assert_eq!(unescape_formula(formula), formula);
        }
    }

    #[test]
    fn other_values_are_written_as_they_are() {
        for value in ["Ana", "", "-", "GIAMBI30-3-31", "a=b", "'text", "'-"] {
            assert_eq!(escape_formula(value), value);
            assert_eq!(unescape_formula(value), value);
        }
    }

    /// Otherwise import would read them back without their first quote.
    #[test]
    fn values_with_quotes_before_a_formula_get_one_more() {
        for value in ["'=1+1", "''+34600000000"] {
            let cell = escape_formula(value);

            assert_eq!(cell, format!("'{value}"));
            assert_eq!(unescape_formula(&cell), value);
        }
    }
}
