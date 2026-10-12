/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {cleanup, render, screen} from "@testing-library/react"
import {ESlateProblemSeverity, ISlateProblem} from "@sequentech/ui-core"
import {SlatesWarnings} from "./SlatesWarnings"

jest.mock("@sequentech/ui-core", () => ({
    ESlateProblemSeverity: {ERROR: "error", WARNING: "warning"},
}))
jest.mock("react-i18next", () => ({
    useTranslation: () => ({t: (key: string) => key}),
}))

const warning: ISlateProblem = {
    severity: ESlateProblemSeverity.WARNING,
    code: "partial_slate_before_full_slate",
    path: "sequent.slates.slates[0]",
    message: "a partial slate is listed before a full slate",
}

describe("SlatesWarnings", () => {
    afterEach(cleanup)

    it("renders nothing without warnings", () => {
        const {container} = render(React.createElement(SlatesWarnings, {warnings: []}))

        expect(container.innerHTML).toBe("")
    })

    it("lists every warning in a warning alert", () => {
        render(
            React.createElement(SlatesWarnings, {
                warnings: [
                    warning,
                    {...warning, path: "sequent.slates.slates[1]", message: "second"},
                ],
            })
        )

        const alert = screen.getByRole("alert")
        expect(alert.className).toContain("MuiAlert-colorWarning")
        expect(alert.textContent).toContain("electionScreen.slates.warnings")
        expect(
            screen.getByText(
                "sequent.slates.slates[0]: a partial slate is listed before a full slate"
            )
        ).toBeTruthy()
        expect(screen.getByText("sequent.slates.slates[1]: second")).toBeTruthy()
    })
})
