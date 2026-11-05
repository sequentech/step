// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {useNotify, useRefresh} from "react-admin"
import {Alert, Box, Button, Stack, Typography} from "@mui/material"
import {useMutation} from "@apollo/client"
import {useTranslation} from "react-i18next"
import {EScheduledOutcomeKind} from "@sequentech/ui-core"
import type {Sequent_Backend_Scheduled_Event} from "@/gql/graphql"
import {APPLY_SCHEDULE_RECOMPUTE, type ApplyScheduleRecomputeData} from "@/queries/Lifecycle"
import type {
    IRetainedSignedClose,
    IScheduleRecompute,
    IScheduledOutcomeRow,
} from "@/types/lifecycle"
import type {IManageElectionDatePayload} from "@/types/scheduledEvents"
import {IPermissions} from "@/types/keycloak"
import {getGraphQLActionErrorReason} from "@/services/graphqlActionError"
import {RetainedSignedCloses} from "@/components/timezones/RetainedSignedCloses"
import {useTimeZoneService} from "@/components/timezones/timeZoneService"

/** A banner narrows the list to the transitions of one outcome. */
export type EOutcomeFilter = EScheduledOutcomeKind

/** The predicted outcomes of a row: one per Post (an event-wide row has one per Post). */
export type RowOutcomes = ReadonlyMap<string, ReadonlyArray<IScheduledOutcomeRow>>

/** Groups `get_scheduled_outcomes` by scheduled event. */
export const outcomesByRow = (rows: ReadonlyArray<IScheduledOutcomeRow>): RowOutcomes => {
    const grouped = new Map<string, Array<IScheduledOutcomeRow>>()
    for (const row of rows) {
        grouped.set(row.scheduled_event_id, [...(grouped.get(row.scheduled_event_id) ?? []), row])
    }
    return grouped
}

/** Only the transitions at one election, when the list is filtered to it. */
export const outcomesForElection = (
    outcomes: RowOutcomes,
    electionId: string | null | undefined
): RowOutcomes =>
    electionId
        ? new Map(
              Array.from(outcomes.entries())
                  .map(
                      ([id, rows]) =>
                          [id, rows.filter((row) => row.election_id === electionId)] as const
                  )
                  .filter(([, rows]) => rows.length > 0)
          )
        : outcomes

/** How many transitions (row and Post) will have `outcome`. */
export const outcomeCount = (outcomes: RowOutcomes, outcome: EOutcomeFilter): number =>
    Array.from(outcomes.values())
        .flat()
        .filter(({explanation}) => explanation.outcome === outcome).length

/** The scheduled events with at least one Post whose predicted outcome is `outcome`. */
export const outcomeFilterIds = (outcomes: RowOutcomes, outcome: EOutcomeFilter): Array<string> =>
    Array.from(outcomes.entries())
        .filter(([, rows]) => rows.some(({explanation}) => explanation.outcome === outcome))
        .map(([id]) => id)

/** The outcomes a banner counts, the strongest first. */
const COUNTED: ReadonlyArray<{outcome: EOutcomeFilter; severity: "error" | "warning"}> = [
    {outcome: EScheduledOutcomeKind.REFUSED, severity: "error"},
    {outcome: EScheduledOutcomeKind.RUNS_UNSIGNED, severity: "warning"},
]
const COUNT_KEY: Record<string, string> = {
    [EScheduledOutcomeKind.REFUSED]: "refused",
    [EScheduledOutcomeKind.RUNS_UNSIGNED]: "runsUnsigned",
}

/** The tzdata check's banner: future times a timezone update moved, applied only on request. */
/** The pending tz database change of a row, if any. */
export const recomputeOf = (
    event: Pick<Sequent_Backend_Scheduled_Event, "annotations">
): IScheduleRecompute | null =>
    (event.annotations as {schedule_recompute?: IScheduleRecompute} | null | undefined)
        ?.schedule_recompute ?? null

