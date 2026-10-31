// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {Stack, Typography} from "@mui/material"
import {useTranslation} from "react-i18next"
import type {IScheduledOutcomeExplanation} from "@sequentech/ui-core"
import type {IScheduledOutcomeRow} from "@/types/lifecycle"
import {ScheduledOutcome} from "@/components/timezones/ScheduledOutcome"

/** Explanations that read the same: the same outcome, decided by the same check. */
export const groupByReason = <T extends {explanation: IScheduledOutcomeExplanation}>(
    outcomes: ReadonlyArray<T>
): Array<Array<T>> => {
    const groups = new Map<string, Array<T>>()
    for (const outcome of outcomes) {
        const key = `${outcome.explanation.outcome}|${outcome.explanation.deciding}`
        groups.set(key, [...(groups.get(key) ?? []), outcome])
    }
    return Array.from(groups.values())
}

/**
 * The predicted outcome of one scheduled opening or closing. An event-wide
 * row has one per election: the elections are grouped by outcome and deciding
 * check, each group with its count and the reason of its first election.
 * Times are in the row's zone.
 */
export const RowOutcome: React.FC<{
    outcomes: ReadonlyArray<IScheduledOutcomeRow>
    /** The row's zone: its election's, else the event's primary. */
    zone: string
}> = ({outcomes, zone}) => {
    const {t} = useTranslation()
    const several = outcomes.length > 1
    return (
        <Stack spacing={1}>
            {groupByReason(outcomes).map((group) => (
                <Stack
                    key={`${group[0].explanation.outcome}-${group[0].explanation.deciding}`}
                    spacing={0.25}
                >
                    <ScheduledOutcome explanation={group[0].explanation} zone={zone} />
                    {several ? (
                        <Typography variant="caption" color="text.secondary">
                            {t("lifecycle.schedule.outcomeElections", {
                                count: group.length,
                                total: outcomes.length,
                            })}
                        </Typography>
                    ) : null}
                </Stack>
            ))}
        </Stack>
    )
}
