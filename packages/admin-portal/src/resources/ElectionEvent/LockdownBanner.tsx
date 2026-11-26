// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext, useState} from "react"
import {Alert, Box, Button, Drawer} from "@mui/material"
import {useGetList, useNotify, useRefresh} from "react-admin"
import {useMutation} from "@apollo/client"
import {MANAGE_ELECTION_DATES} from "@/queries/ManageElectionDates"
import {getGraphQLActionErrorReason} from "@/services/graphqlActionError"
import {useTranslation} from "react-i18next"
import {effectiveTimeZone} from "@sequentech/ui-core"
import type {
    ManageElectionDatesMutation,
    ManageElectionDatesMutationVariables,
    Sequent_Backend_Election_Event,
    Sequent_Backend_Scheduled_Event,
} from "@/gql/graphql"
import {AuthContext} from "@/providers/AuthContextProvider"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {IPermissions} from "@/types/keycloak"
import type {ICronConfig} from "@/types/scheduledEvents"
import type {ILogRow} from "@/components/logs/LogEntryDialog"
import {logDetails, logMessage} from "@/components/logs/logMessage"
import {useTimeZoneService} from "@/components/timezones/timeZoneService"
import {hasOffset} from "@/components/timezones/ZonedDateTime"
import {ACTIVE_SCHEDULED_EVENTS} from "@/services/ScheduledEvent"
import CreateEvent, {EventProcessors} from "../ScheduledEvents/CreateScheduledEvent"
import {useLogsPermissions} from "./useLogsPermissions"

/** Only a genuine transition starts the current period; repeated starts do not. */
function lockdownStarted(logs: ILogRow[]): Date | null {
    for (const log of logs) {
        const details = logDetails(logMessage(log))
        if (details?.locked_down !== true) return null
        if (details.previous === false) {
            const seconds = log.statement_timestamp
            if (typeof seconds !== "number" || !Number.isFinite(seconds)) return null
            const date = new Date(seconds * 1000)
            return Number.isNaN(date.getTime()) ? null : date
        }
    }
    return null
}

