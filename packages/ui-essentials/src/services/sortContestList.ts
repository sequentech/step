// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {cloneDeep} from "lodash"
import {IContest} from "../types/CoreTypes"
import {ContestsOrder} from ".."
import {shuffle} from "moderndash"

export const sortContestList = (
    contests: Array<IContest>,
    order?: ContestsOrder,
    applyRandom?: boolean
): Array<IContest> => {
    let res = cloneDeep(contests)

    switch (order) {
        case ContestsOrder.ALPHABETICAL:
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
        case ContestsOrder.CUSTOM:
            res.sort(
                (a, b) => (a.presentation?.sort_order ?? -1) - (b.presentation?.sort_order ?? -1)
            )
            break
        case ContestsOrder.RANDOM:
            if (applyRandom) {
                res = shuffle(res)
            }
            break
    }

    return res
}
