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

type Renderer = (t: TFunction, details: Record<string, unknown> | null) => string[] | null

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
    SigningActionExecuted: (t, details) => {
        const explanation = outcomeExplanation(details)
        if (!explanation) return null
        return [
            String(t("logsScreen.scheduledOutcome.result", {outcome: outcomeText(t, explanation)})),
            ...reasons(t, explanation),
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
    return renderer ? renderer(t, logDetails(message)) : null
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
                <Typography component="li" variant="body2" key={line}>
                    {line}
                </Typography>
            ))}
        </Box>
    )
}
