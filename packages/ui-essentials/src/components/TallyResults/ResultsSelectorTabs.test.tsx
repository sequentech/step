/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {render, screen, waitFor} from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import "@testing-library/jest-dom"
import {ThemeProvider} from "@mui/material/styles"
import theme from "../../services/theme"
import ResultsSelectorTabs, {type ResultsSelectorTabsProps} from "./ResultsSelectorTabs"

const LABELS = {
    elections: "Elections",
    contests: "Contests",
    areas: "Areas",
    empty: "No results yet",
}
const ELECTIONS = [
    {id: "council/one", label: "Council"},
    {id: "mayor", label: "Mayor"},
]
const defaultProps: ResultsSelectorTabsProps = {
    labels: LABELS,
    elections: ELECTIONS,
    getContests: ({election}) =>
        election
            ? [
                  {id: `${election.id}-first`, label: "First contest"},
                  {id: `${election.id}-second`, label: "Second contest"},
              ]
            : [],
    getAreas: ({contest}) =>
        contest
            ? [
                  {id: null, label: "All areas"},
                  {id: "north", label: "North"},
              ]
            : [],
    renderResult: ({electionId, contestId, areaId}) => (
        <output aria-label="Selected results">
            {JSON.stringify({electionId, contestId, areaId})}
        </output>
    ),
}

function selector(props: Partial<ResultsSelectorTabsProps> = {}) {
    return (
        <ThemeProvider theme={theme}>
            <ResultsSelectorTabs {...defaultProps} {...props} />
        </ThemeProvider>
    )
}
const selected = () =>
    JSON.parse(screen.getByLabelText("Selected results").textContent || "null") as {
        electionId: string | null
        contestId: string | null
        areaId: string | null
    }

it("resets dependent contest and area selection when the election changes", async () => {
    const user = userEvent.setup()
    const onSelectionChange = jest.fn()
    render(selector({onSelectionChange}))
    await user.click(screen.getByRole("tab", {name: "Second contest"}))
    await user.click(screen.getByRole("tab", {name: "North"}))
    expect(selected()).toEqual({
        electionId: "council/one",
        contestId: "council/one-second",
        areaId: "north",
    })
    await user.click(screen.getByRole("tab", {name: "Mayor"}))
    await waitFor(() =>
        expect(selected()).toEqual({electionId: "mayor", contestId: "mayor-first", areaId: null})
    )
    expect(onSelectionChange).toHaveBeenLastCalledWith(expect.objectContaining(selected()))
    expect(screen.getByRole("tab", {name: "All areas"})).toHaveAttribute("aria-selected", "true")
})

it("honors the initial election and recovers when that election disappears", async () => {
    const {rerender} = render(selector({initialElectionId: "mayor"}))
    expect(selected().electionId).toBe("mayor")
    rerender(selector({initialElectionId: "mayor", elections: [ELECTIONS[0]]}))
    await waitFor(() => expect(selected().electionId).toBe("council/one"))
    expect(screen.getByRole("tab", {name: "Council"})).toHaveClass(
        "seq-results-selector__election-tab--council-one"
    )
})

it("renders the empty state and reports no selection when all elections disappear", async () => {
    const onSelectionChange = jest.fn()
    const {rerender} = render(selector({onSelectionChange}))
    rerender(selector({elections: [], onSelectionChange}))
    expect(screen.getByText("No results yet")).toBeVisible()
    expect(screen.queryByRole("tab")).toBeNull()
    await waitFor(() =>
        expect(onSelectionChange).toHaveBeenLastCalledWith(
            expect.objectContaining({electionId: null, contestId: null, areaId: null})
        )
    )
})

it("supports elections with no contests and optional actions without inventing a result", () => {
    render(
        selector({
            initialElectionId: "missing",
            getContests: () => [],
            getAreas: () => [],
            renderElectionActions: () => <button>Election report</button>,
            renderContestActions: () => <span>No contest actions</span>,
            renderAreaActions: () => <span>No area actions</span>,
        })
    )
    expect(selected()).toEqual({electionId: "council/one", contestId: null, areaId: null})
    expect(screen.getByRole("button", {name: "Election report"})).toBeVisible()
    expect(screen.getByText("No contest actions")).toBeVisible()
    expect(screen.getByText("No area actions")).toBeVisible()
})
