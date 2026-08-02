// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {ResultsRow, ResultsSqliteDataset} from "@/types/results"

const numericValue = (value: unknown): number => {
    const numberValue = typeof value === "number" ? value : Number(value)
    return Number.isFinite(numberValue) ? numberValue : 0
}

export const buildAreaElectionSummaries = (dataset: ResultsSqliteDataset): ResultsRow[] =>
    dataset.results_election_area.map((electionArea) => {
        const contestResults = dataset.results_area_contest.filter(
            (contest) =>
                String(contest.election_id) === String(electionArea.election_id) &&
                String(contest.area_id) === String(electionArea.area_id)
        )
        const representativeContest = contestResults.reduce<ResultsRow | undefined>(
            (selected, result) => {
                if (!selected) return result

                const selectedCensus = numericValue(selected.elegible_census)
                const resultCensus = numericValue(result.elegible_census)
                if (resultCensus !== selectedCensus) {
                    return resultCensus > selectedCensus ? result : selected
                }

                return numericValue(result.total_votes) > numericValue(selected.total_votes)
                    ? result
                    : selected
            },
            undefined
        )
        const eligibleCensus = numericValue(representativeContest?.elegible_census)
        const totalVoters = numericValue(representativeContest?.total_votes)

        return {
            ...electionArea,
            elegible_census: eligibleCensus,
            total_voters: totalVoters,
            total_voters_percent: eligibleCensus > 0 ? totalVoters / eligibleCensus : 0,
        }
    })
