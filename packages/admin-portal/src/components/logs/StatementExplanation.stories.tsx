// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {expect, within} from "storybook/test"
import {i18n} from "@sequentech/ui-core"
import {StatementExplanation} from "./StatementExplanation"
import {logMessage} from "./logMessage"

const explanation = (outcome: string, code?: string) => ({
    outcome,
    deciding: "covered",
    checks: [
        {
            id: "covered",
            current: {message_key: "logsScreen.scheduledOutcome.check.covered"},
            published: null,
            allows: outcome !== "refused",
        },
    ],
    next_step: {message_key: "logsScreen.scheduledOutcome.check.defaults"},
    ...(code ? {authorized_by: {request_id: "r", code, signers: []}} : {}),
})

const message = (kind: string, details: unknown) =>
    logMessage({
        message: JSON.stringify({
            statement: {
                head: {kind, description: "Scheduled close of Dubai PCG"},
                body: {Signing: {kind, details_json: JSON.stringify(details)}},
            },
        }),
    })

interface Scenario {
    kind: string
    details: unknown
}

const meta = {
    title: "Admin/Logs/StatementExplanation",
    component: StatementExplanation,
    args: {
        kind: "ScheduledOutcomeChanged",
        details: {
            scheduled_event_id: "se-1",
            before: explanation("runs", "K7Q-2M"),
            after: explanation("refused"),
        },
    },
    render: ({kind, details}: Scenario) => (
        <StatementExplanation kind={kind} message={message(kind, details)} />
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const t = (key: string, params?: Record<string, unknown>) => String(i18n.t(key, params))

/** A prediction change: now and before, the deciding check and the next step. */
export const OutcomeChanged: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            canvas.getByText(
                t("logsScreen.scheduledOutcome.changed", {
                    after: t("logsScreen.scheduledOutcome.outcome.refused"),
                    before: t("logsScreen.scheduledOutcome.outcome.runs"),
                })
            )
        ).toBeVisible()
        expect(canvas.getAllByRole("listitem")).toHaveLength(3)
    },
}

/** A scheduled transition's outcome entry, authorized by a configuration approval. */
export const OutcomeAuthorized: Story = {
    args: {
        kind: "SigningActionExecuted",
        details: {action: "CloseVoting", explanation: explanation("runs", "K7Q-2M")},
    },
    play: async ({canvasElement}) => {
        await expect(
            within(canvasElement).getByText(
                t("logsScreen.scheduledOutcome.authorizedBy", {code: "K7Q-2M"})
            )
        ).toBeVisible()
    },
}

/** Other entries keep their description and JSON only. */
export const NoExplanation: Story = {
    args: {kind: "SigningActionExecuted", details: {action: "OpenVoting"}},
    play: async ({canvasElement}) => {
        expect(within(canvasElement).queryByRole("listitem")).toBeNull()
    },
}
