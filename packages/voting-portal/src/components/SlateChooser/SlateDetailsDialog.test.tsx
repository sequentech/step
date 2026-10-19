// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {configureStore} from "@reduxjs/toolkit"
import {ThemeProvider} from "@mui/material/styles"
import useMediaQuery from "@mui/material/useMediaQuery"
import {act, render, screen, waitFor, within} from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import {theme} from "@sequentech/ui-essentials"
import type {IBallotStyle as IElectionDTO, IContest} from "@sequentech/ui-core"
import {I18nextProvider} from "react-i18next"
import {Provider} from "react-redux"

import {i18n} from "../../testing/ballotHarness"
import {aSlateBallotStyle, PRESIDENT, SLATES, TRUSTEES} from "../../testing/slateFixtures"
import {resolveBallotStyleSlates} from "../../services/Slates"
import ballotSelectionsReducer, {
    resetBallotSelection,
    setBallotSelectionVoteChoice,
} from "../../store/ballotSelections/ballotSelectionsSlice"
import type {IBallotStyle} from "../../store/ballotStyles/ballotStylesSlice"
import extraReducer from "../../store/extra/extraSlice"
import {SlateSelectionChooser} from "./SlateSelectionChooser"

jest.mock("@mui/material/useMediaQuery")
const mockedUseMediaQuery = jest.mocked(useMediaQuery)

const BALLOT_STYLE = aSlateBallotStyle(JSON.stringify(SLATES))

const mount = (ballotStyle: IBallotStyle = BALLOT_STYLE, onEditSelections = jest.fn()) => {
    const store = configureStore({
        reducer: {extra: extraReducer, ballotSelections: ballotSelectionsReducer},
    })
    store.dispatch(resetBallotSelection({ballotStyle, force: true}))
    const slates = resolveBallotStyleSlates(ballotStyle.ballot_eml)
    if (!slates) {
        throw new Error("no slates")
    }
    render(
        <Provider store={store}>
            <I18nextProvider i18n={i18n}>
                <ThemeProvider theme={theme}>
                    <SlateSelectionChooser
                        ballotStyle={ballotStyle}
                        slates={slates}
                        defaultLanguage="en"
                        onEditSelections={onEditSelections}
                    />
                </ThemeProvider>
            </I18nextProvider>
        </Provider>
    )
    const choose = (contestId: string, candidateId: string) =>
        act(() => {
            store.dispatch(
                setBallotSelectionVoteChoice({
                    ballotStyle,
                    contestId,
                    voteChoice: {id: candidateId, selected: 0},
                })
            )
        })
    const selected = (): string[] =>
        (store.getState().ballotSelections[ballotStyle.election_id] ?? []).flatMap((contest) =>
            contest.choices.filter((choice) => choice.selected > -1).map((choice) => choice.id)
        )
    return {choose, selected, onEditSelections}
}

const VIEW_FORWARD = "View slate Forward Together"

const openForward = async () => {
    await userEvent.click(screen.getByRole("button", {name: VIEW_FORWARD}))
    return screen.getByRole("dialog", {name: "Forward Together"})
}

const officesOf = (dialog: HTMLElement) =>
    Array.from(dialog.querySelectorAll(".slate-details-office")).map((office) => [
        office.querySelector(".slate-details-office-name")?.textContent,
        Array.from(office.querySelectorAll(".slate-details-candidate")).map(
            (name) => name.textContent
        ),
    ])

beforeEach(() => mockedUseMediaQuery.mockReturnValue(false))

