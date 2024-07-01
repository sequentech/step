// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import { CandidatesOrder, ContestsOrder, ElectionsOrder, ICandidate, IContest, IElection } from "@sequentech/ui-essentials"
import {sort_elections_list_js, sort_contests_list_js, sort_candidates_list_js} from "sequent-core"


export const sortElectionList = (
    elections: Array<IElection>,
    order?: ElectionsOrder,
    applyRandom?: boolean
): Array<IElection> => {
    try {
        if(!elections || !elections.length) return elections
        return sort_elections_list_js(elections, order, applyRandom)
    } catch (error) {
        console.log(error)
        throw error
    }
}

export const sortContestList = (
    contests: Array<IContest>,
    order?: ContestsOrder,
    applyRandom?: boolean
): Array<IContest> => {
    try {
        if(!contests || !contests.length) return contests
        return sort_contests_list_js(contests, order, applyRandom)
    } catch (error) {
        console.log(error)
        throw error
    }
}

export const sortCandidatesInContest = (
    candidates: Array<ICandidate>,
    order?: CandidatesOrder,
    applyRandom?: boolean
): Array<ICandidate> => {
    try {
        if(!candidates || !candidates.length) return candidates
        return sort_candidates_list_js(candidates, order, applyRandom)
    } catch (error) {
        console.log(error)
        throw error
    }
}
