// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * One row on a ballot: a name, a way to choose it, and nothing else.
 *
 * The most-rendered component in the platform and, until now, one with no test.
 * It draws every option a voter meets — candidates, blank votes, decline-to-vote,
 * write-in lines — and it changes shape on five props: `isSelectable`,
 * `isPreferentialVote`, `iconCheckboxPolicy`, `isWriteIn` and `shouldDisable`.
 *
 * It is also the component the Election Architect's ballot preview draws, today
 * through a byte-identical copy kept honest by a parity checker, and shortly by
 * importing this file. Both reasons point the same way: what this renders is what
 * an election manager approves and what a voter is handed, so it should be pinned
 * before it is shared more widely.
 *
 * Characterisation, not specification — see `Question.test.tsx` for the argument.
 */

import {render, screen} from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import {ThemeProvider} from "@mui/material/styles"
import {I18nextProvider} from "react-i18next"
import i18next from "i18next"

import Candidate from "./Candidate"
import {theme} from "../../services/theme"
import englishTranslation from "../../translations/en"
import {ECandidatesIconCheckboxPolicy} from "@sequentech/ui-core"

const i18n = i18next.createInstance()
void i18n.init({
    lng: "en",
    fallbackLng: "en",
    resources: {
        en: {translation: englishTranslation.translations as Record<string, unknown>},
    },
    interpolation: {escapeValue: false},
})

const show = (props: Record<string, unknown>) =>
    render(
        <I18nextProvider i18n={i18n}>
            <ThemeProvider theme={theme}>
                <ul>
                    <Candidate title="Alice Okonjo" {...props} />
                </ul>
            </ThemeProvider>
        </I18nextProvider>
    )

describe("a candidate a voter can choose", () => {
    it("shows the name, and a box to tick", () => {
        show({isSelectable: true})

        expect(screen.getByText("Alice Okonjo")).toBeInTheDocument()
        expect(screen.getByRole("checkbox")).toBeInTheDocument()
    })

    it("shows a description under the name when there is one", () => {
        show({isSelectable: true, description: "Steward, Local 12"})
        expect(screen.getByText("Steward, Local 12")).toBeInTheDocument()
    })

    it("reports a tick to whoever owns the selection", async () => {
        const setChecked = jest.fn()
        show({isSelectable: true, setChecked})

        await userEvent.click(screen.getByRole("checkbox"))
        expect(setChecked).toHaveBeenCalledWith(true)
    })

    it("reports untick as well as tick", async () => {
        const setChecked = jest.fn()
        show({isSelectable: true, checked: true, setChecked})

        await userEvent.click(screen.getByRole("checkbox"))
        expect(setChecked).toHaveBeenCalledWith(false)
    })

    it("draws a round box where the contest asks for one", () => {
        // The round checkbox is how a single-choice contest signals "pick one" —
        // it is a presentation policy, not a `radio` input, so the role stays
        // `checkbox` and only the icon changes. Pinned because a test looking for
        // `role="radio"` would fail while the ballot is correct.
        show({
            isSelectable: true,
            iconCheckboxPolicy: ECandidatesIconCheckboxPolicy.ROUND_CHECKBOX,
        })

        expect(screen.getByRole("checkbox")).toBeInTheDocument()
    })

    it("goes quiet when it must not be chosen", () => {
        // `shouldDisable` is what the over-vote-and-disable policy reaches for
        // once a voter is at the limit.
        show({isSelectable: true, shouldDisable: true})
        expect(screen.getByRole("checkbox")).toBeDisabled()
    })

    it("offers nothing to click when it is not selectable", () => {
        // The review screen renders rows this way: readable, not markable.
        show({isSelectable: false})

        expect(screen.queryByRole("checkbox")).toBeNull()
        expect(screen.getByText("Alice Okonjo")).toBeInTheDocument()
    })
})

describe("a ranked contest's row", () => {
    it("offers a position to choose instead of a box", () => {
        show({
            isSelectable: true,
            isPreferentialVote: true,
            totalCandidates: 3,
            maxVotes: 3,
        })

        expect(screen.queryByRole("checkbox")).toBeNull()
        expect(screen.getByRole("combobox")).toBeInTheDocument()
    })

    it("shows the position already given, on review", () => {
        // `selectedPosition` is 1-based here — the ordinal a voter reads — while
        // the plan stores a 0-based rank. The conversion happens in the caller,
        // and mixing them up shows every voter a ballot off by one.
        show({
            isSelectable: false,
            isPreferentialVote: true,
            totalCandidates: 3,
            maxVotes: 3,
            selectedPosition: 2,
        })

        expect(screen.getByText(/2/)).toBeInTheDocument()
    })
})

describe("a write-in row", () => {
    it("gives the voter somewhere to type a name", () => {
        show({isSelectable: true, isWriteIn: true, writeInValue: ""})
        expect(screen.getByRole("textbox")).toBeInTheDocument()
    })

    it("reports what was typed", async () => {
        const setWriteInText = jest.fn()
        show({
            isSelectable: true,
            isWriteIn: true,
            writeInValue: "",
            setWriteInText,
        })

        await userEvent.type(screen.getByRole("textbox"), "D")
        expect(setWriteInText).toHaveBeenCalledWith("D")
    })

    it("shows the name already written in", () => {
        show({
            isSelectable: true,
            isWriteIn: true,
            writeInValue: "Dara Quinn",
        })
        expect(screen.getByRole("textbox")).toHaveValue("Dara Quinn")
    })
})

describe("what else a row can carry", () => {
    it("links out to a page about the candidate when given one", () => {
        // `url` draws a *link*, not an image — worth pinning, because the
        // Election Architect's preview passed candidate photographs to this prop
        // and got a "More information" link where a face should have been.
        show({isSelectable: true, url: "https://example.org/alice"})

        const link = screen.getByRole("link")
        expect(link).toHaveAttribute("href", "https://example.org/alice")
    })

    it("draws no link when there is no page", () => {
        show({isSelectable: true})
        expect(screen.queryByRole("link")).toBeNull()
    })

    it("renders whatever it is given as children, which is where a photograph goes", () => {
        show({
            isSelectable: true,
            children: <img alt="Alice Okonjo" src="data:image/png;base64,iVBORw0KGgo=" />,
        })

        expect(screen.getByRole("img", {name: "Alice Okonjo"})).toBeInTheDocument()
    })
})
