// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {describe, expect, it, jest} from "@jest/globals"
import {ResultsManifest, ResultsSqliteDataset} from "@/types/results"

jest.mock("@sequentech/ui-core", () => ({
    parseEntityPresentation: (value: unknown) => {
        let parsed = value
        if (typeof value === "string") {
            try {
                parsed = JSON.parse(value)
            } catch {
                return undefined
            }
        }

        return typeof parsed === "object" && parsed !== null && !Array.isArray(parsed)
            ? parsed
            : undefined
    },
    sortByPresentationOrder: (
        items: unknown[],
        order: string | undefined,
        accessors: {
            getLabel: (item: unknown) => string
            getPresentation: (item: unknown) => {sort_order?: number} | undefined
        }
    ) => {
        const sorted = [...items]
        if (order === "random") {
            return sorted
        }
        if (order === "custom") {
            return sorted.sort(
                (left, right) =>
                    (accessors.getPresentation(left)?.sort_order ?? -1) -
                    (accessors.getPresentation(right)?.sort_order ?? -1)
            )
        }
        return sorted.sort((left, right) =>
            accessors.getLabel(left).localeCompare(accessors.getLabel(right))
        )
    },
}))

jest.mock("./resultLabels", () => ({
    translatedLabel: (
        row: {presentation?: {i18n?: {en?: {name?: string}}}} | undefined,
        _locale: string,
        fallback = "-"
    ) => row?.presentation?.i18n?.en?.name ?? fallback,
}))

import {
    orderElectionResultRows,
    orderResultCandidates,
    orderResultContestIds,
    orderResultElectionIds,
} from "./resultsOrdering"

const manifest: ResultsManifest = {
    schema_version: 1,
    tenant_id: "tenant",
    election_event_id: "event",
    election_ids: ["election-a", "election-z"],
    route_scope: "event",
    publication_id: "publication",
    results_event_id: "results",
    version: 1,
    access: "public",
    visibility_scope: "full_event",
    contests: [
        {
            election_id: "election-z",
            contest_id: "contest-a",
            publication_state: "published",
        },
        {
            election_id: "election-z",
            contest_id: "contest-z",
            publication_state: "published",
        },
    ],
    artifacts: {},
}

const dataset: ResultsSqliteDataset = {
    election_event: [{id: "event", presentation: JSON.stringify({elections_order: "custom"})}],
    election: [
        {
            id: "election-a",
            presentation: {sort_order: 2, i18n: {en: {name: "A election"}}},
        },
        {
            id: "election-z",
            presentation: {
                sort_order: 1,
                contests_order: "custom",
                i18n: {en: {name: "Z election"}},
            },
        },
    ],
    contest: [
        {
            id: "contest-a",
            presentation: {sort_order: 2, i18n: {en: {name: "A contest"}}},
        },
        {
            id: "contest-z",
            presentation: {
                sort_order: 1,
                candidates_order: "custom",
                i18n: {en: {name: "Z contest"}},
            },
        },
    ],
    candidate: [
        {
            id: "candidate-a",
            contest_id: "contest-z",
            presentation: {sort_order: 2, i18n: {en: {name: "A candidate"}}},
        },
        {
            id: "candidate-z",
            contest_id: "contest-z",
            presentation: {sort_order: 1, i18n: {en: {name: "Z candidate"}}},
        },
    ],
    area: [],
    results_event: [],
    results_election: [],
    results_election_area: [],
    results_contest: [],
    results_contest_candidate: [],
    results_area_contest: [],
    results_area_contest_candidate: [],
}

describe("results ordering", () => {
    it("applies custom election, contest, and candidate order", () => {
        expect(orderResultElectionIds(manifest, dataset, "en")).toEqual([
            "election-z",
            "election-a",
        ])
        expect(orderResultContestIds(manifest, dataset, "election-z", "en")).toEqual([
            "contest-z",
            "contest-a",
        ])
        expect(orderResultCandidates(dataset, "contest-z", "en").map((row) => row.id)).toEqual([
            "candidate-z",
            "candidate-a",
        ])
    })

    it("orders election summary rows by their election configuration", () => {
        const rows = [
            {id: "result-a", election_id: "election-a"},
            {id: "result-z", election_id: "election-z"},
        ]

        expect(
            orderElectionResultRows(rows, manifest, dataset, "en").map((row) => row.election_id)
        ).toEqual(["election-z", "election-a"])
    })

    it("falls back safely when election-event presentation is malformed", () => {
        const malformedDataset = {
            ...dataset,
            election_event: [{id: "event", presentation: "{invalid"}],
        }
        const reversedManifest = {
            ...manifest,
            election_ids: ["election-z", "election-a"],
        }

        expect(orderResultElectionIds(reversedManifest, malformedDataset, "en")).toEqual([
            "election-a",
            "election-z",
        ])
    })

    it("preserves the published source order for random elections", () => {
        const randomDataset = {
            ...dataset,
            election_event: [
                {id: "event", presentation: JSON.stringify({elections_order: "random"})},
            ],
        }
        const reversedManifest = {
            ...manifest,
            election_ids: ["election-z", "election-a"],
        }

        expect(orderResultElectionIds(reversedManifest, randomDataset, "en")).toEqual([
            "election-z",
            "election-a",
        ])

        const summaryRows = [
            {id: "result-a", election_id: "election-a"},
            {id: "result-z", election_id: "election-z"},
        ]
        expect(
            orderElectionResultRows(summaryRows, reversedManifest, randomDataset, "en").map(
                (row) => row.election_id
            )
        ).toEqual(["election-z", "election-a"])
    })
})
