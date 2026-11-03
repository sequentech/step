// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {ThemeProvider} from "@mui/material/styles"
import useMediaQuery from "@mui/material/useMediaQuery"
import {render, screen} from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import {theme} from "@sequentech/ui-essentials"
import {I18nextProvider} from "react-i18next"
import type {BallotSelection} from "@sequentech/ui-core"

import {i18n, marks} from "../../testing/ballotHarness"
import {PRESIDENT, TRUSTEES} from "../../testing/slateFixtures"
import {SlateOfficeNavigator} from "./SlateOfficeNavigator"

jest.mock("@mui/material/useMediaQuery")
const mockedUseMediaQuery = jest.mocked(useMediaQuery)

const SELECTION: BallotSelection = [
    marks(PRESIDENT, {"f-president": 0}),
    marks(TRUSTEES, {"f-t1": 0}),
]

const Navigator: React.FC<{initial?: number}> = ({initial = 0}) => {
    const [office, setOffice] = React.useState(initial)
    return (
        <SlateOfficeNavigator
            contests={[PRESIDENT, TRUSTEES]}
            selection={SELECTION}
            office={office}
            onOfficeChange={setOffice}
            renderContest={(contest) => <h2>{contest.name}</h2>}
        />
    )
}

const mount = (initial?: number) =>
    render(
        <I18nextProvider i18n={i18n}>
            <ThemeProvider theme={theme}>
                <Navigator initial={initial} />
            </ThemeProvider>
        </I18nextProvider>
    )

beforeEach(() => mockedUseMediaQuery.mockReturnValue(false))

describe("the office navigator", () => {
    it("shows one office at a time, with its place among the offices", () => {
        mount()

        expect(screen.getByRole("heading", {name: "President"})).toBeVisible()
        expect(screen.getByRole("heading", {name: "Trustees", hidden: true})).not.toBeVisible()
        expect(screen.getByText("Office 1 of 2")).toBeVisible()
    })

    it("lists every office with how many of its choices are taken", () => {
        mount()

        const nav = screen.getByRole("navigation", {name: "Offices"})
        expect(nav).toHaveTextContent("President1/1")
        expect(nav).toHaveTextContent("Trustees1/3")
        expect(screen.getByRole("button", {name: /President/})).toHaveAttribute(
            "aria-current",
            "true"
        )
    })

    it("moves between offices from the list and with Previous and Next office", async () => {
        mount()

        expect(screen.getByRole("button", {name: "Previous office"})).toBeDisabled()
        await userEvent.click(screen.getByRole("button", {name: "Next office"}))
        expect(screen.getByRole("heading", {name: "Trustees"})).toBeVisible()
        expect(screen.getByText("Last office")).toBeVisible()
        expect(screen.queryByRole("button", {name: "Next office"})).toBeNull()

        await userEvent.click(screen.getByRole("button", {name: /President/}))
        expect(screen.getByRole("heading", {name: "President"})).toBeVisible()
    })

    it("offers a list of offices to pick from on a phone", async () => {
        mockedUseMediaQuery.mockReturnValue(true)
        mount()

        expect(screen.queryByRole("navigation", {name: "Offices"})).toBeNull()
        await userEvent.selectOptions(screen.getByRole("combobox", {name: "Office"}), "1")
        expect(screen.getByRole("heading", {name: "Trustees"})).toBeVisible()
        expect(screen.getByRole("option", {name: "Trustees · 1/3"})).toBeInTheDocument()
    })
})
