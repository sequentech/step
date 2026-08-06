// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * Register variants of a language, kept as pure string handling so it can be
 * unit-tested without pulling in the WASM bindings that `i18n.ts` needs.
 *
 * `es-tu` addresses the voter as *tú* where `es` uses *usted*; `cat-tu` as *tu*
 * where `cat` uses *vós*. Plain `es` and `cat` stay formal, so an existing
 * deployment does not change register on upgrade — a deployment opts in by
 * adding the variant code to `enabled_language_codes`.
 *
 * `locale.rs` implements the same rule for the Rust side; the two must agree.
 */

/** Marks the informal-register variant of an internal language code. */
export const INFORMAL_SUFFIX = "tu"

/**
 * Splits a locale into its primary subtag and whether it asks for the informal
 * register.
 *
 * The marker may arrive bare, as in the internal code `es-tu`, or behind the
 * BCP 47 private-use singleton, as in `es-x-tu` — which is what `<html lang>`
 * carries. Region and script subtags are ignored, so `es-ES-x-tu` is still
 * Spanish informal and `es-MX` is not.
 */
export const splitRegister = (lang: string): {base: string; informal: boolean} => {
    const [primary, ...rest] = lang.split("-")
    return {
        base: primary,
        informal: rest.some((part) => part.toLowerCase() === INFORMAL_SUFFIX),
    }
}

/** Re-attaches the register marker to a normalised internal code. */
export const withRegister = (base: string, informal: boolean): string =>
    informal ? `${base}-${INFORMAL_SUFFIX}` : base

/**
 * Re-attaches the register to a BCP 47 tag. Register is not a registered BCP 47
 * subtag, so it goes in a private-use sequence: `es` + informal -> `es-x-tu`.
 * That is well-formed, and assistive technology falls back to the base language
 * rather than rejecting an unregistered variant.
 */
export const withRegisterBCP47 = (bcp47Base: string, informal: boolean): string =>
    informal ? `${bcp47Base.toLowerCase()}-x-${INFORMAL_SUFFIX}` : bcp47Base
