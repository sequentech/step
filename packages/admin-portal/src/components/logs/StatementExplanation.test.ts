// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {TFunction} from "i18next"
import {explanationLines} from "./StatementExplanation"
import {logDetails, logMessage} from "./logMessage"

jest.mock("@sequentech/ui-essentials", () => ({}), {virtual: true})

/** Renders a key with its parameters, so the test reads what each line says. */
const t = ((key: string, params?: Record<string, unknown>) =>
    params && Object.keys(params).length
        ? `${key}(${Object.entries(params)
              .map(([name, value]) => `${name}=${value}`)
              .join(", ")})`
        : key) as unknown as TFunction

const explanation = (outcome: string, deciding: string, extra: Record<string, unknown> = {}) => ({
    outcome,
    deciding,
    checks: [
        {
            id: "covered",
            current: {message_key: "outcome.covered.edited", params: {user: "ofov.admin"}},
            published: null,
            allows: outcome !== "refused",
        },
    ],
    next_step: {message_key: "outcome.next.publishAndApprove"},
    ...extra,
})

/** A row as the list action returns it: the board message as JSON. */
const row = (kind: string, details: unknown) => ({
    statement_kind: kind,
    message: JSON.stringify({
        election_id: "dubai",
        statement: {
            head: {kind, description: "Scheduled close of Dubai PCG"},
            body: {Signing: {kind, details_json: JSON.stringify(details)}},
        },
    }),
})

describe("readable log rows", () => {
    it("explains a scheduled outcome change: now, before, the deciding check and the next step", () => {
        const record = row("ScheduledOutcomeChanged", {
            scheduled_event_id: "se-1",
            before: explanation("runs", "covered", {
                authorized_by: {request_id: "r", code: "K7Q-2M", signers: []},
            }),
            after: explanation("refused", "covered"),
        })
        expect(explanationLines(t, record.statement_kind, logMessage(record))).toEqual([
            "logsScreen.scheduledOutcome.changed(after=logsScreen.scheduledOutcome.outcome.refused, before=logsScreen.scheduledOutcome.outcome.runs)",
            "logsScreen.scheduledOutcome.deciding(check=logsScreen.scheduledOutcome.check.covered, value=outcome.covered.edited(user=ofov.admin))",
            "logsScreen.scheduledOutcome.nextStep(step=outcome.next.publishAndApprove)",
        ])
    })

    it("explains a scheduled transition's outcome entry with its authorization", () => {
        const record = row("SigningActionExecuted", {
            action: "CloseVoting",
            explanation: explanation("runs", "covered", {
                authorized_by: {request_id: "r", code: "K7Q-2M", signers: ["a"]},
            }),
        })
        expect(explanationLines(t, record.statement_kind, logMessage(record))).toEqual([
            "logsScreen.scheduledOutcome.result(outcome=logsScreen.scheduledOutcome.outcome.runs)",
            "logsScreen.scheduledOutcome.deciding(check=logsScreen.scheduledOutcome.check.covered, value=outcome.covered.edited(user=ofov.admin))",
            "logsScreen.scheduledOutcome.authorizedBy(code=K7Q-2M)",
            "logsScreen.scheduledOutcome.nextStep(step=outcome.next.publishAndApprove)",
        ])
    })

    it("leaves every other entry to its description and JSON", () => {
        const signed = row("SigningActionExecuted", {action: "OpenVoting", request_id: "r"})
        expect(explanationLines(t, signed.statement_kind, logMessage(signed))).toBeNull()
        const vote = {statement_kind: "CastVote", message: JSON.stringify({statement: {}})}
        expect(explanationLines(t, vote.statement_kind, logMessage(vote))).toBeNull()
        expect(logDetails(logMessage({message: "not json"}))).toBeNull()
    })
})
