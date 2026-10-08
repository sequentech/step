// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Explanations as `scheduled_outcome` returns them (design §5c), one per
// outcome a story or test shows. Codes, names and times are invented.
import {
    EScheduledOutcomeCheckId as Check,
    EScheduledOutcomeKind as Outcome,
    type IScheduledOutcomeExplanation,
} from "@sequentech/ui-core"

const value = (message_key: string, params?: Record<string, unknown>) => ({message_key, params})
const v = (key: string, params?: Record<string, unknown>) =>
    value(`scheduledOutcome.check.${key}`, params)
const nextStep = (key: string) => value(`scheduledOutcome.nextStep.${key}`)

/** A covered opening: the signed configuration authorizes it. */
export const runsAuthorized = (): IScheduledOutcomeExplanation => ({
    outcome: Outcome.RUNS,
    deciding: Check.COVERED,
    checks: [
        {
            id: Check.NEEDS_SIGNATURES,
            current: v("needsSignatures.yes", {signatures: 2}),
            published: v("needsSignatures.yes", {signatures: 2}),
            allows: true,
        },
        {
            id: Check.COVERED,
            current: v("covered.yes", {code: "K7Q-2M"}),
            published: null,
            allows: true,
        },
        {
            id: Check.STRICTER_COPY,
            current: v("stricterCopy.same"),
            published: null,
            allows: true,
        },
    ],
    next_step: nextStep("none"),
    authorized_by: {
        request_id: "99999999-9999-4999-8999-999999999991",
        code: "K7Q-2M",
        signers: ["Ana P. Reyes", "Jose R. Dela Cruz"],
    },
})

/** An opening edited after its configuration was signed. */
export const refusedEdited = (): IScheduledOutcomeExplanation => ({
    outcome: Outcome.REFUSED,
    deciding: Check.COVERED,
    checks: [
        {
            id: Check.NEEDS_SIGNATURES,
            current: v("needsSignatures.yes", {signatures: 2}),
            published: v("needsSignatures.yes", {signatures: 2}),
            allows: true,
        },
        {
            id: Check.COVERED,
            current: v("covered.changedBy", {
                code: "K7Q-2M",
                edited_at: "2028-04-01T06:30:00Z",
                edited_by: "event.admin",
            }),
            published: null,
            allows: false,
        },
    ],
    next_step: nextStep("publishAndApprove"),
})

/** A close outside the signed configuration, closed at its deadline by policy. */
export const runsUnsigned = (): IScheduledOutcomeExplanation => ({
    outcome: Outcome.RUNS_UNSIGNED,
    deciding: Check.UNSIGNED_CLOSE,
    checks: [
        {
            id: Check.NEEDS_SIGNATURES,
            current: v("needsSignatures.yes", {signatures: 2}),
            published: v("needsSignatures.yes", {signatures: 2}),
            allows: true,
        },
        {
            id: Check.COVERED,
            current: v("covered.notInApproval", {code: "K7Q-2M"}),
            published: null,
            allows: false,
        },
        {
            id: Check.UNSIGNED_CLOSE,
            current: v("unsignedClose.runAsSystem"),
            published: v("unsignedClose.runAsSystem"),
            allows: true,
        },
    ],
    next_step: nextStep("askSignersToClose"),
})

/** A close the current settings would run unsigned, but the published policy refuses. */
export const refusedLooser = (): IScheduledOutcomeExplanation => ({
    outcome: Outcome.REFUSED,
    deciding: Check.STRICTER_COPY,
    checks: [
        {
            id: Check.NEEDS_SIGNATURES,
            current: v("needsSignatures.yes", {signatures: 2}),
            published: v("needsSignatures.yes", {signatures: 2}),
            allows: true,
        },
        {
            id: Check.COVERED,
            current: v("covered.notInApproval", {code: "K7Q-2M"}),
            published: null,
            allows: false,
        },
        {
            id: Check.UNSIGNED_CLOSE,
            current: v("unsignedClose.runAsSystem"),
            published: v("unsignedClose.refuse"),
            allows: false,
        },
        {
            id: Check.STRICTER_COPY,
            current: v("stricterCopy.currentLooser"),
            published: null,
            allows: false,
        },
    ],
    next_step: nextStep("publishAndApprove"),
})

/** Nothing is published: the defaults refuse an opening that needs signatures. */
export const refusedDefaults = (): IScheduledOutcomeExplanation => ({
    outcome: Outcome.REFUSED,
    deciding: Check.DEFAULTS,
    checks: [
        {
            id: Check.NEEDS_SIGNATURES,
            current: v("needsSignatures.yes", {signatures: 2}),
            published: null,
            allows: true,
        },
        {
            id: Check.DEFAULTS,
            current: v("defaults.nothingPublished"),
            published: null,
            allows: false,
        },
    ],
    next_step: nextStep("publishAndApprove"),
})

/** No signatures needed: the scheduler runs it as before. */
export const runsNoSignatures = (): IScheduledOutcomeExplanation => ({
    outcome: Outcome.RUNS,
    deciding: Check.NEEDS_SIGNATURES,
    checks: [
        {
            id: Check.NEEDS_SIGNATURES,
            current: v("needsSignatures.no"),
            published: v("needsSignatures.no"),
            allows: true,
        },
    ],
    next_step: nextStep("none"),
})
