/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {render, screen} from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import "@testing-library/jest-dom"
import {ThemeProvider} from "@mui/material/styles"
import {ECandidatesIconCheckboxPolicy} from "@sequentech/ui-core"
import theme from "../../services/theme"
import Candidate from "./Candidate"

jest.mock(
    "@sequentech/ui-core",
    () => ({
        ECandidatesIconCheckboxPolicy: {
            ROUND_CHECKBOX: "ROUND_CHECKBOX",
            SQUARE_CHECKBOX: "SQUARE_CHECKBOX",
        },
    }),
    {virtual: true}
)
jest.mock("react-i18next", () => ({useTranslation: () => ({t: (key: string) => key})}))

const show = (props: Partial<React.ComponentProps<typeof Candidate>>) =>
    render(
        <ThemeProvider theme={theme}>
            <ul>
                <Candidate title="Ada" isSelectable checked={false} {...props} />
            </ul>
        </ThemeProvider>
    )

it.each([
    ECandidatesIconCheckboxPolicy.SQUARE_CHECKBOX,
    ECandidatesIconCheckboxPolicy.ROUND_CHECKBOX,
])("a %s click sends one selection change", async (iconCheckboxPolicy) => {
    const setChecked = jest.fn()
    show({iconCheckboxPolicy, setChecked})
    await userEvent.click(screen.getByRole("checkbox", {name: "Ada"}))
    expect(setChecked).toHaveBeenCalledTimes(1)
    expect(setChecked).toHaveBeenLastCalledWith(true)
})

it("opening and choosing a rank does not also select the candidate row", async () => {
    const setChecked = jest.fn()
    const handlePreferentialChange = jest.fn()
    show({isPreferentialVote: true, totalCandidates: 3, setChecked, handlePreferentialChange})
    await userEvent.click(screen.getByRole("combobox", {name: /Ada/}))
    await userEvent.click(screen.getByRole("option", {name: /^2/}))
    expect(handlePreferentialChange).toHaveBeenCalledTimes(1)
    expect(handlePreferentialChange).toHaveBeenLastCalledWith(2)
    expect(setChecked).not.toHaveBeenCalled()
})

it("clicking the candidate name still selects the row", async () => {
    const setChecked = jest.fn()
    show({setChecked})
    await userEvent.click(screen.getByText("Ada"))
    expect(setChecked).toHaveBeenCalledTimes(1)
    expect(setChecked).toHaveBeenLastCalledWith(true)
})
