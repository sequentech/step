// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {combineReducers, configureStore} from "@reduxjs/toolkit"
import {ThemeProvider} from "@mui/material/styles"
import {act, render, screen, within} from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import {I18nextProvider} from "react-i18next"
import {Provider} from "react-redux"
import theme from "../../../../ui-essentials/src/services/theme"
import {
    buildSlatesBallot,
    buildSlatesBallotStyle,
    PRESIDENT,
    TRUSTEES,
} from "../../fixtures/slateChoices"
import ballotSelectionsReducer, {
    resetBallotSelection,
    setBallotSelectionVoteChoice,
} from "../../store/ballotSelections/ballotSelectionsSlice"
import {i18n} from "../../testing/ballotHarness"
import {SlateClearAction} from "./SlateClearAction"

const ballot = buildSlatesBallot()
const ballotStyle = buildSlatesBallotStyle(ballot)
const CLEAR = "Clear selections"

const mount = () => {
    const store = configureStore({
        reducer: combineReducers({ballotSelections: ballotSelectionsReducer}),
    })
    store.dispatch(resetBallotSelection({ballotStyle, force: true}))
    const {container} = render(
        <I18nextProvider i18n={i18n}>
            <ThemeProvider theme={theme}>
                <Provider store={store}>
                    <SlateClearAction ballotStyle={ballotStyle} />
                </Provider>
            </ThemeProvider>
        </I18nextProvider>
    )
    const selected = (): string[] =>
        (store.getState().ballotSelections[ballotStyle.election_id] ?? []).flatMap((contest) =>
            contest.choices.filter((choice) => choice.selected > -1).map((choice) => choice.id)
        )
    const mark = (contestId: string, id: string) =>
        act(() => {
            store.dispatch(
                setBallotSelectionVoteChoice({
                    ballotStyle,
                    contestId,
                    voteChoice: {id, selected: 0},
                })
            )
        })
    return {container, selected, mark}
}

describe("SlateClearAction", () => {
    it("is disabled while nothing is selected", () => {
        const {container} = mount()
        expect(container.querySelector(".slate-clear")).toContainElement(
            screen.getByRole("button", {name: CLEAR})
        )
        expect(screen.getByRole("button", {name: CLEAR})).toHaveClass("slate-clear-button")
        expect(screen.getByRole("button", {name: CLEAR})).toBeDisabled()
    })

    it("asks first and lists every choice it would remove", async () => {
        const {selected, mark} = mount()
        mark(PRESIDENT, "p-independent")
        mark(TRUSTEES, "t-forward-1")
        await userEvent.click(screen.getByRole("button", {name: CLEAR}))
        const dialog = screen.getByRole("dialog", {name: "Clear all your choices?"})
        const president = within(dialog).getByText("President").closest(".slate-replace-contest")
        expect(within(president as HTMLElement).getByText("Renata Halloran")).toBeVisible()
        expect(within(president as HTMLElement).getByText("No candidate")).toBeVisible()
        expect(within(dialog).getByText("Elio Marchetti")).toBeVisible()
        expect(within(dialog).queryByText("Secretary-Treasurer")).toBeNull()
        expect(selected()).toEqual(["p-independent", "t-forward-1"])
    })

    it("changes nothing when the voter keeps the choices", async () => {
        const {selected, mark} = mount()
        mark(PRESIDENT, "p-independent")
        await userEvent.click(screen.getByRole("button", {name: CLEAR}))
        await userEvent.click(screen.getByRole("button", {name: "Keep my choices"}))
        expect(selected()).toEqual(["p-independent"])
        expect(await screen.findByRole("status")).toBeEmptyDOMElement()
    })

    it("removes every choice once the voter confirms and says so", async () => {
        const {selected, mark} = mount()
        mark(PRESIDENT, "p-independent")
        mark(TRUSTEES, "t-forward-1")
        await userEvent.click(screen.getByRole("button", {name: CLEAR}))
        await userEvent.click(screen.getByRole("button", {name: "Clear choices"}))
        expect(selected()).toEqual([])
        expect(await screen.findByRole("status")).toHaveTextContent(
            "All your choices have been cleared."
        )
        expect(screen.getByRole("button", {name: CLEAR})).toBeDisabled()
    })

    it("stops announcing once the voter selects again", async () => {
        const {mark} = mount()
        mark(PRESIDENT, "p-independent")
        await userEvent.click(screen.getByRole("button", {name: CLEAR}))
        await userEvent.click(screen.getByRole("button", {name: "Clear choices"}))
        mark(PRESIDENT, "p-forward")
        expect(await screen.findByRole("status")).toBeEmptyDOMElement()
    })
})
