// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {normalizeBallotId} from "@sequentech/ui-core"

const isHex = (text: string): boolean => text.length % 2 === 0 && /^[0-9a-fA-F]+$/.test(text)

// What a voter may type to find a ballot: the hash of the ballot, or the
// Ballot ID the ballot box signed for it. Nothing typed is not an error.
export const isBallotIdInput = (input: string): boolean =>
    input.trim() === "" || isHex(input) || normalizeBallotId(input) !== null

// A ballot box's Ballot ID as the ballot box writes it; anything else as typed.
export const typedBallotId = (input: string): string => normalizeBallotId(input) ?? input
