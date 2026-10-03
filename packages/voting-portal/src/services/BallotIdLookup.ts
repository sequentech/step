// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {normalizeBallotId} from "@sequentech/ui-core"

// Telephone voters are read the first characters of their ballot's hash.
const TELEPHONE_BALLOT_ID_PREFIX_LENGTH = 4

const isHex = (text: string): boolean => text.length % 2 === 0 && /^[0-9a-fA-F]+$/.test(text)

// What a voter may type to find a ballot: the hash of the ballot, or the
// Ballot ID the ballot box signed for it. Nothing typed is not an error.
export const isBallotIdInput = (input: string): boolean =>
    input.trim() === "" || isHex(input) || normalizeBallotId(input) !== null

// A ballot box's Ballot ID as the ballot box writes it; anything else as typed.
export const typedBallotId = (input: string): string => normalizeBallotId(input) ?? input

// The pattern a cast vote's ballot ID must match. Empty when the text cannot
// name a ballot.
export const ballotIdLookupPattern = (
    ballotId: string | undefined,
    telephoneVotingEnabled: boolean
): string => {
    const receivedBallotId = normalizeBallotId(ballotId ?? "")
    if (receivedBallotId) {
        return receivedBallotId
    }
    const hash = ballotId?.toLowerCase() ?? ""
    if (!/^[0-9a-f]+$/.test(hash)) {
        return ""
    }
    return telephoneVotingEnabled && hash.length === TELEPHONE_BALLOT_ID_PREFIX_LENGTH
        ? `${hash}%`
        : hash
}
