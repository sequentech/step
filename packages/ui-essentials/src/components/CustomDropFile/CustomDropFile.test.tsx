/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React, {createRef} from "react"
import {act, fireEvent, render, screen, waitFor} from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import "@testing-library/jest-dom"
import {ThemeProvider} from "@mui/material/styles"
import theme from "../../services/theme"
import CustomDropFile, {type DropFileProps} from "./CustomDropFile"
import DropFile from "../DropFile/DropFile"
import DropJsonFile from "../../../../admin-portal/src/components/DropJsonFile"
import mockSpanish from "../../translations/es"

jest.mock("react-i18next", () => ({
    useTranslation: () => ({
        t: (key: string) => {
            const name = key.replace(
                "dragNDrop.",
                ""
            ) as keyof typeof mockSpanish.translations.dragNDrop
            return mockSpanish.translations.dragNDrop[name] || key
        },
    }),
}))

jest.mock(
    "@sequentech/ui-core",
    () => ({
        useForwardedRef: jest.requireActual<typeof import("../../../../ui-core/src/utils/ref")>(
            "../../../../ui-core/src/utils/ref"
        ).useForwardedRef,
    }),
    {virtual: true}
)

jest.mock(
    "@sequentech/ui-essentials",
    () => ({
        CustomDropFile: jest.requireActual("./CustomDropFile").default,
        Icon: jest.requireActual("../Icon/Icon").default,
        theme: jest.requireActual("../../services/theme").default,
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
    return {...view, ref}
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
    const {ref} = renderDrop({handleFiles: jest.fn(), accept: ".json"})
    const input = ref.current!
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
    const {ref} = renderDrop({handleFiles})
    const file = new File(['{"election":"synthetic"}'], "election.json", {type: "application/json"})
    fireEvent.change(ref.current!, {target: {files: fileList(file)}})
    await waitFor(() => expect(handleFiles).toHaveBeenCalledTimes(1))
    expect(handleFiles.mock.calls[0][0].item(0)).toBe(file)
    expect(screen.getByText("election.json")).toBeVisible()
    await waitFor(() => expect(ref.current).not.toBeDisabled())
    fireEvent.click(screen.getByTestId("drop-label-file"))
    fireEvent.change(ref.current!, {target: {files: fileList(file)}})
    await waitFor(() => expect(handleFiles).toHaveBeenCalledTimes(2))
})

it("shows drag feedback, handles a drop, and dismisses the overlay when the pointer leaves", async () => {
    const handleFiles = jest.fn()
    const {container} = renderDrop({handleFiles})
    const form = container.querySelector("form")!
    fireEvent.dragEnter(form)
    const overlay = container.querySelector(".drag-file-element")!
    expect(overlay).toBeInTheDocument()
    fireEvent.dragOver(overlay)
    fireEvent.drop(overlay, {dataTransfer: {files: fileList(new File(["data"], "dropped.json"))}})
    await waitFor(() => expect(handleFiles).toHaveBeenCalledTimes(1))
    expect(screen.getByText("dropped.json")).toBeVisible()
    expect(container.querySelector(".drag-file-element")).toBeNull()
    fireEvent.dragEnter(form)
    fireEvent.dragLeave(container.querySelector(".drag-file-element")!)
    expect(container.querySelector(".drag-file-element")).toBeNull()
})

it("ignores cancellation and empty drops without invoking the importer", () => {
    const handleFiles = jest.fn()
    const {ref, container} = renderDrop({handleFiles})
    fireEvent.change(ref.current!, {target: {files: null}})
    fireEvent.dragEnter(container.querySelector("form")!)
    fireEvent.drop(container.querySelector(".drag-file-element")!, {dataTransfer: {files: []}})
    expect(handleFiles).not.toHaveBeenCalled()
    expect(fireEvent.submit(container.querySelector("form")!)).toBe(false)
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
    const {ref} = renderDrop({handleFiles, errorMessage: "Could not import this file"})
    const files = fileList(new File(["data"], "election.json"))
    fireEvent.change(ref.current!, {target: {files}})
    expect(await screen.findByRole("alert")).toHaveTextContent("Could not import this file")
    fireEvent.change(ref.current!, {target: {files}})
    expect(await screen.findByRole("alert")).toHaveTextContent("Could not import this file")
    fireEvent.change(ref.current!, {target: {files}})
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
    const {ref} = renderDrop({handleFiles})
    const files = fileList(new File(["data"], "election.json"))
    fireEvent.change(ref.current!, {target: {files}})
    fireEvent.change(ref.current!, {target: {files}})
    expect(handleFiles).toHaveBeenCalledTimes(1)
    expect(ref.current).toBeDisabled()
    await act(async () => {
        finish()
        await pending
    })
    expect(ref.current).not.toBeDisabled()
})

it("blocks imports re-entered before React commits the busy state", async () => {
    let input: HTMLInputElement
    let finish: () => void = () => {
        throw new Error("import did not start")
    }
    const pending = new Promise<void>((resolve) => {
        finish = resolve
    })
    const files = fileList(new File(["data"], "election.json"))
    const handleFiles = jest.fn(() => {
        // A consumer may dispatch another selection synchronously. The state
        // update has not committed yet, but the first import already owns it.
        if (handleFiles.mock.calls.length === 1) fireEvent.change(input, {target: {files}})
        return pending
    })
    const {ref} = renderDrop({handleFiles})
    input = ref.current!
    fireEvent.change(input, {target: {files}})
    expect(handleFiles).toHaveBeenCalledTimes(1)
    await act(async () => {
        finish()
        await pending
    })
    fireEvent.change(input, {target: {files}})
    await waitFor(() => expect(handleFiles).toHaveBeenCalledTimes(2))
})

it.each([DropFile, DropJsonFile])(
    "opens the picker once from a wrapper browse label and uses translated failure text",
    async (Component) => {
        const user = userEvent.setup()
        const {container} = render(
            <ThemeProvider theme={theme}>
                <Component
                    handleFiles={() => {
                        throw new Error("private parser detail")
                    }}
                />
            </ThemeProvider>
        )
        const input = container.querySelector<HTMLInputElement>("input[type=file]")!
        const picker = jest.spyOn(input, "click").mockImplementation(() => {})
        expect(screen.getAllByRole("button")).toHaveLength(1)
        expect(
            screen
                .getByTestId("drop-label-file")
                .querySelector("button, input, a[href], [role=button]")
        ).toBeNull()
        await user.click(screen.getByTestId("drop-file-button"))
        expect(picker).toHaveBeenCalledTimes(1)
        fireEvent.change(input, {target: {files: fileList(new File(["bad"], "invalid.json"))}})
        expect(await screen.findByRole("alert")).toHaveTextContent(
            "No se pudo importar este archivo. Inténtalo de nuevo."
        )
        expect(screen.queryByText("private parser detail")).toBeNull()
        picker.mockRestore()
    }
)
