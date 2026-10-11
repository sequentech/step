// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * The reference a voter gives support while their enrollment is reviewed,
 * such as "Q5KW-8RFX": Crockford base32 of the first 40 bits of SHA-256 of
 * the application's id (Keycloak's `SupportReference`). It can't be turned
 * back into the id, so the server compares it
 * (`find_applications_by_support_reference`).
 */
const ALPHABET = "0123456789ABCDEFGHJKMNPQRSTVWXYZ"

/**
 * A search text read as a support reference, as Crockford base32 reads it:
 * any case, with or without the dash or a space, I and L as 1, O as 0.
 * `null` when the text isn't one.
 */
export const normalizeSupportReference = (text: string): string | null => {
    const compact = text
        .trim()
        .toUpperCase()
        .replace(/[\s-]/g, "")
        .replace(/[IL]/g, "1")
        .replace(/O/g, "0")
    if (compact.length !== 8 || compact.split("").some((char) => !ALPHABET.includes(char))) {
        return null
    }
    return `${compact.slice(0, 4)}-${compact.slice(4)}`
}

/**
 * The list filter that carries a resolved reference: the applications whose
 * reference is the one searched, kept with the reference it was resolved for.
 */
export const SUPPORT_REFERENCE_FILTER = "support_reference"

export interface ISupportReferenceMatch {
    reference: string
    ids: string[]
}
