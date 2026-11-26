// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, within} from "storybook/test"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {RequirementsSummary} from "./PackageHub"
import {COMPLETE_DELIVERY} from "./configurationPackage"

const meta = {
    title: "Admin/Settings/RequirementsSummary",
    component: RequirementsSummary,
    render: () => (
        <AdminStoryProvider boundary={graphqlBoundary({})}>
            <span data-testid="package-requirements-summary-example"><RequirementsSummary requirements={COMPLETE_DELIVERY} /></span>
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Record<string, never>>
export default meta

export const Default: StoryObj = {
    play: async ({canvasElement}) => {
        await expect(within(canvasElement).getByTestId("package-requirements-summary-example")).toHaveTextContent("6")
    },
}
