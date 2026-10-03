// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {render, screen} from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import i18next from "i18next"
import {I18nextProvider} from "react-i18next"
import {ThemeProvider} from "@mui/material/styles"
import theme from "../../../../ui-essentials/src/services/theme"
import englishTranslation from "../../translations/en"
import {ESlateSelectionStatus} from "../../services/SlateSelection"
import {SlateSelectionStatus} from "./SlateSelectionStatus"
import {SlateMember} from "./SlateMember"
import {ESlateBallotTab, SlateBallotTabs} from "./SlateBallotTabs"

const i18n = i18next.createInstance()
void i18n.init({
    lng: "en",
    fallbackLng: "en",
    resources: {en: {translation: englishTranslation.translations}},
    interpolation: {escapeValue: false},
})

const mount = (ui: React.ReactElement) =>
    render(
        <ThemeProvider theme={theme}>
            <I18nextProvider i18n={i18n}>{ui}</I18nextProvider>
        </ThemeProvider>
    )

const summary = (status: ESlateSelectionStatus, selected: number, total: number) => ({
    status,
    selected,
    total,
    selectedMemberIds: [],
})

describe("SlateSelectionStatus", () => {
    it("shows nothing when no member is selected", () => {
        const {container} = mount(
            <SlateSelectionStatus summary={summary(ESlateSelectionStatus.NONE, 0, 7)} />
        )

        expect(container).toBeEmptyDOMElement()
    })

    it("says all members are selected", () => {
        mount(<SlateSelectionStatus summary={summary(ESlateSelectionStatus.ALL, 7, 7)} />)

        expect(screen.getByText("All 7 selected")).toHaveClass("slate-selection-status-all")
    })

    it("says the selection is mixed, with the count", () => {
        mount(<SlateSelectionStatus summary={summary(ESlateSelectionStatus.MIXED, 1, 7)} />)

        expect(screen.getByText("Mixed · 1 of 7 selected")).toHaveClass(
            "slate-selection-status-mixed"
        )
    })

    it("says the slate is partly selected, with the count", () => {
        mount(<SlateSelectionStatus summary={summary(ESlateSelectionStatus.PARTLY, 2, 3)} />)

        expect(screen.getByText("Partly selected · 2 of 3")).toHaveClass(
            "slate-selection-status-partly"
        )
    })
})

describe("SlateMember", () => {
    it("marks a selected member with a class and a label for screen readers", () => {
        mount(
            <ul>
                <SlateMember selected>Rowan Scott</SlateMember>
            </ul>
        )

        const member = screen.getByRole("listitem")
        expect(member).toHaveClass("slate-member-selected")
        expect(member).toHaveTextContent("Rowan Scott")
        expect(member).toHaveTextContent("Selected")
        expect(member.querySelector(".slate-member-check")).not.toBeNull()
    })

    it("leaves an unselected member unmarked", () => {
        mount(
            <ul>
                <SlateMember selected={false}>Charlie Kim</SlateMember>
            </ul>
        )

        const member = screen.getByRole("listitem")
        expect(member).not.toHaveClass("slate-member-selected")
        expect(member).not.toHaveTextContent("Selected")
        expect(member.querySelector(".slate-member-check")).toBeNull()
    })
})

describe("SlateBallotTabs", () => {
    const Harness = ({initial = ESlateBallotTab.SLATES}: {initial?: ESlateBallotTab}) => {
        const [tab, setTab] = React.useState(initial)
        return (
            <SlateBallotTabs
                value={tab}
                onChange={setTab}
                slates={<p>slate cards</p>}
                candidates={<p>contest list</p>}
            />
        )
    }

    it("offers the slates first and the individual candidates second", () => {
        mount(<Harness />)

        const tabs = screen.getAllByRole("tab")
        expect(tabs.map((tab) => tab.textContent)).toEqual([
            "Choose a slate",
            "Individual candidates",
        ])
        expect(tabs[0]).toHaveAttribute("aria-selected", "true")
        expect(screen.getByRole("tabpanel")).toHaveTextContent("slate cards")
    })

    it("switches to the individual candidates and back", async () => {
        const user = userEvent.setup()
        mount(<Harness />)

        await user.click(screen.getByRole("tab", {name: "Individual candidates"}))
        expect(screen.getByRole("tabpanel")).toHaveTextContent("contest list")

        await user.click(screen.getByRole("tab", {name: "Choose a slate"}))
        expect(screen.getByRole("tabpanel")).toHaveTextContent("slate cards")
    })

    it("keeps the hidden panel mounted so the contests keep their state", () => {
        mount(<Harness />)

        const hidden = screen.getByText("contest list").closest("[role=tabpanel]")
        expect(hidden).toHaveAttribute("hidden")
        expect(screen.getByText("contest list")).toBeInTheDocument()
    })

    it("moves focus to the selected tab when a control inside a panel switches it", async () => {
        const user = userEvent.setup()
        const Switching = () => {
            const [tab, setTab] = React.useState(ESlateBallotTab.SLATES)
            return (
                <SlateBallotTabs
                    value={tab}
                    onChange={setTab}
                    slates={
                        <button onClick={() => setTab(ESlateBallotTab.CANDIDATES)}>
                            Edit selections
                        </button>
                    }
                    candidates={<p>contest list</p>}
                />
            )
        }
        mount(<Switching />)

        await user.click(screen.getByRole("button", {name: "Edit selections"}))

        expect(screen.getByRole("tab", {name: "Individual candidates"})).toHaveFocus()
    })

    it("ties each tab to its panel", () => {
        mount(<Harness initial={ESlateBallotTab.CANDIDATES} />)

        const tab = screen.getByRole("tab", {name: "Individual candidates"})
        const panel = screen.getByRole("tabpanel")
        expect(tab).toHaveAttribute("aria-controls", panel.id)
        expect(panel).toHaveAttribute("aria-labelledby", tab.id)
    })
})
