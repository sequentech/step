// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, within} from "storybook/test"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {ReportResult} from "./ReportResult"
import {EFakeReportState} from "./types"

let graphql: ReturnType<typeof graphqlBoundary>

const meta = {
    title: "Admin/Settings/Sent messages/ReportResult",
    component: ReportResult,
    args: {report: {state: EFakeReportState.ANSWERED, status: 200}},
    beforeEach: () => {
        graphql = graphqlBoundary({})
    },
    render: (args) => (
        <AdminStoryProvider boundary={graphql}>
            <ReportResult {...args} />
        </AdminStoryProvider>
    ),
} satisfies Meta<typeof ReportResult>
export default meta
type Story = StoryObj<typeof meta>

export const Accepted: Story = {
    play: async ({canvasElement}) => {
        const mark = within(canvasElement).getByRole("img", {
            name: "Report accepted by the webhook component (HTTP 200)",
        })
        await expect(mark).toHaveTextContent("✓✓")
    },
}

export const Refused: Story = {
    args: {report: {state: EFakeReportState.ANSWERED, status: 503}},
    play: async ({canvasElement}) => {
        await expect(within(canvasElement).getByText("Report refused: HTTP 503")).toBeVisible()
    },
}

export const Pending: Story = {
    args: {report: {state: EFakeReportState.PENDING}},
    play: async ({canvasElement}) => {
        await expect(within(canvasElement).getByText("Report pending")).toBeVisible()
    },
}

export const NotSent: Story = {
    args: {report: {state: EFakeReportState.NOT_SENT, reason: "Several accounts use this sender"}},
    play: async ({canvasElement}) => {
        await expect(
            within(canvasElement).getByText("Report not sent: Several accounts use this sender")
        ).toBeVisible()
    },
}

export const Unreachable: Story = {
    args: {report: {state: EFakeReportState.UNREACHABLE, reason: null}},
    play: async ({canvasElement}) => {
        await expect(
            within(canvasElement).getByText("The report got no answer: no reason given")
        ).toBeVisible()
    },
}

export const NoReceipts: Story = {
    args: {
        report: {
            state: EFakeReportState.NO_RECEIPTS,
            reason: "This provider sends no delivery reports",
        },
    },
    play: async ({canvasElement}) => {
        await expect(
            within(canvasElement).getByText("No delivery reports on this channel")
        ).toBeVisible()
    },
}