const RecomputeBanner: React.FC<{
    electionEventId: string
    canApply: boolean
    zoneOf: (electionId?: string | null) => string
    scheduledEvents: ReadonlyArray<Sequent_Backend_Scheduled_Event>
}> = ({electionEventId, canApply, zoneOf, scheduledEvents}) => {
    const {t} = useTranslation()
    const service = useTimeZoneService()
    const notify = useNotify()
    const refresh = useRefresh()
    const changed = scheduledEvents.flatMap((event) => {
        const recompute = recomputeOf(event)
        return recompute ? [{event, recompute}] : []
    })
    const [apply, {loading}] = useMutation<ApplyScheduleRecomputeData>(APPLY_SCHEDULE_RECOMPUTE, {
        context: {headers: {"x-hasura-role": IPermissions.SCHEDULED_EVENT_WRITE}},
    })
    const onApply = async () => {
        try {
            const {data} = await apply({variables: {electionEventId}})
            notify(
                t("lifecycle.schedule.recompute.applied", {
                    count: data?.apply_schedule_recompute?.updated ?? 0,
                }),
                {type: "success"}
            )
            refresh()
        } catch (error) {
            notify(getGraphQLActionErrorReason(error) ?? t("lifecycle.schedule.recompute.error"), {
                type: "error",
            })
        }
    }
    return (
        <Alert
            severity="warning"
            data-testid="schedule-recompute"
            action={
                canApply ? (
                    <Button color="inherit" size="small" disabled={loading} onClick={onApply}>
                        {t("lifecycle.schedule.recompute.apply")}
                    </Button>
                ) : undefined
            }
        >
            <Typography variant="body2">
                {t("lifecycle.schedule.recompute.title", {count: changed.length})}
            </Typography>
            <Box component="ul" sx={{m: 0, pl: 2}}>
                {changed.map(({event, recompute}) => {
                    const zone =
                        recompute.timezone ??
                        zoneOf(
                            (event.event_payload as IManageElectionDatePayload | undefined)
                                ?.election_id
                        )
                    return (
                        <li key={String(event.id)}>
                            {t("lifecycle.schedule.recompute.change", {
                                type: t(`eventsScreen.eventType.${event.event_processor}`),
                                before: service.formatDateTimeZone(
                                    recompute.previous,
                                    zone,
                                    service.text
                                ),
                                after: service.formatDateTimeZone(
                                    recompute.scheduled_date,
                                    zone,
                                    service.text
                                ),
                            })}
                        </li>
                    )
                })}
            </Box>
        </Alert>
    )
}

/**
 * Above the Scheduled Events list: a timezone update to apply, the totals of
 * openings and closings that won't run as scheduled (each filters the list),
 * changes voters don't see until the next publication, and times without an
 * offset, which never run.
 */
export const ScheduleBanners: React.FC<{
    electionEventId: string
    outcomes: RowOutcomes
    retainedCloses?: ReadonlyArray<IRetainedSignedClose>
    retainedUnavailable?: boolean
    electionNameOf?: (electionId: string) => string
    filter: EOutcomeFilter | null
    onFilter: (filter: EOutcomeFilter | null) => void
    unpublishedCount: number
    published: boolean
    offsetless: number
    canApply: boolean
    zoneOf: (electionId?: string | null) => string
    scheduledEvents: ReadonlyArray<Sequent_Backend_Scheduled_Event>
}> = ({
    electionEventId,
    outcomes,
    retainedCloses = [],
    retainedUnavailable = false,
    electionNameOf,
    filter,
    onFilter,
    unpublishedCount,
    published,
    offsetless,
    canApply,
    zoneOf,
    scheduledEvents,
}) => {
    const {t} = useTranslation()
    // Rows, and the election transitions they make (an event-wide row makes one per election).
    const totals = COUNTED.map((counted) => ({
        ...counted,
        count: outcomeFilterIds(outcomes, counted.outcome).length,
        transitions: outcomeCount(outcomes, counted.outcome),
    })).filter(({count}) => count > 0)
    return (
        <Stack spacing={1} sx={{mx: 2, mb: 1}} data-testid="schedule-banners">
            <RetainedSignedCloses
                closes={retainedCloses.filter((close) => !close.fired_at)}
                zoneOf={zoneOf}
                nameOf={electionNameOf ?? (() => t("eventsScreen.fields.electionId"))}
                unavailable={retainedUnavailable}
            />
            {scheduledEvents.some((event) => recomputeOf(event)) ? (
                <RecomputeBanner
                    electionEventId={electionEventId}
                    canApply={canApply}
                    zoneOf={zoneOf}
                    scheduledEvents={scheduledEvents}
                />
            ) : null}
            {filter ? (
                <Alert
                    severity="info"
                    action={
                        <Button color="inherit" size="small" onClick={() => onFilter(null)}>
                            {t("lifecycle.schedule.totals.showAll")}
                        </Button>
                    }
                >
                    {t(`lifecycle.schedule.totals.showing.${COUNT_KEY[filter]}`, {
                        count: outcomeFilterIds(outcomes, filter).length,
                        transitions: outcomeCount(outcomes, filter),
                    })}
                </Alert>
            ) : (
                totals.map(({outcome, severity, count, transitions}) => (
                    <Alert
                        key={outcome}
                        severity={severity}
                        data-testid={`schedule-totals-${outcome}`}
                        action={
                            <Button color="inherit" size="small" onClick={() => onFilter(outcome)}>
                                {t("lifecycle.schedule.totals.review")}
                            </Button>
                        }
                    >
                        {t(`lifecycle.schedule.totals.${COUNT_KEY[outcome]}`, {count, transitions})}
                    </Alert>
                ))
            )}
            {!published ? (
                <Alert severity="info">{t("lifecycle.schedule.notPublished")}</Alert>
            ) : unpublishedCount > 0 ? (
                <Alert severity="info" data-testid="schedule-unpublished">
                    {t("lifecycle.schedule.unpublishedChanges", {count: unpublishedCount})}
                </Alert>
            ) : null}
            {offsetless > 0 ? (
                <Alert severity="warning" data-testid="schedule-offsetless">
                    {t("lifecycle.schedule.offsetless", {count: offsetless})}
                </Alert>
            ) : null}
        </Stack>
    )
}
