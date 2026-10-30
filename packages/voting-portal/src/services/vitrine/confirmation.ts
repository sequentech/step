// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {shortBallotId} from "./ballotId"

/** A file name for the receipt the voter saves, from the short Ballot ID:
 * `ballot-30ff-d314.vitrine-receipt`. */
export const receiptFileName = (ballotId: string): string => {
    const short = shortBallotId(ballotId).replace(/ /g, "-")
    return `ballot${short ? `-${short}` : ""}.vitrine-receipt`
}
