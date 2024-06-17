// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {cloneDeep} from "lodash"
import {IElection} from "../types/CoreTypes"

export const sortElectionList = (elections: Array<IElection>): Array<IElection> => {
    console.log(`calling sortContestList`)
    elections = cloneDeep(elections)

    // Sort by alias or else by name
    elections.sort((a, b) => {
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

    return elections
}
