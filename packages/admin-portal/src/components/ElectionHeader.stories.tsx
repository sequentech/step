// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, within} from "storybook/test"
import {i18n} from "@sequentech/ui-core"
import ElectionHeader from "./ElectionHeader"

const meta = {
    title: "Admin/Components/ElectionHeader",
    component: ElectionHeader,
    args: {title: "tallysheet.title", subtitle: "tallysheet.subtitle"},
} satisfies Meta<typeof ElectionHeader>
export default meta
type Story = StoryObj<typeof meta>

export const Populated: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        // Both props are translation keys, shown in the toolbar language.
        await expect(canvas.getByText(i18n.t("tallysheet.title"))).toBeVisible()
        await expect(canvas.getByText(i18n.t("tallysheet.subtitle"))).toBeVisible()
    },
}

export const RecordName: Story = {
    args: {title: "Council election", subtitle: "electionScreen.common.subtitle"},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        // Screens pass an already translated record alias, which is shown as is.
        await expect(canvas.getByText("Council election")).toBeVisible()
        await expect(canvas.getByText(i18n.t("electionScreen.common.subtitle"))).toBeVisible()
    },
}

export const WithoutSubtitle: Story = {
    args: {subtitle: ""},
    play: async ({canvasElement}) => {
        const title = within(canvasElement).getByText(i18n.t("tallysheet.title"))
        await expect(title).toBeVisible()
        expect(title.nextElementSibling).toBeEmptyDOMElement()
    },
}
