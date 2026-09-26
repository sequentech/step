// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, within} from "storybook/test"
import {SecretAttributeInput} from "./SecretAttributeInput"

const meta = {
    title: "Admin/User/SecretAttributeInput",
    component: SecretAttributeInput,
    args: {
        label: "Security answer",
        values: [],
        stored: true,
        editable: true,
        multivalued: false,
        canReveal: true,
        revealed: false,
        revealing: false,
        required: false,
        labels: {
            reveal: "Reveal",
            hide: "Hide",
            clear: "Clear",
            add: "Add value",
            remove: "Remove",
        },
        onChange: fn(),
        onReveal: fn(),
    },
} satisfies Meta<typeof SecretAttributeInput>
export default meta
type Story = StoryObj<typeof meta>

const field = (canvasElement: HTMLElement) =>
    within(canvasElement).getByLabelText(/^Security answer/)

export const StoredAndHidden: Story = {
    play: async ({canvasElement, args}) => {
        expect(field(canvasElement)).toHaveAttribute("type", "password")
        expect(field(canvasElement)).toHaveValue("")
        expect(field(canvasElement)).toHaveAttribute("placeholder", "••••••••")
        await userEvent.click(within(canvasElement).getByRole("button", {name: "Reveal"}))
        expect(args.onReveal).toHaveBeenCalledTimes(1)
        expect(args.onChange).not.toHaveBeenCalled()
    },
}

export const Revealed: Story = {
    args: {values: ["synthetic answer"], revealed: true},
    play: async ({canvasElement}) => {
        expect(field(canvasElement)).toHaveAttribute("type", "text")
        expect(field(canvasElement)).toHaveValue("synthetic answer")
        await expect(within(canvasElement).getByRole("button", {name: "Hide"})).toBeEnabled()
    },
}

export const Revealing: Story = {
    args: {revealing: true},
    play: async ({canvasElement}) => {
        const reveal = within(canvasElement).getByRole("button", {name: "Reveal"})
        expect(reveal).toBeDisabled()
        expect(reveal).toHaveAttribute("aria-busy", "true")
        expect(within(canvasElement).getByRole("button", {name: "Clear"})).toBeDisabled()
        expect(field(canvasElement)).toBeDisabled()
    },
}

export const TypeANewValue: Story = {
    args: {stored: false},
    play: async ({canvasElement, args}) => {
        await userEvent.type(field(canvasElement), "x")
        expect(args.onChange).toHaveBeenLastCalledWith(["x"])
    },
}

export const ClearTheStoredValue: Story = {
    play: async ({canvasElement, args}) => {
        await userEvent.click(within(canvasElement).getByRole("button", {name: "Clear"}))
        expect(args.onChange).toHaveBeenCalledWith([])
    },
}

export const ReadOnlyWithoutRevealPermission: Story = {
    args: {editable: false, canReveal: false},
    play: async ({canvasElement}) => {
        expect(field(canvasElement)).toHaveAttribute("readonly")
        expect(within(canvasElement).queryByRole("button")).not.toBeInTheDocument()
    },
}

export const MultivaluedEditing: Story = {
    args: {stored: false, multivalued: true, revealed: true, values: ["first", "second"]},
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        expect(canvas.getAllByLabelText("Security answer")).toHaveLength(2)
        await userEvent.click(canvas.getByRole("button", {name: "Add value"}))
        expect(args.onChange).toHaveBeenLastCalledWith(["first", "second", ""])
        await userEvent.click(canvas.getAllByRole("button", {name: "Remove"})[0])
        expect(args.onChange).toHaveBeenLastCalledWith(["second"])
    },
}

export const RequiredWithError: Story = {
    args: {stored: false, required: true, error: true, helperText: "This field is required"},
    play: async ({canvasElement}) => {
        expect(field(canvasElement)).toBeRequired()
        expect(field(canvasElement)).toHaveAttribute("aria-invalid", "true")
        await expect(within(canvasElement).getByText("This field is required")).toBeVisible()
    },
}
