import {CandidatesOrder, ICandidate} from "@root/types/CoreTypes"
import {cloneDeep} from "lodash"

export function sortCandidatesInContest(
    candidates: ICandidate[],
    order?: CandidatesOrder,
    applyRandom?: boolean
): ICandidate[] {
    const res = cloneDeep(candidates)

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
                for (let i = res.length - 1; i > 0; i--) {
                    const j = Math.floor(Math.random() * (i + 1))
                    ;[res[i], res[j]] = [res[j], res[i]]
                }
            }
            break
    }

    return res
}
