// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {cloneDeep} from "lodash"
import {IContest} from "../types/CoreTypes"

export const sortContestByCreationDate = (contests: Array<IContest>): Array<IContest> => {
    contests = cloneDeep(contests)

    contests.sort((a, b) => {
        const dateA = a.created_at ? new Date(a.created_at) : new Date(0)
        const dateB = b.created_at ? new Date(b.created_at) : new Date(0)

        return dateA.getTime() - dateB.getTime()
    })

    return contests
}
