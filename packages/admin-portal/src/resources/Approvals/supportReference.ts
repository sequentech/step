// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * The reference a voter gives support while their enrollment is reviewed,
 * such as "Q5KW-8RFX". Port of Keycloak's `SupportReference`
 * (voter-enrollment): Crockford base32 of the first 40 bits of SHA-256 of
 * `"application:" + id`, the id in lower case, in two groups of four. It
 * comes from the application's id only, so staff can find the application
 * by it, and it can't be turned back into the id.
 */
const ALPHABET = "0123456789ABCDEFGHJKMNPQRSTVWXYZ"

export const supportReference = async (applicationId: string): Promise<string> => {
    const data = new TextEncoder().encode(`application:${applicationId.trim().toLowerCase()}`)
    const digest = new Uint8Array(await globalThis.crypto.subtle.digest("SHA-256", data))
    // 40 bits fit a double exactly.
    let bits = 0
    for (let i = 0; i < 5; i++) {
        bits = bits * 256 + digest[i]
    }
    let reference = ""
    for (let i = 7; i >= 0; i--) {
        reference += ALPHABET[Math.floor(bits / 2 ** (i * 5)) % 32]
        if (i === 4) reference += "-"
    }
    return reference
}

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

/** The ids among `ids` whose support reference is `reference`. */
export const findBySupportReference = async (
    ids: string[],
    reference: string
): Promise<string[]> => {
    const references = await Promise.all(ids.map(supportReference))
    return ids.filter((_, index) => references[index] === reference)
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
