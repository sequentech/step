// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {configureStore} from "@reduxjs/toolkit"
import {render, screen} from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import useMediaQuery from "@mui/material/useMediaQuery"
import {ThemeProvider} from "@mui/material/styles"
import i18next from "i18next"
import {I18nextProvider} from "react-i18next"
import {Provider} from "react-redux"
import theme from "../../../../ui-essentials/src/services/theme"
import englishTranslation from "../../translations/en"
import extraReducer, {
    selectSlateListExpanded,
    setSlateListExpanded,
} from "../../store/extra/extraSlice"
import ballotSelectionsReducer from "../../store/ballotSelections/ballotSelectionsSlice"
import {RootState} from "../../store/store"
import {SlateCandidateList} from "./SlateCandidateList"

jest.mock("@mui/material/useMediaQuery")
const mockedUseMediaQuery = jest.mocked(useMediaQuery)

const i18n = i18next.createInstance()
void i18n.init({
    lng: "en",
    fallbackLng: "en",
    resources: {en: {translation: englishTranslation.translations}},
    interpolation: {escapeValue: false},
})

const ELECTION_ID = "election-1"

const newStore = () =>
    configureStore({reducer: {extra: extraReducer, ballotSelections: ballotSelectionsReducer}})

const mount = (
    store: ReturnType<typeof newStore>,
    props: {slateId?: string; defaultExpanded?: boolean} = {}
) =>
    render(
        <Provider store={store}>
            <ThemeProvider theme={theme}>
                <I18nextProvider i18n={i18n}>
                    <SlateCandidateList
                        electionId={ELECTION_ID}
                        slateId={props.slateId ?? "forward"}
                        defaultExpanded={props.defaultExpanded ?? false}
                    >
                        <ul>
                            <li>Rowan Scott</li>
                        </ul>
                    </SlateCandidateList>
                </I18nextProvider>
            </ThemeProvider>
        </Provider>
    )

const expandedIn = (store: ReturnType<typeof newStore>, slateId: string) =>
    selectSlateListExpanded(ELECTION_ID, slateId)(store.getState() as unknown as RootState)

describe("extraSlice slate candidate lists", () => {
    it("has no stored state until the voter toggles a list", () => {
        expect(expandedIn(newStore(), "forward")).toBeUndefined()
    })

    it("remembers each slate of each election separately", () => {
        const store = newStore()

        store.dispatch(
            setSlateListExpanded({electionId: ELECTION_ID, slateId: "forward", expanded: true})
        )
        store.dispatch(
            setSlateListExpanded({electionId: ELECTION_ID, slateId: "voices", expanded: false})
        )

        expect(expandedIn(store, "forward")).toBe(true)
        expect(expandedIn(store, "voices")).toBe(false)
        expect(
            selectSlateListExpanded(
                "election-2",
                "forward"
            )(store.getState() as unknown as RootState)
        ).toBeUndefined()
    })
})

describe("SlateCandidateList", () => {
    describe("on desktop", () => {
        beforeEach(() => mockedUseMediaQuery.mockReturnValue(false))

        it("always shows the candidates, without a toggle", () => {
            mount(newStore(), {defaultExpanded: false})

            expect(screen.getByText("Rowan Scott")).toBeVisible()
            expect(screen.queryByRole("button")).toBeNull()
        })
    })

    describe("on a phone", () => {
        beforeEach(() => mockedUseMediaQuery.mockReturnValue(true))

        it("starts collapsed when the election is configured so", () => {
            mount(newStore(), {defaultExpanded: false})

            const toggle = screen.getByRole("button", {name: "Show candidates"})
            expect(toggle).toHaveAttribute("aria-expanded", "false")
            expect(screen.getByText("Rowan Scott")).not.toBeVisible()
        })

        it("starts expanded when the election is configured so", () => {
            mount(newStore(), {defaultExpanded: true})

            const toggle = screen.getByRole("button", {name: "Hide candidates"})
            expect(toggle).toHaveAttribute("aria-expanded", "true")
            expect(screen.getByText("Rowan Scott")).toBeVisible()
        })

        it("lets the voter expand and collapse the list", async () => {
            const user = userEvent.setup()
            mount(newStore(), {defaultExpanded: false})

            await user.click(screen.getByRole("button", {name: "Show candidates"}))
            expect(screen.getByText("Rowan Scott")).toBeVisible()

            await user.click(screen.getByRole("button", {name: "Hide candidates"}))
            expect(screen.getByText("Rowan Scott")).not.toBeVisible()
        })

        it("points the toggle at the list it controls", () => {
            mount(newStore())

            const toggle = screen.getByRole("button")
            const list = screen.getByText("Rowan Scott").closest(".slate-candidate-list")
            expect(toggle).toHaveAttribute("aria-controls", list?.id)
        })

        it("keeps the voter's toggle when the screen is shown again", async () => {
            const user = userEvent.setup()
            const store = newStore()
            const first = mount(store, {defaultExpanded: false})

            await user.click(screen.getByRole("button", {name: "Show candidates"}))
            first.unmount()
            mount(store, {defaultExpanded: false})

            expect(screen.getByRole("button", {name: "Hide candidates"})).toBeInTheDocument()
            expect(screen.getByText("Rowan Scott")).toBeVisible()
        })

        it("does not touch the ballot selections when toggled", async () => {
            const user = userEvent.setup()
            const store = newStore()
            const before = store.getState().ballotSelections
            mount(store)

            await user.click(screen.getByRole("button"))

            expect(store.getState().ballotSelections).toBe(before)
        })
    })
})
