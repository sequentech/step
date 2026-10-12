/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {render, screen} from "@testing-library/react"
import "@testing-library/jest-dom"
import {ThemeProvider} from "@mui/material/styles"
import Candidate from "./Candidate"
import {theme} from "../../services/theme"

jest.mock("../LinkBehavior/LinkBehavior", () => "a")
jest.mock(
    "@sequentech/ui-core",
    () => jest.requireActual("../../../../ui-core/src/types/ContestPresentation"),
    {virtual: true}
)
jest.mock("react-i18next", () => ({
    useTranslation: () => ({t: (key: string): string => key}),
}))

const show = (props: Partial<React.ComponentProps<typeof Candidate>>) =>
    render(
        <ThemeProvider theme={theme}>
            <ul>
                <Candidate title="Alice Okonjo" isSelectable={true} {...props} />
            </ul>
        </ThemeProvider>
    )

describe("the slate line of a candidate", () => {
    it("shows its slate under the name and names every control with both", () => {
        show({subtitle: "Independent Voices"})

        expect(screen.getByText("Independent Voices")).toHaveClass("candidate-subtitle")
        expect(
            screen.getByRole("checkbox", {name: "Alice Okonjo Independent Voices"})
        ).toBeInTheDocument()
    })

    it("draws no slate line and keeps the name as the label without one", () => {
        const {container} = show({})

        expect(container.querySelector(".candidate-subtitle")).toBeNull()
        expect(screen.getByRole("checkbox", {name: "Alice Okonjo"})).toBeInTheDocument()
    })
})
