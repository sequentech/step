// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Tracking IDs as voters read them: Crockford base32 in groups of four joined
// by hyphens, `FTBE-MHRX-...`. The short ID is the first two groups. The
// verifier compares the whole ID: a short ID has 40 bits, which a birthday
// search could match. A voter pseudonym, `FTBE:MHRX`, is never a Tracking ID.

export const BALLOT_ID_GROUP = 4
export const SHORT_BALLOT_ID_LENGTH = 8
export const FULL_BALLOT_ID_LENGTH = 52
export const BALLOT_ID_SEPARATOR = "-"
/** The groups that make the short ID, which the screens set in bold. */
export const SHORT_BALLOT_ID_GROUPS = SHORT_BALLOT_ID_LENGTH / BALLOT_ID_GROUP

/** What a voter typed or pasted, as a Tracking ID. */
export enum EBallotIdInput {
    EMPTY = "empty",
    SHORT = "short",
    FULL = "full",
    INVALID = "invalid",
}

const ALPHABET = /^[0-9A-HJKMNP-TV-Z]+$/
const GROUPS = new RegExp(`.{1,${BALLOT_ID_GROUP}}`, "g")

const readCharacter = (character: string): string => {
    const upper = character.toUpperCase()
    if (upper === "O") return "0"
    if (upper === "I" || upper === "L") return "1"
    return upper
}

/** The ID's characters: spaces, hyphens and case don't count, O is 0 and I
 * and L are 1. Anything else stays, so that it makes the ID invalid. */
export const ballotIdChars = (value: string): string =>
    Array.from(value.replace(/[\s-]+/g, ""), readCharacter).join("")

/** The ID's groups of four. */
export const ballotIdGroups = (value: string): string[] => ballotIdChars(value).match(GROUPS) ?? []

/** The ID in groups of four: `FTBE-MHRX-...`. */
export const groupBallotId = (value: string): string =>
    ballotIdGroups(value).join(BALLOT_ID_SEPARATOR)

/** The short ID: the first two groups, `FTBE-MHRX`. */
export const shortBallotId = (value: string): string =>
    groupBallotId(ballotIdChars(value).slice(0, SHORT_BALLOT_ID_LENGTH))

/** Whether the input is a short ID, a full one, or neither. */
export const ballotIdInput = (value: string): EBallotIdInput => {
    const chars = ballotIdChars(value)
    if (!chars) return EBallotIdInput.EMPTY
    if (!ALPHABET.test(chars)) return EBallotIdInput.INVALID
    if (chars.length === SHORT_BALLOT_ID_LENGTH) return EBallotIdInput.SHORT
    if (chars.length === FULL_BALLOT_ID_LENGTH) return EBallotIdInput.FULL
    return EBallotIdInput.INVALID
}

/** What an ID field shows as the voter types: the ID's characters, grouped, up
 * to a full ID. A colon stays, so that a pseudonym is refused, not read as an ID. */
export const typedBallotId = (value: string): string =>
    groupBallotId(
        ballotIdChars(value)
            .replace(/[^0-9A-HJKMNP-TV-Z:]/g, "")
            .slice(0, FULL_BALLOT_ID_LENGTH)
    )

/** Whether two IDs are the same, whatever their spacing and case. */
export const sameBallotId = (a: string, b: string): boolean => ballotIdChars(a) === ballotIdChars(b)

const DIGEST_BYTES = 32
const CROCKFORD = "0123456789ABCDEFGHJKMNPQRSTVWXYZ"

/** The 32-byte digest a full Tracking ID writes, in hex, as the entry of a
 * ballot holds it: the 52 characters are 260 bits, the first 256 the digest and
 * the last 4 zero. Undefined for anything else. */
export const trackingIdDigestHex = (value: string): string | undefined => {
    const chars = ballotIdChars(value)
    if (chars.length !== FULL_BALLOT_ID_LENGTH) return undefined
    let bits = ""
    for (const character of chars) {
        const index = CROCKFORD.indexOf(character)
        if (index < 0) return undefined
        bits += index.toString(2).padStart(5, "0")
    }
    if (/1/.test(bits.slice(DIGEST_BYTES * 8))) return undefined
    let hex = ""
    for (let at = 0; at < DIGEST_BYTES * 8; at += 8) {
        hex += parseInt(bits.slice(at, at + 8), 2)
            .toString(16)
            .padStart(2, "0")
    }
    return hex
}

/** The full Tracking ID a 32-byte digest in hex writes. */
export const trackingIdOfDigestHex = (hex: string): string | undefined => {
    if (!/^[0-9a-f]{64}$/i.test(hex)) return undefined
    let bits = ""
    for (let at = 0; at < hex.length; at += 2) {
        bits += parseInt(hex.slice(at, at + 2), 16)
            .toString(2)
            .padStart(8, "0")
    }
    bits = bits.padEnd(FULL_BALLOT_ID_LENGTH * 5, "0")
    let chars = ""
    for (let at = 0; at < bits.length; at += 5) {
        chars += CROCKFORD[parseInt(bits.slice(at, at + 5), 2)]
    }
    return groupBallotId(chars)
}
