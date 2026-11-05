// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {configureStore} from "@reduxjs/toolkit"
import {ThemeProvider} from "@mui/material/styles"
import useMediaQuery from "@mui/material/useMediaQuery"
import {act, render, screen, within} from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import {theme} from "@sequentech/ui-essentials"
import {I18nextProvider} from "react-i18next"
import {Provider} from "react-redux"

import {i18n} from "../../testing/ballotHarness"
import {aSlateBallotStyle, PRESIDENT, SLATES, TRUSTEES} from "../../testing/slateFixtures"
import {resolveSlates} from "../../services/Slates"
import ballotSelectionsReducer, {
    resetBallotSelection,
    setBallotSelectionVoteChoice,
} from "../../store/ballotSelections/ballotSelectionsSlice"
import extraReducer from "../../store/extra/extraSlice"
import {SlateApplyAction} from "./SlateApplyAction"
import {SlateSelectionChooser} from "./SlateSelectionChooser"

jest.mock("@mui/material/useMediaQuery")
const mockedUseMediaQuery = jest.mocked(useMediaQuery)

const BALLOT_STYLE = aSlateBallotStyle(JSON.stringify(SLATES))
const RESOLVED = resolveSlates(SLATES, [PRESIDENT, TRUSTEES])

const newStore = () => {
    const store = configureStore({
        reducer: {extra: extraReducer, ballotSelections: ballotSelectionsReducer},
    })
    store.dispatch(resetBallotSelection({ballotStyle: BALLOT_STYLE, force: true}))
    return store
}
type Store = ReturnType<typeof newStore>

const choose = (store: Store, contestId: string, candidateId: string, selected = 0) =>
    act(() => {
        store.dispatch(
            setBallotSelectionVoteChoice({
                ballotStyle: BALLOT_STYLE,
                contestId,
                voteChoice: {id: candidateId, selected},
            })
        )
    })

const mount = (store: Store, onEditSelections = jest.fn()) =>
    render(
        <Provider store={store}>
            <I18nextProvider i18n={i18n}>
                <ThemeProvider theme={theme}>
                    <SlateSelectionChooser
                        ballotStyle={BALLOT_STYLE}
                        slates={RESOLVED}
                        defaultLanguage="en"
                        onEditSelections={onEditSelections}
                        renderApplyAction={(slate) => (
                            <SlateApplyAction ballotStyle={BALLOT_STYLE} slate={slate} />
                        )}
                    />
                </ThemeProvider>
            </I18nextProvider>
        </Provider>
    )

const cardOf = (name: string) => {
    const card = screen.getByRole("heading", {level: 3, name}).closest("li")
    if (!card) {
        throw new Error(`no card for ${name}`)
    }
    return within(card)
}

const selectedNames = (name: string) =>
    Array.from(
        (
            screen.getByRole("heading", {level: 3, name}).closest("li") as HTMLElement
        ).querySelectorAll(".slate-member-selected .slate-member-name")
    ).map((member) => member.textContent)

beforeEach(() => mockedUseMediaQuery.mockReturnValue(false))

