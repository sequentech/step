// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Tracking IDs as voters read them: Crockford base32 in groups of four joined
// by hyphens, `FTBE-MHRX-...`. The short ID is the first two groups.
// A voter pseudonym, `FTBE:MHRX`, is never a Tracking ID.

export const BALLOT_ID_GROUP = 4
export const SHORT_BALLOT_ID_LENGTH = 8
export const FULL_BALLOT_ID_LENGTH = 52
export const BALLOT_ID_SEPARATOR = "-"
/** Groups per line of a full ID: two lines on wide screens, four on phones. */
export const WIDE_GROUPS_PER_LINE = 8
export const NARROW_GROUPS_PER_LINE = 4

/** What a voter typed or pasted, as a Tracking ID. */
export enum EBallotIdInput {
    EMPTY = "empty",
    SHORT = "short",
    /** More than the short ID, less than the full one: the start of an ID. */
    PARTIAL = "partial",
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

/** The full ID's groups, `perLine` to a line. */
export const ballotIdLines = (value: string, perLine: number): string[][] => {
    const groups = ballotIdGroups(value)
    const lines: string[][] = []
    for (let start = 0; start < groups.length; start += Math.max(1, perLine)) {
        lines.push(groups.slice(start, start + Math.max(1, perLine)))
    }
    return lines
}

/** Whether the input is a short ID, the start of one, a full one, or none. */
export const ballotIdInput = (value: string): EBallotIdInput => {
    const chars = ballotIdChars(value)
    if (!chars) return EBallotIdInput.EMPTY
    if (!ALPHABET.test(chars)) return EBallotIdInput.INVALID
    if (chars.length === SHORT_BALLOT_ID_LENGTH) return EBallotIdInput.SHORT
    if (chars.length === FULL_BALLOT_ID_LENGTH) return EBallotIdInput.FULL
    if (chars.length > SHORT_BALLOT_ID_LENGTH && chars.length < FULL_BALLOT_ID_LENGTH) {
        return EBallotIdInput.PARTIAL
    }
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
