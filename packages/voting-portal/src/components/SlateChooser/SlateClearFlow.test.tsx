// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
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
    SECRETARY,
    TRUSTEES,
} from "../../fixtures/slateChoices"
import ballotSelectionsReducer, {
    resetBallotSelection,
    setBallotSelectionVoteChoice,
} from "../../store/ballotSelections/ballotSelectionsSlice"
import {i18n} from "../../testing/ballotHarness"
import extraReducer from "../../store/extra/extraSlice"
import type {IBallotStyle} from "../../store/ballotStyles/ballotStylesSlice"
import {useSlateClear} from "./SlateClearFlow"

const ballot = buildSlatesBallot()
const ballotStyle = buildSlatesBallotStyle(ballot)
const CLEAR = "Clear selections"

/** The footer's Clear selections, as the voting screen wires it. */
const ClearButton: React.FC<{ballotStyle: IBallotStyle}> = ({ballotStyle: style}) => {
    const {canClear, requestClear, feedback} = useSlateClear(style)
    return (
        <>
            <button className="clear-selection-button" disabled={!canClear} onClick={requestClear}>
                {CLEAR}
            </button>
            {feedback}
        </>
    )
}

const mount = (style = ballotStyle) => {
    const store = configureStore({
        reducer: combineReducers({ballotSelections: ballotSelectionsReducer, extra: extraReducer}),
    })
    store.dispatch(resetBallotSelection({ballotStyle: style, force: true}))
    const {container} = render(
        <I18nextProvider i18n={i18n}>
            <ThemeProvider theme={theme}>
                <Provider store={store}>
                    <ClearButton ballotStyle={style} />
                </Provider>
            </ThemeProvider>
        </I18nextProvider>
    )
    const selected = (): string[] =>
        (store.getState().ballotSelections[style.election_id] ?? []).flatMap((contest) =>
            contest.choices.filter((choice) => choice.selected > -1).map((choice) => choice.id)
        )
    const mark = (contestId: string, id: string) =>
        act(() => {
            store.dispatch(
                setBallotSelectionVoteChoice({
                    ballotStyle: style,
                    contestId,
                    voteChoice: {id, selected: 0},
                })
            )
        })
    return {container, selected, mark}
}

describe("clearing a ballot with slates", () => {
    it("cannot be used while nothing is selected", () => {
        mount()
        expect(screen.getByRole("button", {name: CLEAR})).toBeDisabled()
        expect(screen.queryByRole("dialog")).toBeNull()
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

describe("the contests of the confirmation", () => {
    it("follow the ballot order, not the order of the ballot style", async () => {
        const reordered = buildSlatesBallot()
        reordered.contests = reordered.contests
            .map((contest, index) => ({...contest, presentation: {sort_order: index}}))
            .reverse()
        const {mark} = mount(buildSlatesBallotStyle(reordered))
        mark(TRUSTEES, "t-forward-1")
        mark(PRESIDENT, "p-independent")
        mark(SECRETARY, "s-forward")
        await userEvent.click(screen.getByRole("button", {name: CLEAR}))

        const names = Array.from(
            screen.getByRole("dialog").querySelectorAll(".slate-replace-contest-name")
        ).map((name) => name.textContent)
        expect(names).toEqual(["President", "Secretary-Treasurer", "Trustees"])
    })
})
