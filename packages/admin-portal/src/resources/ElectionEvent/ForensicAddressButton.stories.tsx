// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, within} from "storybook/test"
import {ForensicAddressButton} from "./ForensicFields"

const meta = {
    title: "Admin/Election event/ForensicAddressButton",
    component: ForensicAddressButton,
    args: {ip: "192.0.2.10", onOpen: fn()},
} satisfies Meta<typeof ForensicAddressButton>
export default meta
type Story = StoryObj<typeof meta>

export const OpenAddress: Story = {
    play: async ({canvasElement, args}) => {
        await userEvent.click(within(canvasElement).getByRole("button", {name: args.ip}))
        expect(args.onOpen).toHaveBeenCalledOnce()
    },
}

export const Ipv6: Story = {
    ...OpenAddress,
    args: {ip: "2001:db8:1234:5678:90ab:cdef:1234:5678"},
}
