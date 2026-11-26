// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, within} from "storybook/test"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {WizardSteps} from "./PackageHub"

const meta = {
    title: "Admin/Settings/WizardSteps",
    component: WizardSteps,
    render: () => (
        <AdminStoryProvider boundary={graphqlBoundary({})}>
            <WizardSteps current={1} />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Record<string, never>>
export default meta

export const Default: StoryObj = {
    play: async ({canvasElement}) => {
        await expect(
            within(canvasElement)
                .getByTestId("package-wizard-steps")
                .querySelector('[aria-current="step"]')
        ).toBeVisible()
    },
}
