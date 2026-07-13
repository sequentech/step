// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {renderToStaticMarkup} from "react-dom/server"

jest.mock("./TallyResultsCharts", () => ({
    ParticipationSummaryChart: () => null,
}))
jest.mock("react-i18next", () => ({
    useTranslation: () => ({
        t: (key) =>
            ({
                "tally.table.global": "Participation Summary",
                "tally.table.total": "Total",
                "tally.table.turnout": "%",
                "tally.table.elegible_census": "Eligible Voters",
                "tally.table.total_auditable_votes": "Total Auditable Votes",
                "tally.table.total_votes_counted": "Total Votes Counted",
                "tally.table.total_valid_votes": "Total Valid Votes",
                "tally.table.total_invalid_votes": "Total Invalid Votes",
                "tally.table.explicit_invalid_votes": "Explicitly Invalid Votes",
                "tally.table.implicit_invalid_votes": "Implicitly Invalid Votes",
                "tally.table.blank_votes": "Blank Votes",
                "tally.table.weight": "Weight",
            })[key] ?? key,
    }),
}))
jest.mock("@sequentech/ui-core", () => ({
    formatPercentOne: (value) => `${Number(value).toFixed(2)}%`,
    isNumber: (value) => typeof value === "number" && !Number.isNaN(value),
}))

import {TallyResultsSummary} from "./TallyResultsSummary"

const escapeRegExp = (value) => value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")

describe("TallyResultsSummary", () => {
    it("renders stable css classes for participation summary rows", () => {
        const general = [
            {
                elegible_census: 1316,
                total_auditable_votes: 0,
                total_auditable_votes_percent: 0,
                total_votes: 1,
                total_votes_percent: 0.08,
                total_valid_votes: 0,
                total_valid_votes_percent: 0,
                total_invalid_votes: 1,
                total_invalid_votes_percent: 100,
                explicit_invalid_votes: 0,
                explicit_invalid_votes_percent: 0,
                implicit_invalid_votes: 1,
                implicit_invalid_votes_percent: 100,
                blank_votes: 0,
                blank_votes_percent: 0,
                weight: 1,
            },
        ]

        const markup = renderToStaticMarkup(
            React.createElement(TallyResultsSummary, {
                general,
                chartName: "Participation Summary",
                showWeight: true,
                weight: 1,
            })
        )

        const expectedRows = [
            {rowClassName: "eligible-voters", label: "Eligible Voters", total: "1316", percent: ""},
            {
                rowClassName: "total-auditable-votes",
                label: "Total Auditable Votes",
                total: "0",
                percent: "0.00%",
            },
            {
                rowClassName: "total-votes-counted",
                label: "Total Votes Counted",
                total: "1",
                percent: "0.08%",
            },
            {
                rowClassName: "total-valid-votes",
                label: "Total Valid Votes",
                total: "0",
                percent: "0.00%",
            },
            {
                rowClassName: "total-invalid-votes",
                label: "Total Invalid Votes",
                total: "1",
                percent: "100.00%",
            },
            {
                rowClassName: "explicitly-invalid-votes",
                label: "Explicitly Invalid Votes",
                total: "0",
                percent: "0.00%",
            },
            {
                rowClassName: "implicitly-invalid-votes",
                label: "Implicitly Invalid Votes",
                total: "1",
                percent: "100.00%",
            },
            {rowClassName: "blank-votes", label: "Blank Votes", total: "0", percent: "0.00%"},
            {rowClassName: "weight", label: "Weight", total: "1", percent: ""},
        ]

        expect(markup).toContain("participation-summary-table")

        for (const {rowClassName, label, total, percent} of expectedRows) {
            expect(markup).toMatch(
                new RegExp(
                    `<tr[^>]*class="[^"]*participation-summary-row[^"]*${escapeRegExp(rowClassName)}[^"]*"[^>]*>[\\s\\S]*?${escapeRegExp(label)}[\\s\\S]*?${escapeRegExp(total)}[\\s\\S]*?${escapeRegExp(percent)}[\\s\\S]*?<\\/tr>`
                )
            )
        }
    })
})
