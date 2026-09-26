// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, within} from "storybook/test"
import {i18n} from "@sequentech/ui-core"
import {HeaderTitle} from "./HeaderTitle"
import {EStoryLocale} from "../../../ui-essentials/.storybook/globals"

const meta = {
    title: "Admin/Components/HeaderTitle",
    component: HeaderTitle,
    args: {
        title: "electionTypeScreen.common.settingTitle",
        subtitle: "electionTypeScreen.common.settingSubtitle",
    },
} satisfies Meta<typeof HeaderTitle>
export default meta
type Story = StoryObj<typeof meta>

export const TranslatedKeys: Story = {
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText(i18n.t(args.title))).toBeVisible()
        await expect(canvas.getByText(i18n.t(args.subtitle))).toBeVisible()
    },
}

export const SpanishLocale: Story = {
    globals: {locale: EStoryLocale.SPANISH},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText("Configuración")).toBeVisible()
        expect(canvas.queryByText("Settings")).not.toBeInTheDocument()
    },
}

export const PlainTextIsShownAsIs: Story = {
    args: {title: "Council settings", subtitle: "Synthetic subtitle"},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText("Council settings")).toBeVisible()
        await expect(canvas.getByText("Synthetic subtitle")).toBeVisible()
    },
}
