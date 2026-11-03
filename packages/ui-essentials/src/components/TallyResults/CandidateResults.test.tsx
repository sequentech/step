// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {renderToStaticMarkup} from "react-dom/server"
import type {GridColDef, GridRenderCellParams} from "@mui/x-data-grid"
import type {CandidateResultRow} from "./types"

jest.mock("./ChartPanel", () => {
    const react = jest.requireActual<typeof import("react")>("react")

    return {
        Chart: ({
            className,
            options,
            series = [],
        }: {
            className?: string
            options?: {tooltip?: {y?: {formatter?: (value: number) => string}}}
            series?: number[]
        }) =>
            react.createElement("div", {
                className,
                "data-tooltip-labels": series
                    .map((value) => options?.tooltip?.y?.formatter?.(value))
                    .join("|"),
            }),
        ChartPanel: ({children}: {children: React.ReactNode}) =>
            react.createElement("section", null, children),
    }
})

// The data grid lays rows out only once it has measured itself, which never
// happens in a static render, so render every cell the way its column does.
jest.mock("@mui/x-data-grid", () => {
    const react = jest.requireActual<typeof import("react")>("react")

    return {
        DataGrid: ({
            rows,
            columns,
        }: {
            rows: CandidateResultRow[]
            columns: GridColDef<CandidateResultRow>[]
        }) =>
            react.createElement(
                "table",
                null,
                react.createElement(
                    "tbody",
                    null,
                    rows.map((row) =>
                        react.createElement(
                            "tr",
                            {key: row.id},
                            columns.map((column) => {
                                const value = row[column.field as keyof CandidateResultRow]

                                return react.createElement(
                                    "td",
                                    {key: column.field, className: column.field},
                                    column.renderCell
                                        ? column.renderCell({
                                              value,
                                              row,
                                          } as GridRenderCellParams<CandidateResultRow>)
                                        : value
                                )
                            })
                        )
                    )
                )
            ),
    }
})

// ui-core's built dist is unavailable when this package's tests run alone, so
// load the number format modules from source.
jest.mock(
    "@sequentech/ui-core",
    () => ({
        ...jest.requireActual<typeof import("@sequentech/ui-core")>(
            "../../../../ui-core/src/types/VotingChannel"
        ),
        ...jest.requireActual<typeof import("@sequentech/ui-core")>(
            "../../../../ui-core/src/types/ElectionEventPresentation"
        ),
        ...jest.requireActual<typeof import("@sequentech/ui-core")>(
            "../../../../ui-core/src/services/numberFormat"
        ),
        ...jest.requireActual<typeof import("@sequentech/ui-core")>(
            "../../../../ui-core/src/services/NumberFormatContext"
        ),
        ...jest.requireActual<typeof import("@sequentech/ui-core")>(
            "../../../../ui-core/src/services/percentFormatter"
        ),
    }),
    {virtual: true}
)

import {ENumberFormatPolicy, NumberFormatProvider} from "@sequentech/ui-core"
import {CandidateResults} from "./CandidateResults"

const candidates: CandidateResultRow[] = [
    {
        id: "candidate-a",
        name: "Candidate A",
        castVotes: 10999999,
        castVotesPercent: 0.9166666,
        winningPosition: 1,
    },
    {
        id: "candidate-b",
        name: "Candidate B",
        castVotes: 1000001,
        castVotesPercent: 0.0833334,
        winningPosition: null,
    },
]

describe("CandidateResults", () => {
    it("writes candidate votes and percentages in the event's number format", () => {
        const markup = renderToStaticMarkup(
            <NumberFormatProvider policy={ENumberFormatPolicy.PERIOD_COMMA}>
                <CandidateResults candidates={candidates} chartName="Election - Contest" />
            </NumberFormatProvider>
        )

        expect(markup).toContain('class="castVotes">10.999.999<')
        expect(markup).toContain('class="castVotesPercent">91,67%<')
        expect(markup).toContain('class="castVotes">1.000.001<')
        expect(markup).toContain('class="castVotesPercent">8,33%<')
        expect(markup).toContain('class="winningPosition">1<')
        expect(markup).toContain('class="winningPosition">-<')
        expect(markup).toContain('data-tooltip-labels="10.999.999|1.000.001"')
    })

    it("writes comma grouped votes for events without a number format", () => {
        const markup = renderToStaticMarkup(
            <CandidateResults candidates={candidates} chartName="Election - Contest" />
        )

        expect(markup).toContain('class="castVotes">10,999,999<')
        expect(markup).toContain('class="castVotesPercent">91.67%<')
        expect(markup).toContain('data-tooltip-labels="10,999,999|1,000,001"')
    })
})
