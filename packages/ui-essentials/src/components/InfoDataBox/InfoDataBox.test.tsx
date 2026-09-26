/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import "@testing-library/jest-dom"
import {render, screen} from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import InfoDataBox from "./InfoDataBox"

test("keyboard users can reach scrollable ballot data", async () => {
    const user = userEvent.setup()
    render(
        <>
            <InfoDataBox>Auditable ballot data</InfoDataBox>
            <button>Continue</button>
        </>
    )
    await user.tab()
    expect(screen.getByText("Auditable ballot data")).toHaveFocus()
    await user.tab()
    expect(screen.getByRole("button", {name: "Continue"})).toHaveFocus()
})
