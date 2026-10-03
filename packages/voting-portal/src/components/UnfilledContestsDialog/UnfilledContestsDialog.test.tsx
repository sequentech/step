// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {render, screen, within} from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import {ThemeProvider} from "@mui/material/styles"
import {IContest} from "@sequentech/ui-core"
import {theme} from "@sequentech/ui-essentials"
import UnfilledContestsDialog from "./UnfilledContestsDialog"
import {IUnfilledContest} from "../../services/UnfilledContests"

jest.mock("react-i18next", () => ({
    useTranslation: () => ({
        t: (key: string, values?: Record<string, unknown>): string =>
            values ? `${key} ${JSON.stringify(values)}` : key,
        i18n: {language: "es"},
    }),
}))

const KEY = "reviewScreen.unfilledContestsDialog"

const contest = (id: string, name: string, spanishName?: string): IContest =>
    ({
        id,
        name,
        candidates: [],
        name_i18n: spanishName ? {es: spanishName} : undefined,
    }) as unknown as IContest

const UNFILLED: Array<IUnfilledContest> = [
    {contest: contest("vp", "Vice President", "Vicepresidencia"), selected: 0, max: 1},
    {contest: contest("trustees", "Trustees"), selected: 2, max: 3},
]

const renderDialog = (open = true) => {
    const handleClose = jest.fn()
    render(
        <ThemeProvider theme={theme}>
            <UnfilledContestsDialog
                open={open}
                unfilledContests={UNFILLED}
                handleClose={handleClose}
            />
        </ThemeProvider>
    )
    return handleClose
}

describe("UnfilledContestsDialog", () => {
    it("names every unfilled contest with how many choices were used", () => {
        renderDialog()
        const dialog = screen.getByRole("dialog", {name: `${KEY}.title`})
        const items = within(dialog).getAllByRole("listitem")
        expect(items).toHaveLength(2)
        expect(items[0]).toHaveTextContent("Vicepresidencia")
        expect(items[0]).toHaveTextContent(`${KEY}.selected {"selected":0,"max":1}`)
        expect(items[0]).toHaveTextContent(`${KEY}.nothingSelected`)
        expect(items[1]).toHaveTextContent("Trustees")
        expect(items[1]).toHaveTextContent(`${KEY}.selected {"selected":2,"max":3}`)
        expect(items[1]).not.toHaveTextContent(`${KEY}.nothingSelected`)
    })

    it("continues only when the voter says so", async () => {
        const user = userEvent.setup()
        const handleClose = renderDialog()
        await user.click(screen.getByRole("button", {name: `${KEY}.cancel`}))
        expect(handleClose).toHaveBeenLastCalledWith(false)
        await user.click(screen.getByRole("button", {name: `${KEY}.ok`}))
        expect(handleClose).toHaveBeenLastCalledWith(true)
        expect(handleClose).toHaveBeenCalledTimes(2)
    })

    it("renders nothing while closed", () => {
        renderDialog(false)
        expect(screen.queryByRole("dialog")).toBeNull()
    })
})
