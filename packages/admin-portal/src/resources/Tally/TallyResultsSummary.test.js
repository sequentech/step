// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
const React = require("react")
const {renderToStaticMarkup} = require("react-dom/server")

jest.mock("./TallyResultsCharts", () => ({
    ParticipationSummaryChart: () => null,
}))
jest.mock("react-i18next", () => ({
    useTranslation: () => ({
        t: (key) => key,
    }),
}))
jest.mock("@sequentech/ui-core", () => ({
    formatPercentOne: (value) => `${Number(value).toFixed(2)}%`,
    isNumber: (value) => typeof value === "number" && !Number.isNaN(value),
}))

const {TallyResultsSummary} = require("./TallyResultsSummary")

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

        for (const rowClassName of [
            "eligible-voters",
            "total-auditable-votes",
            "total-votes-counted",
            "total-valid-votes",
            "total-invalid-votes",
            "explicitly-invalid-votes",
            "implicitly-invalid-votes",
            "blank-votes",
            "weight",
        ]) {
            expect(markup).toMatch(new RegExp(`participation-summary-row[^"]*${rowClassName}`))
        }
    })
})
