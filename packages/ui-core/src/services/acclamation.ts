// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {IContest} from "../types/CoreTypes"

/**
 * An acclaimed contest is displayed to the voter but never encoded into the
 * ballot: its options cannot be selected and it records no votes.
 */
export const isAcclaimedContest = (contest?: IContest | null): boolean => !!contest?.is_acclaimed

/**
 * True when every contest a voter is entitled to vote on is acclaimed, in
 * which case there is nothing to encode and no ballot to cast.
 */
export const areAllContestsAcclaimed = (contests?: Array<IContest> | null): boolean =>
    !!contests?.length && contests.every(isAcclaimedContest)
