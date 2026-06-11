// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {ICandidate, IContest} from "../types/CoreTypes"
import {ITypePresentation} from "../types/ContestPresentation"
import type {IDecodedVoteChoice, IDecodedVoteContest} from "./wasm"
import {shuffle, splitList} from "../utils/array"
import {
    checkIsCategoryList,
    checkIsExplicitBlankVote,
    checkIsInvalidVote,
} from "./candidatePresentation"

export interface ICategory {
    header?: ICandidate
    candidates: ICandidate[]
}

export type CategoriesMap = Record<string, ICategory>

export interface ICategorizedCandidates {
    invalidOrBlankCandidates: ICandidate[]
    noCategoryCandidates: ICandidate[]
    categoriesMap: CategoriesMap
}

export const categorizeCandidates = (question: IContest): ICategorizedCandidates => {
    const enabledCandidates = question.candidates.filter(
        (candidate) => !(candidate.presentation?.is_disabled ?? false)
    )

    const isInvalidOrBlank = (candidate: ICandidate): boolean =>
        checkIsInvalidVote(candidate) || checkIsExplicitBlankVote(candidate)
    const [validCandidates, invalidOrBlankCandidates] = splitList(
        enabledCandidates,
        isInvalidOrBlank
    )
    const noCategoryCandidates: ICandidate[] = []
    const categoriesMap: CategoriesMap = {}

    for (const answer of validCandidates) {
        const category = answer.candidate_type
        if (!category) {
            noCategoryCandidates.push(answer)
            continue
        }
        if (!categoriesMap[category]) {
            categoriesMap[category] = {candidates: []}
        }
        if (checkIsCategoryList(answer)) {
            categoriesMap[category].header = answer
        } else {
            categoriesMap[category].candidates.push(answer)
        }
    }

    return {invalidOrBlankCandidates, noCategoryCandidates, categoriesMap}
}

export const getShuffledCategories = (
    categories: CategoriesMap,
    shuffleAllOptions: boolean,
    shuffleCategories: boolean,
    shuffleCategoryList: string[],
    typesPresentation?: Record<string, ITypePresentation>
): CategoriesMap => {
    const shuffledCategories: CategoriesMap = {}

    const categoryKeys = shuffleCategories
        ? shuffle(Object.keys(categories))
        : Object.keys(categories).sort((keyA, keyB) => {
              const orderA = typesPresentation?.[keyA]?.sort_order ?? 0
              const orderB = typesPresentation?.[keyB]?.sort_order ?? 0
              return orderA - orderB
          })

    for (const categoryKey of categoryKeys) {
        const category = categories[categoryKey]

        if (shuffleAllOptions || shuffleCategoryList.includes(categoryKey)) {
            category.candidates = shuffle(category.candidates)
        }

        shuffledCategories[categoryKey] = category
    }

    return shuffledCategories
}

export const sortCategoryEntries = (
    categoriesMap: CategoriesMap,
    typesPresentation?: Record<string, ITypePresentation>
): Array<[string, ICategory]> =>
    Object.entries(categoriesMap).sort(
        ([nameA], [nameB]) =>
            (typesPresentation?.[nameA]?.sort_order ?? 0) -
            (typesPresentation?.[nameB]?.sort_order ?? 0)
    )

const resolveChoice = (
    choices: IDecodedVoteChoice[] | Record<string, IDecodedVoteChoice>,
    candidateId: string
): IDecodedVoteChoice | undefined =>
    Array.isArray(choices)
        ? choices.find((choice) => choice.id === candidateId)
        : choices[candidateId]

export const isChoiceSelected = (
    choices: IDecodedVoteChoice[] | Record<string, IDecodedVoteChoice>,
    candidateId: string
): boolean => (resolveChoice(choices, candidateId)?.selected ?? -1) > -1

export const showCategoryOnReview = (
    category: ICategory,
    questionPlaintext?: IDecodedVoteContest
): boolean => {
    if (!questionPlaintext) {
        return false
    }

    const answersFromCategory = category.candidates.map((candidate) => candidate.id)
    if (category.header) {
        answersFromCategory.push(category.header.id)
    }

    return questionPlaintext.choices.some(
        (choice) => choice.selected > -1 && answersFromCategory.includes(choice.id)
    )
}

export const isCategoryListSelected = (
    category: ICategory,
    choices: IDecodedVoteChoice[] | Record<string, IDecodedVoteChoice>
): boolean => {
    if (!category.header) {
        return false
    }

    return isChoiceSelected(choices, category.header.id)
}

export const shouldShowCategoryCandidateOnReview = (
    category: ICategory,
    candidateId: string,
    choices: IDecodedVoteChoice[] | Record<string, IDecodedVoteChoice>
): boolean => isChoiceSelected(choices, candidateId) || isCategoryListSelected(category, choices)
