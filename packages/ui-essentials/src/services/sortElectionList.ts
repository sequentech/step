// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {cloneDeep} from "lodash"
import {IElection} from "../types/CoreTypes"
import {ElectionsOrder} from ".."
import {shuffle} from "moderndash"

export const sortElectionList = (
    elections: Array<IElection>,
    order?: ElectionsOrder,
    applyRandom?: boolean
): Array<IElection> => {
    let res = cloneDeep(elections)

    switch (order) {
        case ElectionsOrder.ALPHABETICAL:
            res.sort((a, b) => {
                const nameA =
                    (a.alias ? a.alias?.toLowerCase() : null) ??
                    (a.name ? a.name?.toLowerCase() : null) ??
                    ""
                const nameB =
                    (b.alias ? b.alias?.toLowerCase() : null) ??
                    (b.name ? b.name?.toLowerCase() : null) ??
                    ""

                if (nameA < nameB) {
                    return -1
                }
                if (nameA > nameB) {
                    return 1
                }

                return 0
            })
            break
        case ElectionsOrder.CUSTOM:
            res.sort(
                (a, b) => (a.presentation?.sort_order ?? -1) - (b.presentation?.sort_order ?? -1)
            )
            break
        case ElectionsOrder.RANDOM:
            if (applyRandom) {
                res = shuffle(res)
            }
            break
    }

    return res
}
