import {CandidatesOrder, ICandidate} from "@root/types/CoreTypes"
import {cloneDeep} from "lodash"
import {shuffle} from "moderndash"

export function sortCandidatesInContest(
    candidates: ICandidate[],
    order?: CandidatesOrder,
    applyRandom?: boolean
): ICandidate[] {
    let res = cloneDeep(candidates)

    switch (order) {
        case CandidatesOrder.ALPHABETICAL:
            res.sort((a, b) => {
                const nameA = a.alias?.toLowerCase() ?? a.name?.toLowerCase() ?? ""
                const nameB = b.alias?.toLowerCase() ?? b.name?.toLowerCase() ?? ""

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
            res.sort((a, b) => a.presentation?.sort_order! - b.presentation?.sort_order!)
            break
        case CandidatesOrder.RANDOM:
            if (applyRandom) {
                res = shuffle(res)
            }
            break
    }

    return res
}
