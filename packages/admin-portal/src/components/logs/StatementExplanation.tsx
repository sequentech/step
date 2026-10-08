// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {Box, Typography} from "@mui/material"
import {useTranslation} from "react-i18next"
import type {TFunction} from "i18next"
import type {IScheduledOutcomeExplanation, IScheduledOutcomeValue} from "@sequentech/ui-core"
import {logDetails, outcomeChange, outcomeExplanation, type ILogMessage} from "./logMessage"

/** A check or next-step value: an i18n key with its parameters. */
const valueText = (t: TFunction, value?: IScheduledOutcomeValue | null): string =>
    value?.message_key ? String(t(value.message_key, value.params ?? {})) : ""

const outcomeText = (t: TFunction, explanation: IScheduledOutcomeExplanation): string =>
    String(t(`logsScreen.scheduledOutcome.outcome.${explanation.outcome}`))

/** The reasons of an explanation: the deciding check, the authorization and the next step. */
const reasons = (t: TFunction, explanation: IScheduledOutcomeExplanation): string[] => {
    const deciding = explanation.checks.find((check) => check.id === explanation.deciding)
    const lines = [
        String(
            t("logsScreen.scheduledOutcome.deciding", {
                check: t(`logsScreen.scheduledOutcome.check.${explanation.deciding}`),
                value: valueText(t, deciding?.current),
            })
        ),
    ]
    if (explanation.authorized_by?.code) {
        lines.push(
            String(
                t("logsScreen.scheduledOutcome.authorizedBy", {
                    code: explanation.authorized_by.code,
                })
            )
        )
    }
    const nextStep = valueText(t, explanation.next_step)
    if (nextStep) {
        lines.push(String(t("logsScreen.scheduledOutcome.nextStep", {step: nextStep})))
    }
    return lines
}

/** The Seal at close reasons a scheduled outcome records in `details.reason`. */
const SEAL_REASONS = ["ballot-box-seal-policy", "never-opened-kept-open"]

/** Why a scheduled outcome left a Post as it was (VOTE-FREEZE), in the admin's language. */
const sealReason = (t: TFunction, details: Record<string, unknown> | null): string | null => {
    const reason = details?.reason
    return typeof reason === "string" && SEAL_REASONS.includes(reason)
        ? String(t(`logsScreen.scheduledOutcome.reason.${reason}`))
        : null
}

type Renderer = (
    t: TFunction,
    details: Record<string, unknown> | null,
    message: ILogMessage | null
) => string[] | null

/** The values of a statement body variant, e.g. `{"BallotBoxSealed": [...]}`. */
const bodyValues = (message: ILogMessage | null, kind: string): unknown[] | null => {
    const body = message?.statement?.body
    if (!body || typeof body !== "object") return null
    const values = (body as Record<string, unknown>)[kind]
    return Array.isArray(values) ? values : null
}

const count = (value: unknown): number => (typeof value === "number" ? value : Number(value) || 0)

/**
 * Readable lines for the ballot box seal entries (VOTE-FREEZE): the seal
 * hash and the counts the signed statement carries, the Close voting request,
 * the reason a seal failed, and what a tally found.
 */
const SEAL_RENDERERS: Record<string, Renderer> = {
    BallotBoxSealed: (t, _details, message) => {
        const values = bodyValues(message, "BallotBoxSealed")
        if (!values) return null
        const [, , hash, inBox, counted, request] = values
        return [
            String(t("logsScreen.ballotBoxSeal.sealHash", {hash: String(hash ?? "")})),
            String(
                t("logsScreen.ballotBoxSeal.counted", {
                    counted: count(counted),
                    inBox: count(inBox),
                })
            ),
            request
                ? String(t("logsScreen.ballotBoxSeal.closeRequest", {request: String(request)}))
                : String(t("logsScreen.ballotBoxSeal.noCloseRequest")),
        ]
    },
    BallotBoxSealFailed: (t, _details, message) => {
        const values = bodyValues(message, "BallotBoxSealFailed")
        if (!values) return null
        return [
            String(t("logsScreen.ballotBoxSeal.failedReason", {reason: String(values[2] ?? "")})),
            String(t("logsScreen.ballotBoxSeal.failedLocked")),
        ]
    },
    TallyBallotBoxVerified: (t, _details, message) => {
        const values = bodyValues(message, "TallyBallotBoxVerified")
        if (!values) return null
        const [, , hash, counted, session] = values
        return [
            String(t("logsScreen.ballotBoxSeal.sealHash", {hash: String(hash ?? "")})),
            String(t("logsScreen.ballotBoxSeal.verifiedCounted", {counted: count(counted)})),
            String(t("logsScreen.ballotBoxSeal.tallySession", {session: String(session ?? "")})),
        ]
    },
    TallyBallotBoxRejected: (t, _details, message) => {
        const values = bodyValues(message, "TallyBallotBoxRejected")
        if (!values) return null
        const [, , differs, session] = values
        return [
            String(t("logsScreen.ballotBoxSeal.differs", {differs: String(differs ?? "")})),
            String(t("logsScreen.ballotBoxSeal.tallySession", {session: String(session ?? "")})),
        ]
    },
}

/**
 * Readable lines for the entries whose details explain a scheduled outcome,
 * keyed by statement kind. Other kinds show only their description.
 */
const RENDERERS: Record<string, Renderer> = {
    ScheduledOutcomeChanged: (t, details) => {
        const change = outcomeChange(details)
        if (!change) return null
        return [
            String(
                t("logsScreen.scheduledOutcome.changed", {
                    after: outcomeText(t, change.after),
                    before: outcomeText(t, change.before),
                })
            ),
            ...reasons(t, change.after),
        ]
    },
    ...SEAL_RENDERERS,
    SigningActionExecuted: (t, details) => {
        const explanation = outcomeExplanation(details)
        const reason = sealReason(t, details)
        if (!explanation && !reason) return null
        return [
            ...(explanation
                ? [
                      String(
                          t("logsScreen.scheduledOutcome.result", {
                              outcome: outcomeText(t, explanation),
                          })
                      ),
                      ...reasons(t, explanation),
                  ]
                : []),
            ...(reason ? [reason] : []),
        ]
    },
}

/** The explanation lines of a log row, or null when its kind has none. */
export const explanationLines = (
    t: TFunction,
    kind: string | null | undefined,
    message: ILogMessage | null
): string[] | null => {
    const renderer = kind ? RENDERERS[kind] : undefined
    return renderer ? renderer(t, logDetails(message), message) : null
}

export const StatementExplanation: React.FC<{
    kind?: string | null
    message: ILogMessage | null
}> = ({kind, message}) => {
    const {t} = useTranslation()
    const lines = explanationLines(t, kind, message)
    if (!lines?.length) return null
    return (
        <Box component="ul" sx={{m: 0, mt: 0.5, pl: 2}} className="log-explanation">
            {lines.map((line) => (
                <Typography
                    component="li"
                    variant="body2"
                    key={line}
                    sx={{overflowWrap: "anywhere"}}
                >
                    {line}
                </Typography>
            ))}
        </Box>
    )
}
