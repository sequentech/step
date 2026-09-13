/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import assert from "node:assert/strict"
import React, {createRef} from "react"
import {act, fireEvent, render, screen, waitFor} from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import "@testing-library/jest-dom"
import {ThemeProvider} from "@mui/material/styles"
import theme from "../../services/theme"
import CustomDropFile, {type DropFileProps} from "./CustomDropFile"

jest.mock(
    "@sequentech/ui-core",
    () => ({
        useForwardedRef: jest.requireActual<typeof import("../../../../ui-core/src/utils/ref")>(
            "../../../../ui-core/src/utils/ref"
        ).useForwardedRef,
    }),
    {virtual: true}
)

function renderDrop(props: DropFileProps) {
    const ref = createRef<HTMLInputElement>()
    const view = render(
        <ThemeProvider theme={theme}>
            <CustomDropFile {...props} ref={ref}>
                Choose election file
            </CustomDropFile>
        </ThemeProvider>
    )
    const input = ref.current
    assert.ok(input, "CustomDropFile must mount its file input before interaction")
    return {...view, input}
}

function fileList(file: File): FileList {
    return {
        0: file,
        length: 1,
        item: (index) => (index === 0 ? file : null),
        [Symbol.iterator]: () => [file].values(),
    }
}

it("opens the native picker from the keyboard and keeps accept on the input", async () => {
    const user = userEvent.setup()
    const {input} = renderDrop({handleFiles: jest.fn(), accept: ".json"})
    const picker = jest.spyOn(input, "click").mockImplementation(() => {})
    const button = screen.getByRole("button", {name: "Choose election file"})
    button.focus()
    await user.keyboard("{Enter} ")
    expect(picker).toHaveBeenCalledTimes(2)
    expect(input).toHaveAttribute("accept", ".json")
    picker.mockRestore()
})

it("forwards actual selected files and permits selecting the same file again", async () => {
    const handleFiles = jest.fn()
    const {input} = renderDrop({handleFiles})
    const file = new File(['{"election":"synthetic"}'], "election.json", {type: "application/json"})
    fireEvent.change(input, {target: {files: fileList(file)}})
    await waitFor(() => expect(handleFiles).toHaveBeenCalledTimes(1))
    expect(handleFiles.mock.calls[0][0].item(0)).toBe(file)
    expect(screen.getByText("election.json")).toBeVisible()
    await waitFor(() => expect(input).not.toBeDisabled())
    fireEvent.click(screen.getByTestId("drop-label-file"))
    fireEvent.change(input, {target: {files: fileList(file)}})
    await waitFor(() => expect(handleFiles).toHaveBeenCalledTimes(2))
})

it("shows drag feedback, handles a drop, and dismisses the overlay when the pointer leaves", async () => {
    const handleFiles = jest.fn()
    const {container} = renderDrop({handleFiles})
    const form = container.querySelector("form")
    assert.ok(form, "The drop target must contain its form")
    fireEvent.dragEnter(form)
    const overlay = container.querySelector(".drag-file-element")
    assert.ok(overlay, "Dragging a file must display the drop overlay")
    expect(overlay).toBeInTheDocument()
    fireEvent.dragOver(overlay)
    fireEvent.drop(overlay, {dataTransfer: {files: fileList(new File(["data"], "dropped.json"))}})
    await waitFor(() => expect(handleFiles).toHaveBeenCalledTimes(1))
    expect(screen.getByText("dropped.json")).toBeVisible()
    expect(container.querySelector(".drag-file-element")).toBeNull()
    fireEvent.dragEnter(form)
    const nextOverlay = container.querySelector(".drag-file-element")
    assert.ok(nextOverlay, "A new drag must recreate the drop overlay")
    fireEvent.dragLeave(nextOverlay)
    expect(container.querySelector(".drag-file-element")).toBeNull()
})

it("ignores cancellation and empty drops without invoking the importer", () => {
    const handleFiles = jest.fn()
    const {input, container} = renderDrop({handleFiles})
    fireEvent.change(input, {target: {files: null}})
    const form = container.querySelector("form")
    assert.ok(form, "The drop target must contain its form")
    fireEvent.dragEnter(form)
    const overlay = container.querySelector(".drag-file-element")
    assert.ok(overlay, "Dragging a file must display the drop overlay")
    fireEvent.drop(overlay, {dataTransfer: {files: []}})
    expect(handleFiles).not.toHaveBeenCalled()
    expect(fireEvent.submit(form)).toBe(false)
})

it("reports a failed importer and allows a subsequent retry", async () => {
    // A synchronous parser failure and a rejected Promise need the same visible
    // outcome. Neither should escape as an unhandled event rejection.
    const handleFiles = jest
        .fn()
        .mockImplementationOnce(() => {
            throw new Error("invalid input")
        })
        .mockRejectedValueOnce(new Error("import failed"))
        .mockResolvedValue(undefined)
    const {input} = renderDrop({handleFiles, errorMessage: "Could not import this file"})
    const files = fileList(new File(["data"], "election.json"))
    fireEvent.change(input, {target: {files}})
    expect(await screen.findByRole("alert")).toHaveTextContent("Could not import this file")
    fireEvent.change(input, {target: {files}})
    expect(await screen.findByRole("alert")).toHaveTextContent("Could not import this file")
    fireEvent.change(input, {target: {files}})
    await waitFor(() => expect(screen.queryByRole("alert")).toBeNull())
    expect(handleFiles).toHaveBeenCalledTimes(3)
})

it("does not start a second import while the first is pending", async () => {
    let finish: () => void = () => {
        throw new Error("import did not start")
    }
    const pending = new Promise<void>((resolve) => {
        finish = resolve
    })
    const handleFiles = jest.fn(() => pending)
    const {input} = renderDrop({handleFiles})
    const files = fileList(new File(["data"], "election.json"))
    fireEvent.change(input, {target: {files}})
    fireEvent.change(input, {target: {files}})
    expect(handleFiles).toHaveBeenCalledTimes(1)
    expect(input).toBeDisabled()
    await act(async () => {
        finish()
        await pending
    })
    expect(input).not.toBeDisabled()
})
