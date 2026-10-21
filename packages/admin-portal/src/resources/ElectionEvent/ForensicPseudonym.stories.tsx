// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, within} from "storybook/test"
import {ForensicPseudonym} from "./ForensicFields"

const meta = {
    title: "Admin/Election event/ForensicPseudonym",
    component: ForensicPseudonym,
    args: {code: "7QKM-2HXD"},
} satisfies Meta<typeof ForensicPseudonym>
export default meta
type Story = StoryObj<typeof meta>

export const Pseudonym: Story = {
    play: async ({canvasElement, args}) => {
        await expect(within(canvasElement).getByText(args.code)).toBeVisible()
    },
}
