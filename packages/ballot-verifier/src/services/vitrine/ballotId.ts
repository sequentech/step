// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Tracking IDs as voters read them: lowercase hex in groups of four. The short
// ID is the first two groups. The verifier compares the whole ID: a short ID
// has 32 bits, which a birthday search could match.

export const BALLOT_ID_GROUP = 4
export const SHORT_BALLOT_ID_LENGTH = 8
export const FULL_BALLOT_ID_LENGTH = 64
/** The groups that make the short ID, which the screens set in bold. */
export const SHORT_BALLOT_ID_GROUPS = SHORT_BALLOT_ID_LENGTH / BALLOT_ID_GROUP

/** What a voter typed or pasted, as a tracking ID. */
export enum EBallotIdInput {
    EMPTY = "empty",
    SHORT = "short",
    FULL = "full",
    INVALID = "invalid",
}

const HEX = /^[0-9a-f]+$/
const GROUPS = new RegExp(`.{1,${BALLOT_ID_GROUP}}`, "g")

/** The ID's hex digits: spaces and case don't count. */
export const ballotIdHex = (value: string): string => value.replace(/\s+/g, "").toLowerCase()

/** The ID's groups of four. */
export const ballotIdGroups = (value: string): string[] => ballotIdHex(value).match(GROUPS) ?? []

/** The ID in groups of four: `30ff d314 4238 …`. */
export const groupBallotId = (value: string): string => ballotIdGroups(value).join(" ")

/** The short ID: the first two groups, `30ff d314`. */
export const shortBallotId = (value: string): string =>
    groupBallotId(ballotIdHex(value).slice(0, SHORT_BALLOT_ID_LENGTH))

/** Whether the input is a short ID, a full one, or neither. */
export const ballotIdInput = (value: string): EBallotIdInput => {
    const hex = ballotIdHex(value)
    if (!hex) return EBallotIdInput.EMPTY
    if (!HEX.test(hex)) return EBallotIdInput.INVALID
    if (hex.length === SHORT_BALLOT_ID_LENGTH) return EBallotIdInput.SHORT
    if (hex.length === FULL_BALLOT_ID_LENGTH) return EBallotIdInput.FULL
    return EBallotIdInput.INVALID
}

/** What an ID field shows as the voter types: hex digits only, grouped, up to
 * a full ID. */
export const typedBallotId = (value: string): string =>
    groupBallotId(value.replace(/[^0-9a-fA-F]/g, "").slice(0, FULL_BALLOT_ID_LENGTH))

/** Whether two IDs are the same, whatever their spacing and case. */
export const sameBallotId = (a: string, b: string): boolean => ballotIdHex(a) === ballotIdHex(b)
