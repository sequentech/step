// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {IAnswer, IQuestion} from "sequent-core"
import {shuffle, splitList} from "@sequentech/ui-essentials"
import {checkIsInvalidVote} from "./ElectionConfigService"

export interface ICategory {
    header?: IAnswer
    candidates: Array<IAnswer>
}

export type CategoriesMap = {[category: string]: ICategory}

export interface ICategorizedCandidates {
    invalidCandidates: Array<IAnswer>
    noCategoryCandidates: Array<IAnswer>
    categoriesMap: CategoriesMap
}

export const categorizeCandidates = (question: IQuestion): ICategorizedCandidates => {
    const [validCandidates, invalidCandidates] = splitList(question.answers, checkIsInvalidVote)
    const nonCategoryCandidates: Array<IAnswer> = []

    const categoriesMap: CategoriesMap = {}
    for (let answer of validCandidates) {
        let category = answer.category
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
        const isCategoryHeader = answer.urls.some(
            (url) => "isCategoryList" === url.title && "true" === url.url
        )
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

export const getShuffledCategories = (
    categories: CategoriesMap,
    shuffleAllOptions: boolean,
    shuffleCategories: boolean,
    shuffleCategoryList: Array<string>
): CategoriesMap => {
    const shuffledCategories: CategoriesMap = {}

    let categoryKeys = shuffleCategories
        ? shuffle(Object.keys(categories))
        : Object.keys(categories)
    for (let categoryKey of categoryKeys) {
        let category = categories[categoryKey]

        if (shuffleAllOptions || shuffleCategoryList.includes(categoryKey)) {
            category.candidates = shuffle(category.candidates)
        }

        shuffledCategories[categoryKey] = category
    }

    return shuffledCategories
}
