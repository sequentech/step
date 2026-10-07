// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {combineReducers, configureStore} from "@reduxjs/toolkit"
import {ThemeProvider} from "@mui/material/styles"
import {render, screen} from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import {Question, theme} from "@sequentech/ui-essentials"
import {I18nextProvider} from "react-i18next"
import {Provider} from "react-redux"
import {
    buildLimitedSlatesBallot,
    buildSlate,
    buildSlatesBallotStyle,
    PRESIDENT,
    TRUSTEES,
} from "../../fixtures/slateChoices"
import type {IResolvedSlate} from "../../services/Slates"
import ballotSelectionsReducer, {
    resetBallotSelection,
} from "../../store/ballotSelections/ballotSelectionsSlice"
import extraReducer from "../../store/extra/extraSlice"
import {i18n} from "../../testing/ballotHarness"
import {BallotSelectionAdapter} from "../BallotSelectionAdapter"
import {SlateApplyAction} from "./SlateApplyAction"

const ballot = buildLimitedSlatesBallot()
const ballotStyle = buildSlatesBallotStyle(ballot)

const mount = (slates: IResolvedSlate[]) => {
    const store = configureStore({
        reducer: combineReducers({ballotSelections: ballotSelectionsReducer, extra: extraReducer}),
    })
    store.dispatch(resetBallotSelection({ballotStyle, force: true}))
    render(
        <I18nextProvider i18n={i18n}>
            <ThemeProvider theme={theme}>
                <Provider store={store}>
                    {slates.map((slate) => (
                        <SlateApplyAction key={slate.id} ballotStyle={ballotStyle} slate={slate} />
                    ))}
                    <BallotSelectionAdapter>
                        {ballot.contests.map((contest) => (
                            <Question
                                key={contest.id}
                                ballotStyle={ballotStyle}
                                question={contest}
                                isReview={false}
                                setDecodedContests={() => undefined}
                                errorSelectionState={[]}
                            />
                        ))}
                    </BallotSelectionAdapter>
                </Provider>
            </ThemeProvider>
        </I18nextProvider>
    )
    return (contestId: string): string[] =>
        (
            store
                .getState()
                .ballotSelections[
                    ballotStyle.election_id
                ]?.find((contest) => contest.contest_id === contestId)?.choices ?? []
        )
            .filter((choice) => choice.selected > -1)
            .map((choice) => choice.id)
}

const candidate = (name: string): HTMLElement => screen.getByRole("checkbox", {name})
const chooseSlate = (name: string) =>
    userEvent.click(screen.getByRole("button", {name: `Choose slate ${name}`}))

describe("selection limits on the ballot", () => {
    it("keeps one candidate in a single-seat office when another is chosen", async () => {
        const selected = mount([])
        await userEvent.click(candidate("Amara Lindqvist"))
        await userEvent.click(candidate("Renata Halloran"))
        expect(selected(PRESIDENT)).toEqual(["p-independent"])
        expect(candidate("Amara Lindqvist")).not.toBeChecked()
    })

    it("accepts three trustees from different slates and blocks a fourth", async () => {
        const selected = mount([])
        await userEvent.click(candidate("Elio Marchetti"))
        await userEvent.click(candidate("Hana Villeneuve"))
        await userEvent.click(candidate("Kasper Yilmaz"))
        expect(candidate("Malik Fenwick")).toBeDisabled()
        await userEvent.click(candidate("Malik Fenwick"), {pointerEventsCheck: 0})
        expect(selected(TRUSTEES)).toEqual(["t-forward-1", "t-members-1", "t-independent"])
    })

    it("blocks a fourth trustee after a slate filled the contest", async () => {
        const selected = mount([buildSlate(ballot, "forward")])
        await chooseSlate("Forward Together")
        expect(candidate("Kasper Yilmaz")).toBeDisabled()
        expect(candidate("Elio Marchetti")).not.toBeDisabled()
        expect(selected(TRUSTEES)).toEqual(["t-forward-1", "t-forward-2", "t-forward-3"])
    })

    it("lets the voter swap a slate trustee for another candidate", async () => {
        const selected = mount([buildSlate(ballot, "forward")])
        await chooseSlate("Forward Together")
        await userEvent.click(candidate("Elio Marchetti"))
        await userEvent.click(candidate("Kasper Yilmaz"))
        expect(selected(TRUSTEES)).toEqual(["t-forward-2", "t-forward-3", "t-independent"])
        expect(candidate("Elio Marchetti")).toBeDisabled()
    })

    it("leaves room for one more trustee after a slate with two", async () => {
        const selected = mount([buildSlate(ballot, "independent-voices")])
        await chooseSlate("Independent Voices")
        expect(candidate("Kasper Yilmaz")).not.toBeDisabled()
        await userEvent.click(candidate("Kasper Yilmaz"))
        expect(selected(TRUSTEES)).toEqual(["t-voices-1", "t-voices-2", "t-independent"])
        expect(candidate("Elio Marchetti")).toBeDisabled()
    })

    it("replaces a single-seat choice with the slate's candidate after confirmation", async () => {
        const selected = mount([buildSlate(ballot, "members-first")])
        await userEvent.click(candidate("Renata Halloran"))
        await chooseSlate("Members First")
        await userEvent.click(screen.getByRole("button", {name: "Replace choices"}))
        expect(selected(PRESIDENT)).toEqual(["p-members"])
    })
})

describe("a slate with more candidates than a contest allows", () => {
    const tooMany = (): IResolvedSlate => {
        const slate = structuredClone(buildSlate(ballot, "forward"))
        slate.contests[2].contest.max_votes = 2
        return slate
    }

    it("cannot be chosen and says why", () => {
        mount([tooMany()])
        const button = screen.getByRole("button", {name: "Choose slate Forward Together"})
        expect(button).toBeDisabled()
        expect(button).toHaveAccessibleDescription(
            "Forward Together cannot be chosen: it has 3 candidates for Trustees, which allows 2. You can still choose candidates individually."
        )
    })

    it("leaves individual choices available", async () => {
        const selected = mount([tooMany()])
        await userEvent.click(candidate("Amara Lindqvist"))
        expect(selected(PRESIDENT)).toEqual(["p-forward"])
    })
})

describe("a slate that fits", () => {
    it("gives no reason next to its button", () => {
        mount([buildSlate(ballot, "forward")])
        expect(document.querySelector(".slate-apply-unavailable")).toBeNull()
    })
})
