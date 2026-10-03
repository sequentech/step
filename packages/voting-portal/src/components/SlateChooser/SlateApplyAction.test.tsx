// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {combineReducers, configureStore} from "@reduxjs/toolkit"
import {ThemeProvider} from "@mui/material/styles"
import {act, render, screen, within} from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import {EMobileCandidateLists} from "@sequentech/ui-core"
import i18next from "i18next"
import {I18nextProvider} from "react-i18next"
import {Provider} from "react-redux"
import theme from "../../../../ui-essentials/src/services/theme"
import {
    buildSlate,
    buildSlates,
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
import englishTranslation from "../../translations/en"
import {SlateApplyAction} from "./SlateApplyAction"
import {SlateChooser} from "./SlateChooser"

const i18n = i18next.createInstance()
void i18n.init({
    lng: "en",
    fallbackLng: "en",
    resources: {en: {translation: englishTranslation.translations}},
    interpolation: {escapeValue: false},
})

const ballot = buildSlatesBallot()
const ballotStyle = buildSlatesBallotStyle(ballot)

const mount = (slateId: string) => {
    const store = configureStore({
        reducer: combineReducers({ballotSelections: ballotSelectionsReducer}),
    })
    store.dispatch(resetBallotSelection({ballotStyle, force: true}))
    render(
        <I18nextProvider i18n={i18n}>
            <ThemeProvider theme={theme}>
                <Provider store={store}>
                    <SlateApplyAction
                        ballotStyle={ballotStyle}
                        slate={buildSlate(ballot, slateId)}
                    />
                </Provider>
            </ThemeProvider>
        </I18nextProvider>
    )
    const selected = (contestId: string): string[] =>
        (
            store
                .getState()
                .ballotSelections[
                    ballotStyle.election_id
                ]?.find((contest) => contest.contest_id === contestId)?.choices ?? []
        )
            .filter((choice) => choice.selected > -1)
            .map((choice) => choice.id)
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
    return {selected, mark}
}

describe("SlateApplyAction", () => {
    it("names the slate on its button", () => {
        mount("forward")
        expect(screen.getByRole("button", {name: "Choose slate Forward Together"})).toBeVisible()
    })

    it("marks the slate's candidates in every contest and says so", async () => {
        const {selected} = mount("forward")
        await userEvent.click(screen.getByRole("button", {name: "Choose slate Forward Together"}))
        expect(selected(PRESIDENT)).toEqual(["p-forward"])
        expect(selected(SECRETARY)).toEqual(["s-forward"])
        expect(selected(TRUSTEES)).toEqual(["t-forward-1", "t-forward-2", "t-forward-3"])
        expect(screen.queryByRole("dialog")).toBeNull()
        expect(screen.getByRole("status")).toHaveTextContent(
            "Forward Together chosen. Candidates selected: 5. Contests: 3."
        )
    })

    it("asks before removing choices and lists what changes", async () => {
        const {selected, mark} = mount("forward")
        mark(PRESIDENT, "p-independent")
        await userEvent.click(screen.getByRole("button", {name: "Choose slate Forward Together"}))
        const dialog = screen.getByRole("dialog")
        expect(within(dialog).getByText("President")).toBeVisible()
        expect(within(dialog).getByText("Renata Halloran")).toBeVisible()
        expect(within(dialog).getByText("Amara Lindqvist")).toBeVisible()
        expect(within(dialog).queryByText("Trustees")).toBeNull()
        expect(selected(PRESIDENT)).toEqual(["p-independent"])
        expect(selected(TRUSTEES)).toEqual([])
    })

    it("changes nothing when the voter keeps the current choices", async () => {
        const {selected, mark} = mount("forward")
        mark(PRESIDENT, "p-independent")
        await userEvent.click(screen.getByRole("button", {name: "Choose slate Forward Together"}))
        await userEvent.click(screen.getByRole("button", {name: "Keep my choices"}))
        expect(selected(PRESIDENT)).toEqual(["p-independent"])
        expect(selected(SECRETARY)).toEqual([])
        expect(await screen.findByRole("status")).toBeEmptyDOMElement()
    })

    it("replaces the choices once the voter confirms", async () => {
        const {selected, mark} = mount("forward")
        mark(PRESIDENT, "p-independent")
        await userEvent.click(screen.getByRole("button", {name: "Choose slate Forward Together"}))
        await userEvent.click(screen.getByRole("button", {name: "Replace choices"}))
        expect(selected(PRESIDENT)).toEqual(["p-forward"])
        expect(selected(TRUSTEES)).toEqual(["t-forward-1", "t-forward-2", "t-forward-3"])
    })

    it("does not ask when a partial slate only touches empty contests", async () => {
        const {selected, mark} = mount("independent-voices")
        mark(PRESIDENT, "p-independent")
        await userEvent.click(screen.getByRole("button", {name: "Choose slate Independent Voices"}))
        expect(screen.queryByRole("dialog")).toBeNull()
        expect(selected(PRESIDENT)).toEqual(["p-independent"])
        expect(selected(TRUSTEES)).toEqual(["t-voices-1", "t-voices-2"])
        expect(screen.getByRole("status")).toHaveTextContent(
            "Independent Voices chosen. Candidates selected: 2. Contests: 1."
        )
    })

    it("stops announcing the slate once its candidates are no longer the selection", async () => {
        const {mark} = mount("independent-voices")
        await userEvent.click(screen.getByRole("button", {name: "Choose slate Independent Voices"}))
        mark(TRUSTEES, "t-independent")
        expect(await screen.findByRole("status")).toBeEmptyDOMElement()
    })
})

describe("SlateApplyAction in the slate cards", () => {
    it("chooses the slate of the card it is in", async () => {
        const store = configureStore({
            reducer: combineReducers({ballotSelections: ballotSelectionsReducer}),
        })
        store.dispatch(resetBallotSelection({ballotStyle, force: true}))
        render(
            <I18nextProvider i18n={i18n}>
                <ThemeProvider theme={theme}>
                    <Provider store={store}>
                        <SlateChooser
                            slates={{
                                mobileCandidateLists: EMobileCandidateLists.COLLAPSED,
                                slates: buildSlates(ballot),
                            }}
                            defaultLanguage="en"
                            renderActions={(slate) => (
                                <SlateApplyAction ballotStyle={ballotStyle} slate={slate} />
                            )}
                        />
                    </Provider>
                </ThemeProvider>
            </I18nextProvider>
        )
        const card = screen.getByRole("heading", {level: 3, name: "Members First"}).closest("li")
        expect(card).not.toBeNull()
        await userEvent.click(
            within(card as HTMLElement).getByRole("button", {name: "Choose slate Members First"})
        )
        const selection = store.getState().ballotSelections[ballotStyle.election_id] ?? []
        expect(
            selection.flatMap((contest) =>
                contest.choices.filter((choice) => choice.selected > -1).map((choice) => choice.id)
            )
        ).toEqual(["p-members", "s-members", "t-members-1", "t-members-2", "t-members-3"])
    })
})
