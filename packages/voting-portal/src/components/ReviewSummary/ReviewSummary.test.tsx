// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {ThemeProvider} from "@mui/material/styles"
import {act, render, screen, within} from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import {theme} from "@sequentech/ui-essentials"
import {BallotSelection} from "@sequentech/ui-core"
import {I18nextProvider} from "react-i18next"
import {createMemoryRouter, RouterProvider} from "react-router-dom"

import {i18n, marks} from "../../testing/ballotHarness"
import {PRESIDENT, SLATES, TRUSTEES} from "../../testing/slateFixtures"
import {getEditContestId} from "../../services/EditContest"
import {getBallotReviewSummary} from "../../services/ReviewSummary"
import {resolveSlates} from "../../services/Slates"
import {ReviewContestFooter} from "./ReviewContestFooter"
import {ReviewSelectionSummary} from "./ReviewSelectionSummary"

const CONTESTS = [PRESIDENT, TRUSTEES]
const RESOLVED = resolveSlates(SLATES, CONTESTS)

const ballot = (president: string[], trustees: string[]): BallotSelection => [
    marks(PRESIDENT, Object.fromEntries(president.map((id) => [id, 0]))),
    marks(TRUSTEES, Object.fromEntries(trustees.map((id) => [id, 0]))),
]

const mountSummary = (selection: BallotSelection) =>
    render(
        <I18nextProvider i18n={i18n}>
            <ThemeProvider theme={theme}>
                <ReviewSelectionSummary
                    summary={getBallotReviewSummary(CONTESTS, RESOLVED, selection)}
                    defaultLanguage="en"
                />
            </ThemeProvider>
        </I18nextProvider>
    )

afterEach(async () => {
    await act(() => i18n.changeLanguage("en"))
})

const labelText = () => document.querySelector(".review-selection-label")?.textContent

describe("the summary of the selections in review", () => {
    it("is a region named Your selections with the total of the ballot", () => {
        mountSummary(ballot(["f-president"], ["f-t1", "f-t2"]))

        const region = screen.getByRole("region", {name: "Your selections"})
        expect(within(region).getByRole("heading", {level: 2})).toHaveTextContent("Your selections")
        expect(region.querySelector(".review-selection-total")).toHaveTextContent("3 of 4")
        expect(region).toHaveTextContent("Candidates selected")
    })

    it("lets long slate names wrap instead of overflowing", () => {
        mountSummary(ballot(["f-president"], ["f-t1", "f-t2"]))

        expect(
            getComputedStyle(screen.getByRole("region", {name: "Your selections"}))
        ).toHaveProperty("overflowWrap", "anywhere")
    })

    it("names the slate the ballot holds exactly", () => {
        mountSummary(ballot(["f-president"], ["f-t1", "f-t2"]))

        expect(labelText()).toBe("Forward Together")
    })

    it("speaks of individual selections once a slate was changed by hand", () => {
        mountSummary(ballot(["i-president"], ["f-t1", "f-t2"]))

        expect(labelText()).toBe("Individual selections")
    })

    it("says nothing is selected on a blank ballot", () => {
        mountSummary(ballot([], []))

        expect(screen.getByRole("region")).toHaveTextContent("0 of 4")
        expect(labelText()).toBe("No candidates selected")
    })

    it("explains that only candidates are voted", () => {
        mountSummary(ballot([], []))

        expect(screen.getByText("Your vote is recorded for each selected candidate.")).toBeVisible()
    })

    it("names the slate in the voter's language", async () => {
        await act(() => i18n.changeLanguage("es"))
        mountSummary(ballot(["f-president"], ["f-t1", "f-t2"]))

        expect(labelText()).toBe("Adelante Juntos")
    })
})

describe("the footer of a contest in review", () => {
    const mountFooter = () => {
        const router = createMemoryRouter(
            [
                {
                    path: "/election/review",
                    element: (
                        <ReviewContestFooter
                            contest={TRUSTEES}
                            count={{selected: 2, max: 3}}
                            to="/election/vote?lang=en"
                        />
                    ),
                },
                {path: "/election/vote", element: <div>Voting screen</div>},
            ],
            {initialEntries: ["/election/review?lang=en"]}
        )
        render(
            <I18nextProvider i18n={i18n}>
                <ThemeProvider theme={theme}>
                    <RouterProvider router={router} />
                </ThemeProvider>
            </I18nextProvider>
        )
        return router
    }

    it("says how many candidates are selected out of the maximum", () => {
        mountFooter()

        expect(screen.getByText("2 of 3 selected")).toBeVisible()
    })

    it("offers to edit the contest by its name", () => {
        mountFooter()

        const edit = screen.getByRole("link", {name: "Edit Trustees"})
        expect(edit).toHaveAttribute("href", "/election/vote?lang=en")
    })

    it("marks a contest with unused choices", () => {
        mountFooter()

        expect(document.querySelector(".review-contest-footer")).toHaveClass(
            "review-contest-unfilled"
        )
        expect(document.querySelector(".review-contest-blank")).toBeNull()
    })

    it("goes to the voting screen and says which contest to open", async () => {
        const router = mountFooter()

        await userEvent.setup().click(screen.getByRole("link", {name: "Edit Trustees"}))

        expect(screen.getByText("Voting screen")).toBeVisible()
        expect(router.state.location.search).toBe("?lang=en")
        expect(getEditContestId(router.state.location.state)).toBe("trustees")
    })
})

describe("getEditContestId", () => {
    it.each([undefined, null, "trustees", {}, {editContestId: 3}])("is nothing for %p", (state) => {
        expect(getEditContestId(state)).toBeUndefined()
    })
})
