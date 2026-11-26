// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, within} from "storybook/test"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {TenantReceipt} from "./PackageHub"

const meta = {
    title: "Admin/Settings/TenantReceipt",
    component: TenantReceipt,
    render: () => (
        <AdminStoryProvider boundary={graphqlBoundary({})}>
            <TenantReceipt receipt={null} onHistory={() => undefined} />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Record<string, never>>
export default meta

export const Default: StoryObj = {
    play: async ({canvasElement}) => {
        await expect(within(canvasElement).getByTestId("package-receipt")).toBeVisible()
    },
}
