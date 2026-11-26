// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, within} from "storybook/test"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {PackageRail} from "./PackageHub"
import {PACKAGE_SECTIONS} from "./configurationPackage"
const rows = PACKAGE_SECTIONS.map((section) => ({section, badge: "blocked" as const}))

const meta = {
    title: "Admin/Settings/PackageRail",
    component: PackageRail,
    render: () => (
        <AdminStoryProvider boundary={graphqlBoundary({})}>
            <PackageRail
                tab="overview"
                onTab={() => undefined}
                sections={rows}
                trustBlocked={false}
            />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Record<string, never>>
export default meta

export const Default: StoryObj = {
    parameters: {widgets: ["BadgeDot"]},
    play: async ({canvasElement}) => {
        await expect(
            within(canvasElement).getByTestId("package-rail-tenant-configuration")
        ).toBeVisible()
    },
}
