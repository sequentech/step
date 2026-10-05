// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {createElement} from "react"
import {renderToStaticMarkup} from "react-dom/server"
import {ResultsManifest, ResultsSerializedJson, ResultsSqliteDataset} from "@/types/results"
import {describe, expect, it, jest} from "@jest/globals"
import {buildAreaElectionSummaries} from "@/services/areaSummaries"

jest.mock("react-i18next", () => ({
    useTranslation: () => ({
        t: (key: string) => key,
        i18n: {language: "en", resolvedLanguage: "en"},
    }),
}))

jest.mock("react-apexcharts", () => ({
    __esModule: true,
    default: () => null,
}))

jest.mock("@sequentech/ui-core", () => ({
    ...jest.requireActual<typeof import("@sequentech/ui-core")>(
        "../../../ui-core/src/types/VotingChannel"
    ),
    ...jest.requireActual<typeof import("@sequentech/ui-core")>(
        "../../../ui-core/src/types/ElectionEventPresentation"
    ),
    ...jest.requireActual<typeof import("@sequentech/ui-core")>(
        "../../../ui-core/src/services/numberFormat"
    ),
    ...jest.requireActual<typeof import("@sequentech/ui-core")>(
        "../../../ui-core/src/services/NumberFormatContext"
    ),
    ...jest.requireActual<typeof import("@sequentech/ui-core")>(
        "../../../ui-core/src/services/percentFormatter"
    ),
    ...jest.requireActual<typeof import("@sequentech/ui-core")>(
        "../../../ui-core/src/services/presentationOrder"
    ),
    ...jest.requireActual<typeof import("@sequentech/ui-core")>(
        "../../../ui-core/src/utils/typechecks"
    ),
}))

jest.mock("@sequentech/ui-essentials", () => ({
    TALLY_RESULTS_PIE_HEIGHT: 170,
    TALLY_RESULTS_PIE_PANEL_WIDTH: 360,
    pieChartNumberFormatOptions: jest.requireActual<typeof import("@sequentech/ui-essentials")>(
        "../../../ui-essentials/src/components/TallyResults/utils"
    ).pieChartNumberFormatOptions,
}))

jest.mock("@/services/resultLabels", () => ({
    manifestTitle: (_title: unknown, _locale: string, fallback: string) => fallback,
    translatedLabel: (_row: unknown, _locale: string, fallback = "-") => fallback,
}))

jest.mock("./ResultsSelectorTabs", () => ({
    ResultsSelectorTabs: () => null,
}))

import {ResultsPageContent} from "./ResultsPageContent"

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

describe("ResultsPageContent", () => {
    const manifest: ResultsManifest = {
        schema_version: 1,
        tenant_id: "tenant-1",
        election_event_id: "event-1",
        election_ids: ["election-1"],
        route_scope: "event",
        publication_id: "publication-1",
        results_event_id: "results-event-1",
        version: 1,
        access: "public",
        visibility_scope: "full_event",
        contests: [],
        artifacts: {},
    }

    const publishedDataset = (eventPresentation: ResultsSerializedJson): ResultsSqliteDataset => ({
        ...dataset([]),
        election_event: [{id: "event-1", presentation: eventPresentation}],
        election: [{id: "election-1"}],
        results_election: [
            {
                id: "result-1",
                election_id: "election-1",
                elegible_census: 12000000,
                total_voters: 8589934,
                total_voters_percent: 0.715827833,
            },
        ],
    })

    const cellText = (markup: string, cell: string): string | undefined =>
        markup.match(new RegExp(`seq-results-summary__${cell}-cell[^>]*>([^<]*)<`))?.[1]

    it("writes figures in the number format the published results carry", () => {
        const markup = renderToStaticMarkup(
            createElement(ResultsPageContent, {
                manifest,
                dataset: publishedDataset(
                    JSON.stringify({css: ".results {}", number_format_policy: "period-comma"})
                ),
            })
        )

        expect(cellText(markup, "eligible")).toBe("12.000.000")
        expect(cellText(markup, "counted")).toBe("8.589.934")
        expect(cellText(markup, "participation")).toBe("71,58%")
    })

    it.each([
        ["published before the number format existed", JSON.stringify({css: ".results {}"})],
        ["without an event presentation", null],
        [
            "with a number format this version does not know",
            JSON.stringify({number_format_policy: "future-format"}),
        ],
    ])("uses comma grouping for results %s", (_case, eventPresentation) => {
        const markup = renderToStaticMarkup(
            createElement(ResultsPageContent, {
                manifest,
                dataset: publishedDataset(eventPresentation),
            })
        )

        expect(cellText(markup, "eligible")).toBe("12,000,000")
        expect(cellText(markup, "counted")).toBe("8,589,934")
        expect(cellText(markup, "participation")).toBe("71.58%")
    })
})
