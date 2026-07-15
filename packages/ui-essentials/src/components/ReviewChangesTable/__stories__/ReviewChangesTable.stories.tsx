// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {Meta, StoryObj} from "@storybook/react"
import {INITIAL_VIEWPORTS} from "@storybook/addon-viewport"
import ReviewChangesTable from "../ReviewChangesTable"

const meta: Meta<typeof ReviewChangesTable> = {
    title: "components/ReviewChangesTable",
    component: ReviewChangesTable,
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

type Story = StoryObj<typeof ReviewChangesTable>

const parameters = {
    viewport: {
        disable: true,
    },
}

export const Primary: Story = {
    args: {
        title: "Review changes",
        subtitle: "Confirm these updates before submitting.",
        fieldLabel: "Field",
        currentValueLabel: "Current value",
        newValueLabel: "New value",
        rows: [
            {
                field: "first_name",
                label: "First Name",
                currentValue: "SO304",
                newValue: "SO304-Edited",
            },
            {
                field: "email",
                label: "Email",
                currentValue: "so304.voter1-old@example.com",
                newValue: "so304.voter1@example.com",
            },
            {
                field: "area",
                label: "Area",
                currentValue: "Unassigned",
                newValue: "Main Area",
            },
        ],
    },
    parameters,
}

export const SingleChange: Story = {
    args: {
        title: "Review changes",
        subtitle: "Confirm these updates before submitting.",
        fieldLabel: "Field",
        currentValueLabel: "Current value",
        newValueLabel: "New value",
        rows: [
            {
                field: "enabled",
                label: "Enabled",
                currentValue: "Yes",
                newValue: "No",
            },
        ],
    },
    parameters,
}

export const NoSubtitle: Story = {
    args: {
        title: "Review changes",
        fieldLabel: "Field",
        currentValueLabel: "Current value",
        newValueLabel: "New value",
        rows: [
            {
                field: "last_name",
                label: "Last Name",
                currentValue: "Doe",
                newValue: "Smith",
            },
        ],
    },
    parameters,
}
