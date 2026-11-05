// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import PhoneInput from "./PhoneInput"

let boundary: ReturnType<typeof graphqlBoundary>

const meta = {
    title: "Admin/Components/PhoneInput",
    component: PhoneInput,
    args: {label: "Mobile number", handlePhoneNumberChange: fn(), fullWidth: true},
    beforeEach: () => {
        boundary = graphqlBoundary({})
    },
    render: (args) => (
        <AdminStoryProvider boundary={boundary}>
            <PhoneInput {...args} />
        </AdminStoryProvider>
    ),
} satisfies Meta<typeof PhoneInput>
export default meta
type Story = StoryObj<typeof meta>

const telephone = (canvasElement: HTMLElement) =>
    waitFor(() => {
        const input = canvasElement.querySelector<HTMLInputElement>('input[type="tel"]')
        if (!input) throw new Error("PhoneInput renders no telephone input")
        return input
    })

export const ChooseCountryThenNumber: Story = {
    parameters: {
        expectedFailure: {
            reason: "Once a country is chosen, intl-tel-input's country button keeps aria-activedescendant on an option of the closed list.",
            a11y: ["aria-valid-attr-value"],
        },
    },
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText("Mobile number")).toBeVisible()
        await userEvent.click(await canvas.findByRole("combobox", {name: "Selected country"}))
        // On a narrow viewport the country list opens as a popup at the end of the page.
        const page = within(document.body)
        await userEvent.type(await page.findByRole("combobox", {name: "Search"}), "Spain")
        await userEvent.click(await page.findByRole("option", {name: /Spain/}))
        await userEvent.type(await telephone(canvasElement), "612345678")
        await waitFor(() =>
            expect(args.handlePhoneNumberChange).toHaveBeenLastCalledWith("+34612345678")
        )
        await expect(canvas.getByText("+34")).toBeVisible()
    },
}

export const InitialValue: Story = {
    args: {initialValue: "+34612345678"},
    play: async ({canvasElement}) => {
        const input = await telephone(canvasElement)
        await waitFor(() => expect(input.value.replace(/\D/g, "")).toBe("612345678"))
        await expect(within(canvasElement).getByText("+34")).toBeVisible()
    },
}

export const Disabled: Story = {
    args: {disabled: true, initialValue: "+34612345678"},
    play: async ({canvasElement, args}) => {
        const input = await telephone(canvasElement)
        await expect(input).toBeDisabled()
        await userEvent.type(input, "9")
        expect(args.handlePhoneNumberChange).not.toHaveBeenCalledWith(
            expect.stringContaining("6123456789")
        )
    },
}
