// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {normalizeTrackingId} from "@sequentech/ui-core"

const isHex = (text: string): boolean => text.length % 2 === 0 && /^[0-9a-fA-F]+$/.test(text)

// What a voter may type to find a ballot: the hash of the ballot, or its
// Tracking ID, or the start of one. A voter pseudonym (`FTBE:MHRX`) is neither.
// Nothing typed is not an error.
export const isBallotIdInput = (input: string): boolean =>
    input.trim() === "" || isHex(input) || normalizeTrackingId(input) !== null

// A Tracking ID as the screens write it; anything else as typed.
export const typedBallotId = (input: string): string => normalizeTrackingId(input) ?? input
