// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, within} from "storybook/test"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {RunStages} from "./PackageHub"
import {runStages} from "./packageOverview"

const meta = {
    title: "Admin/Settings/RunStages",
    component: RunStages,
    render: () => (
        <AdminStoryProvider boundary={graphqlBoundary({})}>
            <RunStages stages={runStages("review", "preview")} />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Record<string, never>>
export default meta

export const Default: StoryObj = {
    play: async ({canvasElement}) => {
        await expect(within(canvasElement).getByTestId("package-run-stages")).toBeVisible()
    },
}