describe("viewing a slate", () => {
    it("is offered on every slate card", () => {
        mount()

        for (const name of ["Forward Together", "Members First", "Independent Voices"]) {
            const button = screen.getByRole("button", {name: `View slate ${name}`})
            expect(button).toHaveClass("slate-view-button")
            expect(button.closest(".slate-card")).not.toBeNull()
        }
    })

    it("opens a dialog with the slate's coverage, count and candidates by office", async () => {
        mount()

        const dialog = await openForward()

        expect(within(dialog).getByText("Partial slate · 3 candidates")).toBeVisible()
        expect(officesOf(dialog)).toEqual([
            ["President", ["Jordan Ellis"]],
            ["Trustees", ["Rowan Scott", "Charlie Kim"]],
        ])
        expect(within(dialog).getByRole("button", {name: "Back"})).toBeVisible()
        expect(within(dialog).getByRole("button", {name: "Select this slate"})).toBeVisible()
    })

    it("lists only the offices of the slate", async () => {
        mount()

        await userEvent.click(screen.getByRole("button", {name: "View slate Independent Voices"}))
        const dialog = screen.getByRole("dialog", {name: "Independent Voices"})

        expect(within(dialog).getByText("Trustees only · 1 candidate")).toBeVisible()
        expect(officesOf(dialog)).toEqual([["Trustees", ["Harper Lane"]]])
    })

    it("lists offices and candidates in the order the ballot shows them", async () => {
        const president = {...PRESIDENT, presentation: {sort_order: 1}} as IContest
        const trustees = {
            ...TRUSTEES,
            presentation: {sort_order: 0, candidates_order: "custom"},
            candidates: TRUSTEES.candidates.map((candidate, index) => ({
                ...candidate,
                presentation: {sort_order: TRUSTEES.candidates.length - index},
            })),
        } as IContest
        const ballotEml: IElectionDTO = {
            ...BALLOT_STYLE.ballot_eml,
            contests: [president, trustees],
        }
        mount({...BALLOT_STYLE, ballot_eml: ballotEml})

        const dialog = await openForward()

        expect(officesOf(dialog)).toEqual([
            ["Trustees", ["Charlie Kim", "Rowan Scott"]],
            ["President", ["Jordan Ellis"]],
        ])
    })

    it.each([
        ["Back", async () => userEvent.click(screen.getByRole("button", {name: "Back"}))],
        ["Escape", async () => userEvent.keyboard("{Escape}")],
        [
            "the close button",
            async () =>
                userEvent.click(
                    document.querySelector(".slate-details-dialog .dialog-close-button") as Element
                ),
        ],
    ])("closes with %s, changes nothing and gives the focus back", async (_name, close) => {
        const {selected} = mount()
        await openForward()

        await close()

        await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull())
        expect(selected()).toEqual([])
        await waitFor(() => expect(screen.getByRole("button", {name: VIEW_FORWARD})).toHaveFocus())
    })

    it("selects the slate without asking on an empty ballot", async () => {
        const {selected} = mount()
        await openForward()

        await userEvent.click(screen.getByRole("button", {name: "Select this slate"}))

        expect(selected()).toEqual(["f-president", "f-t1", "f-t2"])
        expect(
            screen.queryByRole("dialog", {name: "Replace these selections?"})
        ).not.toBeInTheDocument()
    })

    it("asks before replacing choices, and Keep my selections changes nothing", async () => {
        const {selected, choose} = mount()
        choose("president", "i-president")
        await openForward()

        await userEvent.click(screen.getByRole("button", {name: "Select this slate"}))
        const confirm = await screen.findByRole("dialog", {name: "Replace these selections?"})
        expect(confirm).toBeVisible()
        await userEvent.click(within(confirm).getByRole("button", {name: "Keep my selections"}))

        expect(selected()).toEqual(["i-president"])
    })

    it("replaces the choices once the voter confirms", async () => {
        const {selected, choose} = mount()
        choose("president", "i-president")
        await openForward()

        await userEvent.click(screen.getByRole("button", {name: "Select this slate"}))
        const confirm = await screen.findByRole("dialog", {name: "Replace these selections?"})
        await userEvent.click(within(confirm).getByRole("button", {name: "Replace president"}))

        expect(selected()).toEqual(["f-president", "f-t1", "f-t2"])
    })

    it("offers Edit selections when the ballot is exactly that slate", async () => {
        const {choose, onEditSelections} = mount()
        choose("president", "f-president")
        choose("trustees", "f-t1")
        choose("trustees", "f-t2")
        const dialog = await openForward()

        expect(within(dialog).queryByRole("button", {name: "Select this slate"})).toBeNull()
        await userEvent.click(within(dialog).getByRole("button", {name: "Edit selections"}))

        expect(onEditSelections).toHaveBeenCalledWith(expect.objectContaining({id: "forward"}))
    })
})
