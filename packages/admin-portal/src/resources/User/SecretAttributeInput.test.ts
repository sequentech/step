/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useState} from "react"
import {fireEvent, render, screen} from "@testing-library/react"
import {SecretAttributeInput, SecretAttributeInputProps} from "./SecretAttributeInput"

jest.mock("@/components/styles/FormStyles", () => ({
    FormStyles: {TextField: require("@mui/material").TextField},
}))

const labels = {
    reveal: "Reveal",
    hide: "Hide",
    clear: "Clear",
    add: "Add value",
    remove: "Remove value",
}
const setup = (overrides: Partial<SecretAttributeInputProps> = {}) => {
    const onChange = jest.fn()
    const onReveal = jest.fn()
    const props: SecretAttributeInputProps = {
        label: "Reference",
        values: ["first", "second"],
        stored: true,
        editable: true,
        multivalued: true,
        canReveal: true,
        revealed: true,
        revealing: false,
        required: false,
        labels,
        onChange,
        onReveal,
        ...overrides,
    }
    function Form() {
        const [values, setValues] = useState(props.values)
        return React.createElement(SecretAttributeInput, {
            ...props,
            values,
            onChange: (next) => {
                setValues(next)
                onChange(next)
            },
        })
    }
    render(React.createElement(Form))
    return {onChange, onReveal}
}

it("replaces one value without discarding the other values", () => {
    const {onChange} = setup()
    fireEvent.change(screen.getAllByLabelText("Reference")[0], {target: {value: "changed"}})
    expect(onChange).toHaveBeenLastCalledWith(["changed", "second"])
})

it("clears an unrevealed stored value with write permission alone", () => {
    const {onChange, onReveal} = setup({values: [], canReveal: false, revealed: false})
    const clear = screen.getByRole("button", {name: "Clear"})
    expect(clear.textContent).toBe("")
    expect(clear.closest(".MuiInputAdornment-positionEnd")).not.toBeNull()
    fireEvent.click(clear)
    expect(onChange).toHaveBeenCalledWith([])
    expect(onReveal).not.toHaveBeenCalled()
})

it("adds and removes individual values", () => {
    const {onChange} = setup()
    fireEvent.click(screen.getByRole("button", {name: "Add value"}))
    expect(onChange).toHaveBeenLastCalledWith(["first", "second", ""])
    fireEvent.click(screen.getAllByRole("button", {name: "Remove value"})[0])
    expect(onChange).toHaveBeenLastCalledWith(["second", ""])
})

it("allows reveal without write permission and offers no edit actions", () => {
    const {onReveal} = setup({values: [], editable: false, revealed: false})
    fireEvent.click(screen.getByRole("button", {name: "Reveal"}))
    expect(onReveal).toHaveBeenCalledTimes(1)
    expect(screen.queryByRole("button", {name: "Clear"})).toBeNull()
    expect(screen.queryByRole("button", {name: "Add value"})).toBeNull()
})

it("locks input and actions while reveal is pending", () => {
    const {onChange, onReveal} = setup({revealing: true})
    expect((screen.getAllByLabelText("Reference")[0] as HTMLInputElement).disabled).toBe(true)
    fireEvent.click(screen.getByRole("button", {name: "Clear"}))
    fireEvent.click(screen.getByRole("button", {name: "Hide"}))
    expect(onChange).not.toHaveBeenCalled()
    expect(onReveal).not.toHaveBeenCalled()
})
