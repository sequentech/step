// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {ReactElement, useMemo, useState} from "react"
import {
    DatagridConfigurable,
    List,
    TextField,
    FunctionField,
    NumberField,
    useRecordContext,
    TextInput,
    DateTimeInput,
    SelectInput,
} from "react-admin"
import {ListActions} from "@/components/ListActions"
import {useTranslation} from "react-i18next"
import {timeZoneOption} from "@sequentech/ui-core"
import {Sequent_Backend_Election, Sequent_Backend_Election_Event} from "@/gql/graphql"
import {ResetFilters} from "./ResetFilters"
import {useLogsPermissions} from "@/resources/ElectionEvent/useLogsPermissions"
import {MessageField} from "./MessageField"
import {ThreeStateDatagridHeader} from "./ThreeStateDatagridHeader"
import {useZonedFormat, type IZonedFormat} from "@/hooks/useZonedFormat"
import {
    ELECTORAL_LOG_DEFAULT_ZONE_FILTER,
    ELECTORAL_LOG_ZONE_FILTER,
} from "@/queries/ListElectoralLog"
import {logMessage} from "./logs/logMessage"
import {LogTime} from "./logs/LogTime"
import {StatementExplanation} from "./logs/StatementExplanation"
import {ExportLogsDialog} from "./logs/ExportLogsDialog"
import {useLogRowZone, useLogZones, type ILogZones} from "./logs/useLogZones"

const OMIT_FIELDS = ["user_id"]

/** A row's Created or Statement Timestamp, in the row's log zone with my time below. */
const LogRowTime: React.FC<{
    source: "created" | "statement_timestamp"
    zones: ILogZones
    format: IZonedFormat
}> = ({source, zones, format}) => {
    const record = useRecordContext<{
        created?: number
        statement_timestamp?: number
        message?: string
    }>()
    const zoneOf = useLogRowZone(zones)
    return (
        <LogTime
            seconds={record?.[source]}
            zone={zoneOf(logMessage(record)?.election_id)}
            format={format}
        />
    )
}

export interface ElectoralLogListProps {
    aside?: ReactElement
    filterToShow?: ElectoralLogFilters
    filterValue?: string
    electionEventId?: string
    showActions?: boolean
}

export enum ElectoralLogFilters {
    ID = "id",
    STATEMENT_KIND = "statement_kind",
    USER_ID = "user_id",
    USERNAME = "username",
}

