// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * Rust's string handling where the portal must give the backend's answer
 * (the voter's channel, a masked destination). JavaScript's `trim` and
 * `toUpperCase` differ from Rust's: `trim` also drops U+FEFF but keeps U+0085,
 * and `toUpperCase` maps `ſ` to `S`.
 */

/** `char::is_whitespace`: the Unicode White_Space property. */
const RUST_WHITESPACE = "\t\n\v\f\r \u0085   -     　"
const LEADING = new RegExp(`^[${RUST_WHITESPACE}]+`)
const TRAILING = new RegExp(`[${RUST_WHITESPACE}]+$`)
const ANY = new RegExp(`[${RUST_WHITESPACE}]`)

/** `str::trim`. */
export const rustTrim = (value: string): string => value.replace(LEADING, "").replace(TRAILING, "")

/** Whether the text has a character `char::is_whitespace` holds. */
export const hasRustWhitespace = (value: string): boolean => ANY.test(value)

/** `str::to_ascii_uppercase`: a–z only. */
export const toAsciiUppercase = (value: string): string =>
    value.replace(/[a-z]/g, (letter) => letter.toUpperCase())