describe("the slates of a ballot with selections", () => {
    it("show no mark and no summary before anything is selected", () => {
        mount(newStore())

        expect(document.querySelector(".slate-member-selected")).toBeNull()
        expect(document.querySelector(".slate-selection-status")).toBeNull()
    })

    it("mark a member chosen individually and count it", () => {
        const store = newStore()
        mount(store)

        choose(store, "trustees", "f-t1")

        expect(selectedNames("Forward Together")).toEqual(["Rowan Scott"])
        expect(cardOf("Forward Together").getByText("Partly selected · 1 of 3")).toBeVisible()
        expect(selectedNames("Members First")).toEqual([])
    })

    it("mark one trustee of each slate as a mixed selection in every card", () => {
        const store = newStore()
        mount(store)

        choose(store, "trustees", "f-t1")
        choose(store, "trustees", "m-t1")
        choose(store, "trustees", "v-t1")

        expect(selectedNames("Forward Together")).toEqual(["Rowan Scott"])
        expect(selectedNames("Members First")).toEqual(["Skyler James"])
        expect(selectedNames("Independent Voices")).toEqual(["Harper Lane"])
        expect(cardOf("Forward Together").getByText("Mixed · 1 of 3 selected")).toBeVisible()
        expect(cardOf("Members First").getByText("Mixed · 1 of 2 selected")).toBeVisible()
        expect(cardOf("Independent Voices").getByText("Mixed · 1 of 1 selected")).toBeVisible()
    })

    it("say all are selected when the covered offices hold exactly the slate", () => {
        const store = newStore()
        mount(store)

        choose(store, "president", "i-president")
        choose(store, "trustees", "v-t1")

        expect(cardOf("Independent Voices").getByText("All 1 selected")).toBeVisible()
    })

    it("turn a full slate into a mixed one when an independent replaces a member", () => {
        const store = newStore()
        mount(store)
        choose(store, "president", "f-president")
        choose(store, "trustees", "f-t1")
        choose(store, "trustees", "f-t2")
        expect(cardOf("Forward Together").getByText("All 3 selected")).toBeVisible()

        choose(store, "president", "f-president", -1)
        choose(store, "president", "i-president")

        expect(selectedNames("Forward Together")).toEqual(["Rowan Scott", "Charlie Kim"])
        expect(cardOf("Forward Together").getByText("Mixed · 2 of 3 selected")).toBeVisible()
    })

    it("clear the mark and the summary when the member is deselected", () => {
        const store = newStore()
        mount(store)
        choose(store, "trustees", "v-t1")
        expect(selectedNames("Independent Voices")).toEqual(["Harper Lane"])

        choose(store, "trustees", "v-t1", -1)

        expect(selectedNames("Independent Voices")).toEqual([])
        expect(document.querySelector(".slate-selection-status")).toBeNull()
    })

    it("show the current selections again when the screen is shown again", () => {
        const store = newStore()
        const first = mount(store)
        choose(store, "trustees", "m-t1")
        first.unmount()

        mount(store)

        expect(selectedNames("Members First")).toEqual(["Skyler James"])
        expect(cardOf("Members First").getByText("Partly selected · 1 of 2")).toBeVisible()
    })

    it("offer to select a slate unless it is already fully selected", async () => {
        const user = userEvent.setup()
        const store = newStore()
        const onEditSelections = jest.fn()
        mount(store, onEditSelections)

        expect(
            cardOf("Independent Voices").getByRole("button", {
                name: "Choose slate Independent Voices",
            })
        ).toBeVisible()
        expect(screen.queryByRole("button", {name: "Edit selections"})).toBeNull()

        choose(store, "trustees", "v-t1")

        expect(
            cardOf("Independent Voices").queryByRole("button", {
                name: "Choose slate Independent Voices",
            })
        ).toBeNull()
        await user.click(
            cardOf("Independent Voices").getByRole("button", {name: "Edit selections"})
        )
        expect(onEditSelections).toHaveBeenCalledTimes(1)
        expect(
            cardOf("Forward Together").getByRole("button", {name: "Choose slate Forward Together"})
        ).toBeVisible()
    })

    it("mark the whole slate, keep its announcement and focus Edit selections once it is selected", async () => {
        const user = userEvent.setup()
        const store = newStore()
        mount(store)

        await user.click(
            cardOf("Forward Together").getByRole("button", {name: "Choose slate Forward Together"})
        )

        expect(selectedNames("Forward Together")).toEqual([
            "Jordan Ellis",
            "Rowan Scott",
            "Charlie Kim",
        ])
        expect(cardOf("Forward Together").getByText("All 3 selected")).toBeVisible()
        expect(cardOf("Forward Together").getByRole("status")).toHaveTextContent(
            "Forward Together chosen"
        )
        expect(
            cardOf("Forward Together").getByRole("button", {name: "Edit selections"})
        ).toHaveFocus()
    })

    it("follow a slate that is changed by hand into a mixed ballot", async () => {
        const user = userEvent.setup()
        const store = newStore()
        mount(store)
        await user.click(
            cardOf("Forward Together").getByRole("button", {name: "Choose slate Forward Together"})
        )

        choose(store, "trustees", "f-t2", -1)
        choose(store, "trustees", "v-t1")

        expect(selectedNames("Forward Together")).toEqual(["Jordan Ellis", "Rowan Scott"])
        expect(cardOf("Forward Together").getByText("Mixed · 2 of 3 selected")).toBeVisible()
        expect(cardOf("Independent Voices").getByText("Mixed · 1 of 1 selected")).toBeVisible()
        expect(
            cardOf("Forward Together").getByRole("button", {name: "Choose slate Forward Together"})
        ).toBeVisible()
        expect(
            cardOf("Forward Together").queryByRole("button", {name: "Edit selections"})
        ).toBeNull()
    })

    describe("on a phone", () => {
        beforeEach(() => mockedUseMediaQuery.mockReturnValue(true))

        it("keep the summary visible above a collapsed candidate list", () => {
            const store = newStore()
            mount(store)

            choose(store, "trustees", "f-t1")

            const card = cardOf("Forward Together")
            expect(card.getByRole("button", {name: "Show candidates"})).toHaveAttribute(
                "aria-expanded",
                "false"
            )
            expect(card.getByText("Rowan Scott")).not.toBeVisible()
            expect(card.getByText("Partly selected · 1 of 3")).toBeVisible()
        })

        it("show the marks once the voter expands the list", async () => {
            const user = userEvent.setup()
            const store = newStore()
            mount(store)
            choose(store, "trustees", "f-t1")

            await user.click(
                cardOf("Forward Together").getByRole("button", {name: "Show candidates"})
            )

            expect(cardOf("Forward Together").getByText("Rowan Scott")).toBeVisible()
            expect(selectedNames("Forward Together")).toEqual(["Rowan Scott"])
        })
    })
})
