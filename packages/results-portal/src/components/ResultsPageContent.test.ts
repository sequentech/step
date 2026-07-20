// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {ResultsSqliteDataset} from "@/types/results"
import {describe, expect, it} from "@jest/globals"
import {buildAreaElectionSummaries} from "@/services/areaSummaries"

const dataset = (
    resultsAreaContest: ResultsSqliteDataset["results_area_contest"]
): ResultsSqliteDataset => ({
    election_event: [],
    election: [],
    contest: [],
    candidate: [],
    area: [],
    results_event: [],
    results_election: [],
    results_election_area: [
        {
            id: "result-area-1",
            election_id: "election-1",
            area_id: "area-1",
            name: "Area 1",
        },
    ],
    results_contest: [],
    results_contest_candidate: [],
    results_area_contest: resultsAreaContest,
    results_area_contest_candidate: [],
})

describe("buildAreaElectionSummaries", () => {
    it("derives area turnout from scoped contest results", () => {
        const summaries = buildAreaElectionSummaries(
            dataset([
                {
                    election_id: "election-1",
                    area_id: "area-1",
                    elegible_census: 10,
                    total_votes: 4,
                },
                {
                    election_id: "election-1",
                    area_id: "area-1",
                    elegible_census: 10,
                    total_votes: 3,
                },
                {
                    election_id: "election-1",
                    area_id: "another-area",
                    elegible_census: 99,
                    total_votes: 99,
                },
            ])
        )

        expect(summaries[0]).toMatchObject({
            election_id: "election-1",
            area_id: "area-1",
            elegible_census: 10,
            total_voters: 4,
            total_voters_percent: 0.4,
        })
    })

    it("keeps zero totals for an empty area tally so the chart renders non-voters", () => {
        const summaries = buildAreaElectionSummaries(
            dataset([
                {
                    election_id: "election-1",
                    area_id: "area-1",
                    elegible_census: 0,
                    total_votes: 0,
                },
            ])
        )

        expect(summaries[0]).toMatchObject({
            elegible_census: 0,
            total_voters: 0,
            total_voters_percent: 0,
        })
    })

    it("does not combine the census and vote count from different contests", () => {
        const summaries = buildAreaElectionSummaries(
            dataset([
                {
                    election_id: "election-1",
                    area_id: "area-1",
                    elegible_census: 10,
                    total_votes: 4,
                },
                {
                    election_id: "election-1",
                    area_id: "area-1",
                    elegible_census: 5,
                    total_votes: 5,
                },
            ])
        )

        expect(summaries[0]).toMatchObject({
            elegible_census: 10,
            total_voters: 4,
            total_voters_percent: 0.4,
        })
    })
})
