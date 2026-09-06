/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {fireEvent, render, screen} from "@testing-library/react"
import "@testing-library/jest-dom"
import {faCircleQuestion} from "@fortawesome/free-solid-svg-icons"
import IconButton from "./IconButton"

jest.mock("../LinkBehavior/LinkBehavior", () => "a")

it("exposes a button hook while preserving legacy SVG classes and click behavior", () => {
    const onClick = jest.fn()
    render(
        <IconButton
            icon={faCircleQuestion}
            ariaLabel="Ballot ID help"
            className="legacy-help-icon"
            buttonClassName="ballot-id-help-button"
            onClick={onClick}
        />
    )
    const button = screen.getByRole("button", {name: "Ballot ID help"})

    expect(button).toHaveClass("icon-button", "ballot-id-help-button")
    expect(button.querySelector("svg")).toHaveClass("legacy-help-icon")
    expect(button.querySelector(".ballot-id-help-button")).toBeNull()
    fireEvent.click(button)
    expect(onClick).toHaveBeenCalledTimes(1)
})
