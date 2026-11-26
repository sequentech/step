/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {fireEvent, render, screen} from "@testing-library/react"
import "@testing-library/jest-dom"
import {ETemplateButtonType, ITemplateButton} from "@/types/messaging"
import {TemplateButtonsInput} from "./TemplateButtonsInput"

jest.mock("react-i18next", () => ({
    useTranslation: () => ({t: (key: string) => key, i18n: {language: "en"}}),
}))

const DEFAULTS: ITemplateButton[] = [
    {type: ETemplateButtonType.URL, parameter: "{{code}}"},
    {type: ETemplateButtonType.QUICK_REPLY, parameter: "NA"},
]

const renderInput = (buttons: ITemplateButton[] | null | undefined, disabled = false) => {
    const onChange = jest.fn()
    render(
        <TemplateButtonsInput
            buttons={buttons}
            defaults={DEFAULTS}
            disabled={disabled}
            onChange={onChange}
        />
    )
    return onChange
}

const values = () =>
    screen
        .queryAllByLabelText("messaging.templateButtons.parameter")
        .map((input) => (input as HTMLInputElement).value)

it("shows the default buttons until the template sets its own", () => {
    const onChange = renderInput(undefined)
    expect(screen.getByLabelText("messaging.templateButtons.choice.DEFAULT")).toBeChecked()
    expect(values()).toEqual([])
    expect(screen.getByText(/\{\{code\}\}/)).toBeInTheDocument()
    expect(screen.getByText(/NA/)).toBeInTheDocument()
    // Setting them starts from the defaults.
    fireEvent.click(screen.getByLabelText("messaging.templateButtons.choice.CUSTOM"))
    expect(onChange).toHaveBeenCalledWith(DEFAULTS)
})

it("edits the template's own buttons in order, and goes back to the defaults", () => {
    const onChange = renderInput([
        {type: ETemplateButtonType.URL, parameter: "{{code}}"},
        {type: ETemplateButtonType.QUICK_REPLY, parameter: "NA"},
    ])
    expect(screen.getByLabelText("messaging.templateButtons.choice.CUSTOM")).toBeChecked()
    expect(values()).toEqual(["{{code}}", "NA"])
    fireEvent.change(screen.getAllByLabelText("messaging.templateButtons.parameter")[1], {
        target: {value: "SKIP"},
    })
    expect(onChange).toHaveBeenLastCalledWith([
        {type: ETemplateButtonType.URL, parameter: "{{code}}"},
        {type: ETemplateButtonType.QUICK_REPLY, parameter: "SKIP"},
    ])
    fireEvent.click(screen.getAllByRole("button", {name: "messaging.templateButtons.remove"})[0])
    expect(onChange).toHaveBeenLastCalledWith([
        {type: ETemplateButtonType.QUICK_REPLY, parameter: "NA"},
    ])
    fireEvent.click(screen.getByRole("button", {name: "messaging.templateButtons.add"}))
    expect(onChange).toHaveBeenLastCalledWith([
        ...DEFAULTS,
        {type: ETemplateButtonType.QUICK_REPLY, parameter: ""},
    ])
    fireEvent.click(screen.getByLabelText("messaging.templateButtons.choice.DEFAULT"))
    expect(onChange).toHaveBeenLastCalledWith(null)
})

it("sends no buttons when the template's list is empty, and flags a blank value", () => {
    renderInput([])
    expect(screen.getByText("messaging.templateButtons.none")).toBeInTheDocument()
    expect(values()).toEqual([])
})

it("marks a button without a value", () => {
    renderInput([{type: ETemplateButtonType.QUICK_REPLY, parameter: " "}])
    expect(screen.getByLabelText("messaging.templateButtons.parameter")).toHaveAttribute(
        "aria-invalid",
        "true"
    )
})

it("can't be changed when disabled", () => {
    renderInput(DEFAULTS, true)
    expect(screen.queryByRole("button", {name: "messaging.templateButtons.add"})).toBeNull()
    expect(screen.getAllByLabelText("messaging.templateButtons.parameter")[0]).toBeDisabled()
})
