// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext} from "react"
import {Alert, Box} from "@mui/material"
import {useGetList} from "react-admin"
import {useTranslation} from "react-i18next"
import {effectiveTimeZone} from "@sequentech/ui-core"
import type {Sequent_Backend_Election_Event, Sequent_Backend_Scheduled_Event} from "@/gql/graphql"
import {AuthContext} from "@/providers/AuthContextProvider"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {IPermissions} from "@/types/keycloak"
import type {ICronConfig} from "@/types/scheduledEvents"
import type {ILogRow} from "@/components/logs/LogEntryDialog"
import {logDetails, logMessage} from "@/components/logs/logMessage"
import {useTimeZoneService} from "@/components/timezones/timeZoneService"
import {hasOffset} from "@/components/timezones/ZonedDateTime"
import {ACTIVE_SCHEDULED_EVENTS} from "@/services/ScheduledEvent"
import {EventProcessors} from "../ScheduledEvents/CreateScheduledEvent"
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
    const {data: ends, isSuccess: scheduleLoaded} = useGetList<Sequent_Backend_Scheduled_Event>(
        "sequent_backend_scheduled_event",
        {
            filter: {
                tenant_id: event.tenant_id,
                election_event_id: event.id,
                event_processor: EventProcessors.END_LOCKDOWN_PERIOD,
                ...ACTIVE_SCHEDULED_EVENTS,
                stopped_at: {format: "hasura-raw-query", value: {_is_null: true}},
            },
            pagination: {page: 1, perPage: 1000},
            sort: {field: "id", order: "ASC"},
        },
        {enabled: canReadSchedule, refetchInterval: globalSettings.QUERY_POLL_INTERVAL_MS}
    )
    const started = lockdownStarted(logs ?? [])
    const endDates = (ends ?? [])
        .map((row) => (row.cron_config as ICronConfig | null)?.scheduled_date)
        .filter((date): date is string => typeof date === "string" && hasOffset(date))
        .map((date) => new Date(date))
        .filter((date) => !Number.isNaN(date.getTime()))
        .sort((a, b) => a.getTime() - b.getTime())
    const end = endDates[0]
    const prefix = "electionEventScreen.field.lockdownState"

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
                    {end
                        ? t(`${prefix}.scheduledEnd`, {date: format(end)})
                        : t(ends?.length ? `${prefix}.endScheduled` : `${prefix}.noScheduledEnd`)}
                </>
            )}
        </Alert>
    )
}
