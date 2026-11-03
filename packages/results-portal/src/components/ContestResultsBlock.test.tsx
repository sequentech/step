// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {beforeEach, describe, expect, it, jest} from "@jest/globals"
import {renderToStaticMarkup} from "react-dom/server"
import type {ResultsManifestContest, ResultsSqliteDataset} from "@/types/results"
import {ContestResultsBlock} from "./ContestResultsBlock"

const mockResultsAndParticipation: Array<Record<string, unknown>> = []

// The ui-core modules the block uses, without the WASM bindings its index loads.
jest.mock("@sequentech/ui-core", () => ({
    ...jest.requireActual<object>("../../../ui-core/src/services/translate"),
    ...jest.requireActual<object>("../../../ui-core/src/services/presentationOrder"),
    ...jest.requireActual<object>("../../../ui-core/src/types/VotingChannel"),
    ...jest.requireActual<object>("../../../ui-core/src/types/CoreTypes"),
}))

jest.mock("@sequentech/ui-essentials", () => ({
    ResultsAndParticipation: (props: Record<string, unknown>) => {
        mockResultsAndParticipation.push(props)
        return null
    },
}))

// The count's type is part of the label: i18next only pluralises a numeric count.
jest.mock("react-i18next", () => ({
    useTranslation: () => ({
        t: (key: string, options?: {count?: unknown}) =>
            options?.count === undefined
                ? key
                : `${key}(${typeof options.count} ${String(options.count)})`,
    }),
}))

const dataset = (): ResultsSqliteDataset => ({
    election_event: [],
    election: [{id: "election", name: "Council election"}],
    contest: [{id: "contest", election_id: "election", name: "Council"}],
    candidate: [],
    area: [{id: "area", name: "North district"}],
    results_event: [],
    results_election: [],
    results_election_area: [],
    results_contest: [
        {
            id: "contest-results",
            election_id: "election",
            contest_id: "contest",
            total_votes: "10",
            annotations: JSON.stringify({extended_metrics: {weight: "2"}}),
        },
    ],
    results_contest_candidate: [],
    results_area_contest: [],
    results_area_contest_candidate: [],
})

// The manifest is fetched JSON, so its positions may not be a number.
const render = (
    contest: Omit<Partial<ResultsManifestContest>, "positions"> & {positions?: unknown}
) =>
    renderToStaticMarkup(
        <ContestResultsBlock
            manifestContest={
                {
                    election_id: "election",
                    contest_id: "contest",
                    publication_state: "published",
                    ...contest,
                } as ResultsManifestContest
            }
            dataset={dataset()}
            locale="en"
        />
    )

describe("ContestResultsBlock", () => {
    beforeEach(() => {
        mockResultsAndParticipation.length = 0
    })

    it("pluralises the positions of a manifest that sends them as text", () => {
        const markup = render({positions: "2"})

        expect(markup).toContain("Council election - resultsPortal.position(number 2)")
        expect(mockResultsAndParticipation).toHaveLength(1)
        expect(mockResultsAndParticipation[0]).toMatchObject({
            chartName: "Council election - Council",
            summary: {id: "contest-results", totalVotes: 10, weight: 2},
            preferential: false,
            acclaimed: false,
        })
    })

    it("leaves the positions out when there are none or they are not a number", () => {
        for (const positions of [0, null, undefined, "several"]) {
            const markup = render({positions})

            expect(markup).toContain("Council election")
            expect(markup).not.toContain("resultsPortal.position")
        }
    })

    it("names the area of an area contest and waits for its publication", () => {
        const markup = render({area_id: "area", positions: 1, publication_state: "not_published"})

        expect(markup).toContain(
            "Council election - resultsPortal.position(number 1) - North district"
        )
        expect(markup).toContain("seq-results-contest--not_published")
        expect(markup).toContain("resultsPortal.notPublishedYet")
        expect(mockResultsAndParticipation).toEqual([])
    })
})
