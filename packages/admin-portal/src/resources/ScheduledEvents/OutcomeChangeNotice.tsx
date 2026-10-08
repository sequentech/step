// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {Alert, Stack, Typography} from "@mui/material"
import {useQuery} from "@apollo/client"
import {useTranslation} from "react-i18next"
import {
    PREVIEW_SCHEDULED_OUTCOME_CHANGE,
    type PreviewScheduledOutcomeChangeData,
} from "@/queries/Lifecycle"
import type {IPendingLifecycleChange, IScheduledOutcomeChange} from "@/types/lifecycle"
import {OUTCOME_KEY, ScheduledOutcome} from "@/components/timezones/ScheduledOutcome"
import {EScheduledOutcomeKind} from "@sequentech/ui-core"

/**
 * Before saving a scheduled opening or closing: what saving does to its
 * outcome (design §5c), e.g. that an edit takes it out of the signed
 * configuration, with the new outcome's "Why?". Nothing is shown when
 * saving changes no outcome.
 */
export const OutcomeChangeNotice: React.FC<{
    electionEventId: string
    change: NonNullable<IPendingLifecycleChange["scheduled_event"]>
    /** The row's zone, for times in the explanation. */
    zone: string
}> = ({electionEventId, change, zone}) => {
    const {data} = useQuery<PreviewScheduledOutcomeChangeData>(PREVIEW_SCHEDULED_OUTCOME_CHANGE, {
        variables: {electionEventId, change: {scheduled_event: change}},
        fetchPolicy: "network-only",
    })
    const changes = data?.preview_scheduled_outcome_change?.changes ?? []
    return <OutcomeChanges changes={changes} zone={zone} />
}

export const OutcomeChanges: React.FC<{
    changes: ReadonlyArray<IScheduledOutcomeChange>
    zone: string
}> = ({changes, zone}) => {
    const {t} = useTranslation()
    if (!changes.length) return null
    const chip = (explanation: IScheduledOutcomeChange["after"]) =>
        t(`scheduledOutcome.chip.${OUTCOME_KEY[explanation.outcome]}`)
    // An event-wide row changes at every election: one notice, grouped by reason.
    const groups = groupChanges(changes)
    const refused = changes.some(({after}) => after.outcome === EScheduledOutcomeKind.REFUSED)
    return (
        <Alert
            severity={refused ? "warning" : "info"}
            sx={{width: "100%"}}
            data-testid="outcome-change-notice"
        >
            {changes.length > 1 ? (
                <Typography variant="body2" sx={{mb: 1}}>
                    {t("lifecycle.schedule.outcomeChangeElections", {count: changes.length})}
                </Typography>
            ) : null}
            <Stack spacing={1}>
                {groups.map((group) => {
                    const {before, after} = group[0]
                    return (
                        <Stack
                            key={`${before?.outcome ?? "new"}-${after.outcome}-${after.deciding}`}
                            spacing={0.5}
                        >
                            <Typography variant="body2">
                                {before
                                    ? t("lifecycle.schedule.outcomeChange", {
                                          before: chip(before),
                                          after: chip(after),
                                      })
                                    : t("lifecycle.schedule.outcomeNew", {after: chip(after)})}
                                {changes.length > 1
                                    ? ` ${t("lifecycle.schedule.outcomeElections", {
                                          count: group.length,
                                          total: changes.length,
                                      })}`
                                    : null}
                            </Typography>
                            <ScheduledOutcome explanation={after} zone={zone} />
                        </Stack>
                    )
                })}
            </Stack>
        </Alert>
    )
}

/** Changes that read the same: the same outcome before and after, decided by the same check. */
export const groupChanges = (
    changes: ReadonlyArray<IScheduledOutcomeChange>
): Array<Array<IScheduledOutcomeChange>> => {
    const groups = new Map<string, Array<IScheduledOutcomeChange>>()
    for (const change of changes) {
        const key = `${change.before?.outcome ?? "new"}|${change.after.outcome}|${change.after.deciding}`
        groups.set(key, [...(groups.get(key) ?? []), change])
    }
    return Array.from(groups.values())
}