export const ElectoralLogList: React.FC<ElectoralLogListProps> = ({
    aside,
    filterToShow,
    filterValue,
    electionEventId,
    showActions = true,
}) => {
    const record = useRecordContext<Sequent_Backend_Election_Event | Sequent_Backend_Election>()
    const {t, i18n} = useTranslation()
    const eventId = electionEventId || record?.id || undefined

    const {canExportLogs, showLogsColumns} = useLogsPermissions()
    // In the event's Logs tab the record is the event; elsewhere, the screen's event.
    const zones = useLogZones(record?.id && record.id === eventId ? record : eventId)
    const format = useZonedFormat(zones.primary, {seconds: true})

    const getHeadField = (record: any, field: string) => {
        const value = (
            logMessage(record)?.statement?.head as Record<string, unknown> | undefined
        )?.[field]
        return value ? String(value) : <span>-</span>
    }

    const [openExport, setOpenExport] = useState(false)

    // Without a chosen zone, the range is read in the primary (also after a filter reset).
    const filterObject: {[key: string]: any} = {
        election_event_id: eventId,
        [ELECTORAL_LOG_DEFAULT_ZONE_FILTER]: zones.primary,
    }

    if (filterToShow) {
        filterObject[filterToShow] = filterValue || undefined
    }

    // The primary is the select's empty option: the range's default zone.
    const zoneName = (zone: string) => timeZoneOption(zone, {t, lang: i18n.language}).label
    const zoneChoices = useMemo(
        () =>
            zones.choices
                .filter((zone) => zone !== zones.primary)
                .map((zone) => ({id: zone, name: zoneName(zone)})),
        [zones.choices, zones.primary, t, i18n.language]
    )

    // Range filters hold wall times in the chosen zone; the data provider
    // turns them into instants.
    const filters: Array<ReactElement> = [
        <DateTimeInput
            key="created_from"
            source="created_from"
            label={String(t("logsScreen.filter.createdFrom"))}
            alwaysOn
        />,
        <DateTimeInput
            key="created_to"
            source="created_to"
            label={String(t("logsScreen.filter.createdTo"))}
            alwaysOn
        />,
        <SelectInput
            key={ELECTORAL_LOG_ZONE_FILTER}
            source={ELECTORAL_LOG_ZONE_FILTER}
            label={String(t("logsScreen.filter.timeZone"))}
            choices={zoneChoices}
            emptyText={zoneName(zones.primary)}
            alwaysOn
        />,
        <TextInput
            key={"user_id"}
            source={"user_id"}
            label={String(t("logsScreen.column.user_id"))}
        />,
        <TextInput
            key={"username"}
            source={"username"}
            label={String(t("logsScreen.column.username"))}
        />,
        <DateTimeInput
            key="statement_timestamp_from"
            source="statement_timestamp_from"
            label={String(t("logsScreen.filter.statementTimestampFrom"))}
        />,
        <DateTimeInput
            key="statement_timestamp_to"
            source="statement_timestamp_to"
            label={String(t("logsScreen.filter.statementTimestampTo"))}
        />,
        <TextInput
            key={"statement_kind"}
            source={"statement_kind"}
            label={String(t("logsScreen.column.statement_kind"))}
        />,
    ]

    return (
        <>
            <List
                resource="electoral_log"
                actions={
                    showActions && (
                        <ListActions
                            withColumns={showLogsColumns}
                            withImport={false}
                            doExport={() => setOpenExport(true)}
                            withExport={canExportLogs}
                            withFilter={true}
                        />
                    )
                }
                filters={filters}
                filter={filterObject}
                storeKey={false}
                sort={{
                    field: "id",
                    order: "DESC",
                }}
                aside={aside}
            >
                <ResetFilters />
                <DatagridConfigurable
                    header={ThreeStateDatagridHeader}
                    omit={OMIT_FIELDS}
                    bulkActionButtons={false}
                >
                    <NumberField source="id" label={String(t("logsScreen.column.id"))} />
                    <FunctionField
                        source="user_id"
                        label={String(t("logsScreen.column.user_id"))}
                        render={(record: any) => {
                            const userId = logMessage(record)?.user_id
                            return (
                                <span style={{display: "block", textAlign: "center"}}>
                                    {!userId || userId === "null" ? <span>-</span> : userId}
                                </span>
                            )
                        }}
                    />
                    <FunctionField
                        source="username"
                        label={String(t("logsScreen.column.username"))}
                        render={(record: any) => {
                            const username = logMessage(record)?.username
                            return (
                                <span style={{display: "block", textAlign: "center"}}>
                                    {!username || username === "null" ? <span>-</span> : username}
                                </span>
                            )
                        }}
                    />
                    <FunctionField
                        source="created"
                        label={String(t("logsScreen.column.created"))}
                        render={() => <LogRowTime source="created" zones={zones} format={format} />}
                    />
                    <FunctionField
                        source="statement_timestamp"
                        label={String(t("logsScreen.column.statement_timestamp"))}
                        render={() => (
                            <LogRowTime
                                source="statement_timestamp"
                                zones={zones}
                                format={format}
                            />
                        )}
                    />
                    <TextField
                        source="statement_kind"
                        label={String(t("logsScreen.column.statement_kind"))}
                    />
                    <FunctionField
                        source="event_type"
                        label={String(t("logsScreen.column.event_type"))}
                        render={(record: any) => getHeadField(record, "event_type")}
                    />
                    <FunctionField
                        source="log_type"
                        label={String(t("logsScreen.column.log_type"))}
                        render={(record: any) => getHeadField(record, "log_type")}
                    />
                    <FunctionField
                        source="description"
                        label={String(t("logsScreen.column.description"))}
                        render={(record: any) => (
                            <>
                                <MessageField
                                    content={
                                        logMessage(record)?.statement?.head?.description || "-"
                                    }
                                    initialLength={50}
                                />
                                <StatementExplanation
                                    kind={record.statement_kind}
                                    message={logMessage(record)}
                                />
                            </>
                        )}
                    />
                    <MessageField source="message" />
                </DatagridConfigurable>
            </List>
            <ExportLogsDialog
                electionEventId={eventId ?? ""}
                open={openExport}
                onClose={() => setOpenExport(false)}
                zones={zones.choices}
                byElection={zones.byElection}
            />
        </>
    )
}
