// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {ICandidate} from "../types/CoreTypes"
import {CandidatesOrder} from "../types/ContestPresentation"
import {cloneDeep} from "lodash"
import {shuffle} from "moderndash"

export const sortCandidatesInContest = (
    candidates: Array<ICandidate>,
    order?: CandidatesOrder,
    applyRandom?: boolean
): Array<ICandidate> => {
    let res = cloneDeep(candidates)

    switch (order) {
        case CandidatesOrder.ALPHABETICAL:
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
        case CandidatesOrder.CUSTOM:
            res.sort(
                (a, b) => (a.presentation?.sort_order ?? -1) - (b.presentation?.sort_order ?? -1)
            )
            break
        case CandidatesOrder.RANDOM:
            if (applyRandom) {
                res = shuffle(res)
            }
            break
    }

    return res
}
