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
    /** The statement body's values, for the ballot box seal entries. */
    values?: unknown[]
}

const sealMessage = (kind: string, values: unknown[]) =>
    logMessage({
        message: JSON.stringify({
            statement: {
                head: {kind, description: "Ballot box of Madrid Post, Spain"},
                body: {[kind]: values},
            },
        }),
    })

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
    render: ({kind, details, values}: Scenario) => (
        <StatementExplanation
            kind={kind}
            message={values ? sealMessage(kind, values) : message(kind, details)}
        />
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

const HASH = `ef187f0b${"3c".repeat(56)}22a65e5b`

/** VOTE-FREEZE: a seal entry shows its hash, counts, why the others don't count and Close voting request. */
export const BallotBoxSealed: Story = {
    args: {
        kind: "BallotBoxSealed",
        values: [null, "spain", HASH, 1342, 1340, "7f3a91c2-0de0-4b5e-9c1a-2b3c4d5e6f70"],
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText(`Seal hash: ${HASH}`)).toBeVisible()
        await expect(canvas.getByText("1340 of 1342 ballots counted.")).toBeVisible()
        await expect(
            canvas.getByText(
                "The other 2 ballots were replaced by the voter's later ballot, discarded, or cast by a voter who is not eligible."
            )
        ).toBeVisible()
    },
}

/** VOTE-FREEZE: a failed seal says why, and that the box stays locked. */
export const BallotBoxSealFailed: Story = {
    args: {
        kind: "BallotBoxSealFailed",
        values: [null, "andorra", "a ballot does not match its Ballot ID"],
    },
    play: async ({canvasElement}) => {
        await expect(
            within(canvasElement).getByText(
                "The ballot box stays locked and is not sealed: an incident."
            )
        ).toBeVisible()
    },
}

/** VOTE-FREEZE: the tally's check of a box against its seal. */
export const TallyBallotBoxRejected: Story = {
    args: {
        kind: "TallyBallotBoxRejected",
        values: [null, "spain", "1 sealed ballot is missing", "ts-1"],
    },
    play: async ({canvasElement}) => {
        await expect(
            within(canvasElement).getByText("What differs: 1 sealed ballot is missing")
        ).toBeVisible()
    },
}