/** Mounted only while the event's authoritative state is locked down. */
export const LockdownBanner: React.FC<{event: Sequent_Backend_Election_Event}> = ({event}) => {
    const {t} = useTranslation()
    const auth = useContext(AuthContext)
    const [editingEnd, setEditingEnd] = useState(false)
    const notify = useNotify()
    const refresh = useRefresh()
    const [manageDates, {loading: cancelling}] = useMutation<
        ManageElectionDatesMutation,
        ManageElectionDatesMutationVariables
    >(MANAGE_ELECTION_DATES, {
        context: {headers: {"x-hasura-role": IPermissions.SCHEDULED_EVENT_WRITE}},
    })
    const {globalSettings} = useContext(SettingsContext)
    const {canReadLogs} = useLogsPermissions()
    const time = useTimeZoneService()
    const zone = effectiveTimeZone(event.presentation)
    const format = (date: Date) => time.formatDateTimeZone(date, zone, time.text)
    const canReadSchedule = auth.isAuthorized(true, auth.tenantId, [
        IPermissions.ADMIN_USER,
        IPermissions.SCHEDULED_EVENT_CREATE,
        IPermissions.SCHEDULED_EVENT_WRITE,
        IPermissions.SCHEDULED_EVENT_DELETE,
    ])
    const {data: logs} = useGetList<ILogRow>(
        "electoral_log",
        {
            filter: {election_event_id: event.id, statement_kind: "LockdownChanged"},
            pagination: {page: 1, perPage: 100},
            sort: {field: "id", order: "DESC"},
        },
        {enabled: canReadLogs, refetchInterval: globalSettings.QUERY_POLL_INTERVAL_MS}
    )
    const {
        data: ends,
        total,
        isSuccess: scheduleLoaded,
    } = useGetList<Sequent_Backend_Scheduled_Event>(
        "sequent_backend_scheduled_event",
        {
            filter: {
                tenant_id: event.tenant_id,
                election_event_id: event.id,
                event_processor: EventProcessors.END_LOCKDOWN_PERIOD,
                ...ACTIVE_SCHEDULED_EVENTS,
                stopped_at: {format: "hasura-raw-query", value: {_is_null: true}},
            },
            pagination: {page: 1, perPage: 2},
            sort: {field: "id", order: "ASC"},
        },
        {enabled: canReadSchedule, refetchInterval: globalSettings.QUERY_POLL_INTERVAL_MS}
    )
    const started = lockdownStarted(logs ?? [])
    const row = ends?.[0]
    const taskId = `tenant_${event.tenant_id}_event_${event.id}_END_LOCKDOWN_PERIOD`
    const payload = row?.event_payload
    const eventWide =
        (row?.task_id == null || row.task_id === taskId) &&
        (payload == null ||
            (typeof payload === "object" && !Array.isArray(payload) && payload.election_id == null))
    const instant = (row?.cron_config as ICronConfig | null)?.scheduled_date
    const end =
        eventWide && typeof instant === "string" && hasOffset(instant) ? new Date(instant) : null
    const ambiguous =
        (total ?? ends?.length ?? 0) > 1 || (!!row && (!end || Number.isNaN(end.getTime())))
    const prefix = "electionEventScreen.field.lockdownState"
    const canManageEnd =
        scheduleLoaded &&
        !event.is_archived &&
        !event.elections?.some(
            (post) =>
                post.permission_label && !auth.permissionLabels.includes(post.permission_label)
        ) &&
        auth.isAuthorized(true, event.tenant_id, IPermissions.SCHEDULED_EVENT_WRITE)
    const canScheduleEnd = canManageEnd && !ambiguous

    const cancelEnd = async () => {
        try {
            const {data} = await manageDates({
                variables: {
                    electionEventId: event.id,
                    electionId: null,
                    eventProcessor: EventProcessors.END_LOCKDOWN_PERIOD,
                    scheduledDate: null,
                },
            })
            const error = data?.manage_election_dates?.error_msg
            if (error) notify(error, {type: "error"})
            refresh()
        } catch (error) {
            notify(getGraphQLActionErrorReason(error) ?? t("eventsScreen.messages.editError"), {
                type: "error",
            })
        }
    }

    return (
        <Alert
            severity="warning"
            icon={false}
            role="status"
            aria-label={t(`${prefix}.policyLabel`)}
            sx={{mx: 2, mb: 2, border: "1px solid", borderColor: "warning.light"}}
        >
            <Box component="strong">
                {started ? t(`${prefix}.since`, {date: format(started)}) : t(`${prefix}.banner`)}
            </Box>
            {canReadSchedule && scheduleLoaded && (
                <>
                    {" "}
                    {ambiguous
                        ? t(`${prefix}.ambiguousEnd`)
                        : end
                          ? t(`${prefix}.scheduledEnd`, {date: format(end)})
                          : t(`${prefix}.noScheduledEnd`)}
                </>
            )}
            {canScheduleEnd && (
                <Button size="small" disabled={cancelling} onClick={() => setEditingEnd(true)}>
                    {t(end ? `${prefix}.replaceEnd` : `${prefix}.scheduleEnd`)}
                </Button>
            )}
            {canManageEnd && (ambiguous || end) && (
                <Button size="small" disabled={cancelling} onClick={cancelEnd}>
                    {t(ambiguous ? `${prefix}.cancelInvalidEnds` : `${prefix}.cancelEnd`)}
                </Button>
            )}
            {canScheduleEnd && editingEnd && (
                <Drawer
                    anchor="right"
                    open
                    onClose={() => setEditingEnd(false)}
                    sx={{"& .MuiDrawer-paper": {width: {xs: "100%", md: "50%"}}}}
                >
                    <CreateEvent
                        electionEventId={event.id}
                        fixedProcessor={EventProcessors.END_LOCKDOWN_PERIOD}
                        getElectionName={() => t("lifecycle.schedule.allElections")}
                        setIsOpenDrawer={setEditingEnd}
                    />
                </Drawer>
            )}
        </Alert>
    )
}
