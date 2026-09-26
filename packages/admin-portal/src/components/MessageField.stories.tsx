// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, userEvent, within} from "storybook/test"
import {RecordContextProvider} from "react-admin"
import {i18n} from "@sequentech/ui-core"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {MessageField} from "./MessageField"

const LOG_MESSAGE =
    "Voter 42 cast a ballot in North district from the kiosk at the town hall; " +
    "the receipt was printed and the voter confirmed it."

let graphql: ReturnType<typeof graphqlBoundary>

const meta = {
    title: "Admin/Components/MessageField",
    component: MessageField,
    args: {source: "message", initialLength: 40},
    beforeEach: async () => {
        graphql = graphqlBoundary({}, {schema: true})
        await graphql.ready
    },
    render: (args) => (
        <AdminStoryProvider boundary={graphql}>
            <RecordContextProvider value={{id: 1, message: LOG_MESSAGE}}>
                <MessageField {...args} />
            </RecordContextProvider>
        </AdminStoryProvider>
    ),
} satisfies Meta<typeof MessageField>
export default meta
type Story = StoryObj<typeof meta>

export const Truncated: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            await canvas.findByText("Voter 42 cast a ballot in North district")
        ).toBeVisible()
        await expect(canvas.getByText("...")).toBeVisible()
        expect(canvas.queryByText(LOG_MESSAGE)).not.toBeInTheDocument()
    },
}

export const ShowMoreAndLess: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await userEvent.click(
            await canvas.findByRole("button", {
                name: i18n.t("electionEventScreen.common.showMore"),
            })
        )
        await expect(canvas.getByText(LOG_MESSAGE)).toBeVisible()
        await userEvent.click(
            canvas.getByRole("button", {name: i18n.t("electionEventScreen.common.showLess")})
        )
        await expect(canvas.getByText("Voter 42 cast a ballot in North district")).toBeVisible()
    },
}

export const ShortMessage: Story = {
    args: {initialLength: 256},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText(LOG_MESSAGE)).toBeVisible()
        expect(canvas.queryByRole("button")).not.toBeInTheDocument()
    },
}

export const ContentFromParent: Story = {
    args: {source: undefined, content: "Election event published by admin."},
    play: async ({canvasElement}) => {
        // Content passed directly takes precedence over the record.
        await expect(
            await within(canvasElement).findByText("Election event published by admin.")
        ).toBeVisible()
    },
}
