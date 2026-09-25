// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState} from "react"
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, within} from "storybook/test"
import {SecurityConfirmation} from "./SecurityConfirmation"

const meta = {
    title: "components/SecurityConfirmation",
    component: SecurityConfirmation,
    args: {
        election: {
            id: "election-1",
            tenant_id: "tenant-1",
            election_event_id: "event-1",
            image_document_id: "",
            contests: [],
            presentation: {
                i18n: {en: {security_confirmation_html: "<p>I am eligible to vote.</p>"}},
            },
        },
        checked: false,
        onChange: fn(),
    },
    render: function Confirmation(args) {
        const [checked, setChecked] = useState(args.checked)
        return (
            <SecurityConfirmation
                {...args}
                checked={checked}
                onChange={(value) => {
                    setChecked(value)
                    args.onChange(value)
                }}
            />
        )
    },
} satisfies Meta<typeof SecurityConfirmation>
export default meta
type Story = StoryObj<typeof meta>

export const AcceptWithKeyboard: Story = {
    play: async ({canvasElement, args}) => {
        const checkbox = within(canvasElement).getByRole("checkbox", {
            name: "I am eligible to vote.",
        })
        await expect(checkbox).not.toBeChecked()
        await userEvent.tab()
        await expect(checkbox).toHaveFocus()
        await userEvent.keyboard(" ")
        await expect(checkbox).toBeChecked()
        await expect(args.onChange).toHaveBeenCalledTimes(1)
        await expect(args.onChange).toHaveBeenLastCalledWith(true)
    },
}
