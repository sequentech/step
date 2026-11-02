// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, fireEvent, fn, userEvent, waitFor, within} from "storybook/test"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import DropJsonFile from "./DropJsonFile"

let boundary: ReturnType<typeof graphqlBoundary>

const meta = {
    title: "Admin/Components/DropJsonFile",
    component: DropJsonFile,
    args: {handleFiles: fn()},
    beforeEach: () => {
        boundary = graphqlBoundary({})
    },
    render: (args) => (
        <AdminStoryProvider boundary={boundary}>
            <DropJsonFile {...args} />
        </AdminStoryProvider>
    ),
} satisfies Meta<typeof DropJsonFile>
export default meta
type Story = StoryObj<typeof meta>

const eventFile = () => new File(['{"elections":[]}'], "event.json", {type: "application/json"})

const fileInput = (canvasElement: HTMLElement) => {
    const input = canvasElement.querySelector<HTMLInputElement>('input[type="file"]')
    if (!input) throw new Error("The drop zone has no file input")
    return input
}

export const BrowseFile: Story = {
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText("Drag & drop files or")).toBeVisible()
        await expect(canvas.getByText("Supported format: txt")).toBeVisible()
        const file = eventFile()
        await userEvent.upload(fileInput(canvasElement), file)
        await waitFor(() => expect(args.handleFiles).toHaveBeenCalledTimes(1))
        const [files] = args.handleFiles.mock.calls[0]
        expect([...files]).toEqual([file])
        await expect(canvas.getByText("event.json")).toBeVisible()
        expect(canvas.queryByRole("alert")).not.toBeInTheDocument()
    },
}

export const DropFile: Story = {
    play: async ({canvasElement, args}) => {
        const file = eventFile()
        const transfer = new DataTransfer()
        transfer.items.add(file)
        const drag = (type: string) =>
            new DragEvent(type, {bubbles: true, cancelable: true, dataTransfer: transfer})
        const dropzone = canvasElement.querySelector(".drop-file-dropzone")
        if (!dropzone) throw new Error("The drop zone is missing")
        fireEvent(dropzone, drag("dragenter"))
        const target = await waitFor(() => {
            const element = canvasElement.querySelector(".drag-file-element")
            if (!element) throw new Error("The drop target did not appear")
            return element
        })
        fireEvent(target, drag("drop"))
        await waitFor(() => expect(args.handleFiles).toHaveBeenCalledTimes(1))
        expect([...args.handleFiles.mock.calls[0][0]]).toEqual([file])
        await expect(within(canvasElement).getByText("event.json")).toBeVisible()
        expect(canvasElement.querySelector(".drag-file-element")).toBeNull()
    },
}

export const ImportFailureShowsError: Story = {
    args: {
        handleFiles: fn(async () => {
            throw new Error("Synthetic parser failure")
        }),
    },
    play: async ({canvasElement, args}) => {
        await userEvent.upload(fileInput(canvasElement), eventFile())
        const alert = await within(canvasElement).findByRole("alert")
        await expect(alert).toHaveTextContent("Could not import this file. Please try again.")
        expect(alert).not.toHaveTextContent("Synthetic parser failure")
        expect(args.handleFiles).toHaveBeenCalledTimes(1)
    },
}
