// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {Meta, StoryObj} from "@storybook/react"
import BreadCrumbSteps from "../BreadCrumbSteps"
import {INITIAL_VIEWPORTS} from "@storybook/addon-viewport"

const meta: Meta<typeof BreadCrumbSteps> = {
    title: "components/BreadCrumbSteps",
    component: BreadCrumbSteps,
    parameters: {
        backgrounds: {
            default: "white",
        },
        viewport: {
            viewports: INITIAL_VIEWPORTS,
            defaultViewport: "iphone6",
        },
    },
}

export default meta

type Story = StoryObj<typeof BreadCrumbSteps>

export const Primary: Story = {
    args: {
        labels: ["breadcrumbSteps.import", "breadcrumbSteps.verify", "breadcrumbSteps.finish"],
        selected: 1,
    },
    parameters: {
        viewport: {
            disable: true,
        },
    },
}

export const Warning: Story = {
    args: {
        labels: ["breadcrumbSteps.import", "breadcrumbSteps.verify", "breadcrumbSteps.finish"],
        selected: 2,
        warning: true,
    },
    parameters: {
        viewport: {
            disable: true,
        },
    },
}
