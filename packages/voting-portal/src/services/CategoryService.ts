// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {ICandidate, IContest, ITypePresentation, shuffle, splitList} from "@sequentech/ui-core"
import {
    checkIsCategoryList,
    checkIsExplicitBlankVote,
    checkIsInvalidVote,
} from "./ElectionConfigService"
import {sortBy} from "lodash"

export interface ICategory {
    header?: ICandidate
    candidates: Array<ICandidate>
}

export type CategoriesMap = {[category: string]: ICategory}

export interface ICategorizedCandidates {
    invalidOrBlankCandidates: Array<ICandidate>
    noCategoryCandidates: Array<ICandidate>
    categoriesMap: CategoriesMap
}

export const categorizeCandidates = (question: IContest): ICategorizedCandidates => {
    const enabledCandidates = question.candidates.filter(
        (cand: ICandidate) => !(cand.presentation?.is_disabled ?? false)
    )

    const isInvalidOrBlank = (candidate: ICandidate): boolean =>
        checkIsInvalidVote(candidate) || checkIsExplicitBlankVote(candidate)
    const [validCandidates, invalidOrBlankCandidates] = splitList(
        enabledCandidates,
        isInvalidOrBlank
    )
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
        invalidOrBlankCandidates: invalidOrBlankCandidates,
        noCategoryCandidates: nonCategoryCandidates,
        categoriesMap: categoriesMap,
    }
}

export const getShuffledCategories = (
    categories: CategoriesMap,
    shuffleAllOptions: boolean,
    shuffleCategories: boolean,
    shuffleCategoryList: Array<string>,
    types_presentation?: Record<string, ITypePresentation>
): CategoriesMap => {
    const shuffledCategories: CategoriesMap = {}

    let categoryKeys = shuffleCategories
        ? shuffle(Object.keys(categories))
        : sortBy(
              Object.keys(categories).map((key) => ({
                  key,
                  sort_order: types_presentation?.[key]?.sort_order ?? 0,
              })),
              "sort_order"
          ).map((value) => value.key)
    for (let categoryKey of categoryKeys) {
        let category = categories[categoryKey]

        if (shuffleAllOptions || shuffleCategoryList.includes(categoryKey)) {
            category.candidates = shuffle(category.candidates)
        }

        shuffledCategories[categoryKey] = category
    }

    return shuffledCategories
}
