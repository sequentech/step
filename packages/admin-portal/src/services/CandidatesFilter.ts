// SPDX-FileCopyrightText: 2024 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {Sequent_Backend_Candidate} from "@/gql/graphql"
import {EEnableCheckableLists, ICandidatePresentation} from "@sequentech/ui-core"

export const isCandidateACheckableList = (candidate: Sequent_Backend_Candidate): boolean => {
    let candidatePresentation = candidate.presentation as ICandidatePresentation | undefined

    return candidatePresentation?.is_category_list ?? false
}

export const filterCandidateByCheckableLists = (
    candidate: Sequent_Backend_Candidate,
    contestCheckableLists: EEnableCheckableLists
): boolean => {
    if (isCandidateACheckableList(candidate)) {
        return [
            EEnableCheckableLists.CANDIDATES_AND_LISTS,
            EEnableCheckableLists.LISTS_ONLY,
        ].includes(contestCheckableLists)
    } else {
        return [
            EEnableCheckableLists.CANDIDATES_AND_LISTS,
            EEnableCheckableLists.CANDIDATES_ONLY,
        ].includes(contestCheckableLists)
    }
}
