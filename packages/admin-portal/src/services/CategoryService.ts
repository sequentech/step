// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {shuffle, splitList, ICandidate, IContest} from "@sequentech/ui-core"
import {checkIsCategoryList, checkIsInvalidVote} from "./ElectionConfigService"

export interface ICategory {
    header?: ICandidate
    candidates: Array<ICandidate>
}

export type CategoriesMap = {[category: string]: ICategory}

export interface ICategorizedCandidates {
    invalidCandidates: Array<ICandidate>
    noCategoryCandidates: Array<ICandidate>
    categoriesMap: CategoriesMap
}

export const categorizeCandidates = (question: IContest): ICategorizedCandidates => {
    const [validCandidates, invalidCandidates] = splitList(question.candidates, checkIsInvalidVote)
    const nonCategoryCandidates: Array<ICandidate> = []

    const categoriesMap: CategoriesMap = {}
    for (let answer of validCandidates) {
        let category = answer.candidate_type
        if (!category) {
            nonCategoryCandidates.push(answer)
            continue
        }
        if (!categoriesMap[category]) {
            // initialize category
            categoriesMap[category] = {
                candidates: [],
            }
        }
        const isCategoryHeader = checkIsCategoryList(answer)

        if (isCategoryHeader) {
            categoriesMap[category].header = answer
        } else {
            categoriesMap[category].candidates.push(answer)
        }
    }

    return {
        invalidCandidates: invalidCandidates,
        noCategoryCandidates: nonCategoryCandidates,
        categoriesMap: categoriesMap,
    }
}
