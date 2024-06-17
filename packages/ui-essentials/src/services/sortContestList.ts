// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {cloneDeep} from "lodash"
import {IContest} from "../types/CoreTypes"

export const sortContestList = (contests: Array<IContest>): Array<IContest> => {
    console.log(`calling sortContestList`)
    contests = cloneDeep(contests)

    // For later: sort by creation date:
    //contests.sort((a, b) => {
    //    const dateA = a.created_at ? new Date(a.created_at) : new Date(0)
    //    const dateB = b.created_at ? new Date(b.created_at) : new Date(0)
    //
    //    return dateA.getTime() - dateB.getTime()
    //})

    // Sort by alias or else by name
    contests.sort((a, b) => {
        const nameA =
            (a.alias ? a.alias?.toLowerCase() : null) ??
            (a.name ? a.name?.toLowerCase() : null) ??
            ""
        const nameB =
            (b.alias ? b.alias?.toLowerCase() : null) ??
            (b.name ? b.name?.toLowerCase() : null) ??
            ""
        console.log(`comparing ${nameA} vs ${nameB}`)

        if (nameA < nameB) {
            return -1
        }
        if (nameA > nameB) {
            return 1
        }

        return 0
    })

    return contests
}
