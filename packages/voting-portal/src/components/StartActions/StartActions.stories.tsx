// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, within} from "storybook/test"
import {EConsolidatedReportPolicy, ESecurityConfirmationPolicy} from "@sequentech/ui-core"
import {StartActions} from "./StartActions"

const meta = {
    title: "Voting/Start actions",
    component: StartActions,
    args: {
        election: {
            id: "election-1",
            tenant_id: "tenant-1",
            election_event_id: "event-1",
            image_document_id: "",
            contests: [],
            presentation: {
                consolidated_report_policy: EConsolidatedReportPolicy.DO_NOT_GENERATE,
                security_confirmation_policy: ESecurityConfirmationPolicy.MANDATORY,
                i18n: {en: {security_confirmation_html: "<p>I am eligible to vote.</p>"}},
            },
        },
        isDeclineToVotePolicyEnabled: true,
        onDeclineToVoteClick: fn(),
    },
    parameters: {
        router: {
            path: "/tenant/:tenantId/event/:eventId/election/:electionId/start",
            initialEntries: ["/tenant/tenant-1/event/event-1/election/election-1/start?lang=en"],
        },
    },
    decorators: [
        (Story) => (
            <main>
                <h1>Start voting</h1>
                <Story />
            </main>
        ),
    ],
} satisfies Meta<typeof StartActions>
export default meta
type Story = StoryObj<typeof meta>

export const MandatoryDeclaration: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const checkbox = canvas.getByRole("checkbox", {name: "I am eligible to vote."})
        const start = canvas.getByRole("button", {name: "Start Voting"})
        await expect(start).toBeDisabled()
        await expect(canvas.getByRole("button", {name: "Decline to Vote"})).toBeDisabled()
        await userEvent.tab()
        await expect(checkbox).toHaveFocus()
        await userEvent.keyboard(" ")
        await expect(start).toBeEnabled()
        await userEvent.click(start)
        await expect(canvas.getByLabelText("Current location")).toHaveTextContent(
            "/tenant/tenant-1/event/event-1/election/election-1/vote?lang=en"
        )
    },
}

export const DeclineAfterDeclaration: Story = {
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        const checkbox = canvas.getByRole("checkbox", {name: "I am eligible to vote."})
        await userEvent.click(checkbox)
        await userEvent.click(canvas.getByRole("button", {name: "Decline to Vote"}))
        await expect(args.onDeclineToVoteClick).toHaveBeenCalledTimes(1)
        await expect(canvas.getByLabelText("Current location")).toHaveTextContent("/start?lang=en")
        await userEvent.click(checkbox)
        await expect(canvas.getByRole("button", {name: "Start Voting"})).toBeDisabled()
    },
}

export const NoDeclarationOrDecline: Story = {
    args: {
        election: {
            ...meta.args.election,
            presentation: {
                ...meta.args.election.presentation,
                security_confirmation_policy: ESecurityConfirmationPolicy.NONE,
            },
        },
        isDeclineToVotePolicyEnabled: false,
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.queryByRole("checkbox")).not.toBeInTheDocument()
        await expect(
            canvas.queryByRole("button", {name: "Decline to Vote"})
        ).not.toBeInTheDocument()
        await expect(canvas.getByRole("button", {name: "Start Voting"})).toBeEnabled()
    },
}
