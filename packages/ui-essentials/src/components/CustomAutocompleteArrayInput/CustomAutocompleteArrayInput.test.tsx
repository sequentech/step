/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {render, screen} from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import CustomAutocompleteArrayInput from "./CustomAutocompleteArrayInput"

it.each(["new", "new new"])("creates each new label once when Enter submits %s", async (text) => {
    const user = userEvent.setup()
    const onCreate = jest.fn()
    const onChange = jest.fn()
    render(<CustomAutocompleteArrayInput label="Labels" onCreate={onCreate} onChange={onChange} />)
    await user.type(screen.getByRole("combobox", {name: "Labels"}), `${text}{Enter}`)
    expect(onChange).toHaveBeenLastCalledWith(["new"])
    expect(onCreate).toHaveBeenCalledTimes(1)
    expect(onCreate).toHaveBeenCalledWith("new")
})

it("selects an existing label without requesting its creation", async () => {
    const user = userEvent.setup()
    const onCreate = jest.fn()
    const onChange = jest.fn()
    render(
        <CustomAutocompleteArrayInput
            label="Labels"
            choices={[{id: "label-7", name: "known"}]}
            onCreate={onCreate}
            onChange={onChange}
        />
    )
    await user.type(screen.getByRole("combobox", {name: "Labels"}), "known{Enter}")
    expect(onChange).toHaveBeenLastCalledWith(["known"])
    expect(onCreate).not.toHaveBeenCalled()
})

it("does not create whitespace labels or allow edits while disabled", async () => {
    const user = userEvent.setup()
    const onCreate = jest.fn()
    const onChange = jest.fn()
    const {rerender} = render(
        <CustomAutocompleteArrayInput label="Labels" onCreate={onCreate} onChange={onChange} />
    )
    await user.type(screen.getByRole("combobox", {name: "Labels"}), "   {Enter}")
    expect(onCreate).not.toHaveBeenCalled()
    rerender(
        <CustomAutocompleteArrayInput
            label="Labels"
            defaultValue={["retained"]}
            disabled
            onCreate={onCreate}
            onChange={onChange}
        />
    )
    onChange.mockClear()
    const input = screen.getByRole("combobox", {name: "Labels"})
    expect((input as HTMLInputElement).disabled).toBe(true)
    await user.type(input, "ignored{Enter}")
    expect(onChange).not.toHaveBeenCalled()
    expect(onCreate).not.toHaveBeenCalled()
})
