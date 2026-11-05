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

it("selects and removes a known choice without a creation callback", async () => {
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
    const input = screen.getByRole("combobox", {name: "Labels"})
    await user.click(input)
    await user.click(screen.getByRole("option", {name: "known"}))
    expect(onChange).toHaveBeenLastCalledWith(["known"])
    await user.click(input)
    await user.keyboard("{Backspace}")
    expect(onChange).toHaveBeenLastCalledWith([])
    expect(onCreate).not.toHaveBeenCalled()
})

it("preserves initial selections while creating several distinct labels", async () => {
    const user = userEvent.setup()
    const onChange = jest.fn()
    render(
        <CustomAutocompleteArrayInput
            label="Labels"
            defaultValue={["retained"]}
            onChange={onChange}
        />
    )
    await user.type(screen.getByRole("combobox", {name: "Labels"}), "second third second{Enter}")
    expect(onChange).toHaveBeenLastCalledWith(["retained", "second", "third"])
    expect((screen.getByRole("combobox", {name: "Labels"}) as HTMLInputElement).value).toBe("")
})

it("uses asynchronously supplied choices while retaining labels created locally", async () => {
    const user = userEvent.setup()
    const onCreate = jest.fn()
    const onChange = jest.fn()
    const props = {label: "Labels", onCreate, onChange}
    const {rerender} = render(<CustomAutocompleteArrayInput {...props} />)
    const input = screen.getByRole("combobox", {name: "Labels"})
    await user.type(input, "local{Enter}")
    expect(onCreate).toHaveBeenCalledWith("local")
    onCreate.mockClear()
    rerender(
        <CustomAutocompleteArrayInput {...props} choices={[{id: "remote-1", name: "known"}]} />
    )
    await user.type(input, "known{Enter}")
    expect(onCreate).not.toHaveBeenCalled()
    expect(onChange).toHaveBeenLastCalledWith(["local", "known"])
    // Removing a selected chip does not erase its locally created option.
    await user.keyboard("{Backspace}{Backspace}")
    await user.click(input)
    expect(screen.getByRole("option", {name: "local"})).toBeTruthy()
    expect(screen.getByRole("option", {name: "known"})).toBeTruthy()
})

it("retains a missing option when an initial selection is entered again", async () => {
    const user = userEvent.setup()
    const onCreate = jest.fn()
    const onChange = jest.fn()
    render(
        <CustomAutocompleteArrayInput
            label="Labels"
            defaultValue={["retained"]}
            onCreate={onCreate}
            onChange={onChange}
        />
    )
    const input = screen.getByRole("combobox", {name: "Labels"})
    await user.type(input, "retained retained{Enter}")
    expect(onChange).toHaveBeenLastCalledWith(["retained"])
    expect(onCreate).not.toHaveBeenCalled()
    // Removing the chip must leave the entered label available for reselection,
    // even though it was already selected when its missing option was added.
    await user.keyboard("{Backspace}")
    await user.click(input)
    expect(screen.getAllByRole("option", {name: "retained"})).toHaveLength(1)
    await user.click(screen.getByRole("option", {name: "retained"}))
    expect(onChange).toHaveBeenLastCalledWith(["retained"])
    expect(onCreate).not.toHaveBeenCalled()
})
