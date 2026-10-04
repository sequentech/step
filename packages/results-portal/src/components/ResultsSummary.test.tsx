// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {describe, expect, it, jest} from "@jest/globals"
import {renderToStaticMarkup} from "react-dom/server"

jest.mock("react-apexcharts", () => {
    const react = jest.requireActual<typeof import("react")>("react")

    return {
        __esModule: true,
        // Labels every slice and tooltip the way the chart library would, through
        // the formatters the component passes.
        default: ({
            className,
            height,
            options,
            series = [],
        }: {
            className?: string
            height?: number | string
            options?: {
                labels?: string[]
                dataLabels?: {formatter?: (percentage: number) => string}
                tooltip?: {y?: {formatter?: (value: number) => string}}
            }
            series?: number[]
        }) => {
            const total = series.reduce((sum, value) => sum + value, 0)

            return react.createElement("div", {
                className,
                "data-height": height,
                "data-labels": JSON.stringify(options?.labels ?? []),
                "data-series": JSON.stringify(series),
                "data-slice-labels": series
                    .map((value) => options?.dataLabels?.formatter?.((value / total) * 100))
                    .join("|"),
                "data-tooltip-labels": series
                    .map((value) => options?.tooltip?.y?.formatter?.(value))
                    .join("|"),
            })
        },
    }
})

jest.mock("react-i18next", () => ({
    useTranslation: () => ({
        t: (key: string) =>
            ({
                "resultsPortal.summary.title": "General information",
                "resultsPortal.summary.ariaLabel": "General results information",
                "resultsPortal.summary.election": "Election",
                "resultsPortal.summary.eligibleVoters": "Eligible voters",
                "resultsPortal.summary.totalVotesCounted": "Total votes counted",
                "resultsPortal.summary.participation": "Participation",
                "resultsPortal.summary.totalBlankBallots": "Total blank ballots",
                "resultsPortal.resultsAndParticipation.nonVoters": "Non voters",
            })[key] ?? key,
    }),
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
    isNumber: (value: unknown) => typeof value === "number" && Number.isFinite(value),
}))

jest.mock("@sequentech/ui-essentials", () => ({
    TALLY_RESULTS_PIE_HEIGHT: 170,
    TALLY_RESULTS_PIE_PANEL_WIDTH: 360,
    pieChartNumberFormatOptions: jest.requireActual<typeof import("@sequentech/ui-essentials")>(
        "../../../ui-essentials/src/components/TallyResults/utils"
    ).pieChartNumberFormatOptions,
}))

jest.mock("@/services/resultLabels", () => ({
    translatedLabel: () => "Election",
}))

import {ENumberFormatPolicy, NumberFormatProvider} from "@sequentech/ui-core"
import {ResultsSummary} from "./ResultsSummary"

const electionResult = {
    id: "result-id",
    election_id: "election-id",
    name: "Election",
    elegible_census: 12000000,
    total_voters: 8589934,
    total_voters_percent: 0.715827833,
    blank_ballots: 1234,
}

describe("ResultsSummary", () => {
    it("renders a 100 percent non-voters pie for an empty tally", () => {
        const markup = renderToStaticMarkup(
            <ResultsSummary
                elections={[{id: "election-id", presentation: {en: "Election"}}]}
                resultsElections={[
                    {
                        id: "result-id",
                        election_id: "election-id",
                        name: "Election",
                        elegible_census: 0,
                        total_voters: 0,
                        total_voters_percent: 0,
                    },
                ]}
                locale="en"
            />
        )

        expect(markup).toContain("seq-results-summary__chart")
        expect(markup).toContain("seq-results-summary__pie")
        expect(markup).toContain('data-height="170"')
        expect(markup).toContain("Non voters")
        expect(markup).toContain('data-series="[100]"')
    })

    it("shows the blank ballots column when a row carries a numeric value", () => {
        const markup = renderToStaticMarkup(
            <ResultsSummary
                elections={[{id: "election-id", presentation: {en: "Election"}}]}
                resultsElections={[
                    {
                        id: "result-id",
                        election_id: "election-id",
                        name: "Election",
                        elegible_census: 10,
                        total_voters: 5,
                        total_voters_percent: 50,
                        blank_ballots: 2,
                    },
                ]}
                locale="en"
            />
        )

        expect(markup).toContain("seq-results-summary__blank-ballots-heading")
        expect(markup).toContain("seq-results-summary__blank-ballots-cell")
        expect(markup).toContain("Total blank ballots")
    })

    it("hides the blank ballots column when no row carries a numeric value", () => {
        const markup = renderToStaticMarkup(
            <ResultsSummary
                elections={[{id: "election-id", presentation: {en: "Election"}}]}
                resultsElections={[
                    {
                        id: "result-id",
                        election_id: "election-id",
                        name: "Election",
                        elegible_census: 10,
                        total_voters: 5,
                        total_voters_percent: 50,
                        blank_ballots: null,
                    },
                ]}
                locale="en"
            />
        )

        expect(markup).not.toContain("seq-results-summary__blank-ballots-heading")
        expect(markup).not.toContain("seq-results-summary__blank-ballots-cell")
    })

    it("writes the general information in the event's number format", () => {
        const markup = renderToStaticMarkup(
            <NumberFormatProvider policy={ENumberFormatPolicy.PERIOD_COMMA}>
                <ResultsSummary
                    elections={[{id: "election-id", presentation: {en: "Election"}}]}
                    resultsElections={[electionResult]}
                    locale="en"
                />
            </NumberFormatProvider>
        )

        expect(markup).toContain(">12.000.000<")
        expect(markup).toContain(">8.589.934<")
        expect(markup).toContain(">1.234<")
        expect(markup).toContain(">71,58%<")
        expect(markup).toContain('data-tooltip-labels="8.589.934|3.410.066"')
        expect(markup).toContain('data-slice-labels="71,6%|28,4%"')
    })

    it("writes comma grouped figures for results without a number format", () => {
        const markup = renderToStaticMarkup(
            <ResultsSummary
                elections={[{id: "election-id", presentation: {en: "Election"}}]}
                resultsElections={[electionResult]}
                locale="en"
            />
        )

        expect(markup).toContain(">12,000,000<")
        expect(markup).toContain(">8,589,934<")
        expect(markup).toContain(">1,234<")
        expect(markup).toContain(">71.58%<")
        expect(markup).toContain('data-tooltip-labels="8,589,934|3,410,066"')
        expect(markup).toContain('data-slice-labels="71.6%|28.4%"')
    })

    it("keeps a dash for figures the results do not carry", () => {
        const markup = renderToStaticMarkup(
            <NumberFormatProvider policy={ENumberFormatPolicy.PERIOD_COMMA}>
                <ResultsSummary
                    elections={[{id: "election-id", presentation: {en: "Election"}}]}
                    resultsElections={[{id: "result-id", election_id: "election-id"}]}
                    locale="en"
                />
            </NumberFormatProvider>
        )

        expect(markup).toMatch(/seq-results-summary__eligible-cell[^"]*">-</)
        expect(markup).toMatch(/seq-results-summary__participation-cell[^"]*">-</)
    })
})
