// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {ReactElement, useState} from "react"
import {
    DatagridConfigurable,
    List,
    Button,
    TextField,
    FunctionField,
    NumberField,
    useListContext,
    useRecordContext,
    TextInput,
    SelectInput,
    useInput,
} from "react-admin"
import {Box, ButtonBase, Chip, Typography} from "@mui/material"
import {LogRangeDateTimeInput} from "./logs/LogRangeDateTimeInput"
import {ListActions} from "@/components/ListActions"
import {useTranslation} from "react-i18next"
import {Sequent_Backend_Election, Sequent_Backend_Election_Event} from "@/gql/graphql"
import {ChannelLabel, Dialog} from "@sequentech/ui-essentials"
import {AUDIT_ELECTORAL_LOG} from "@/queries/AuditElectoralLog"
import {useMutation} from "@apollo/client"
import {IPermissions} from "@/types/keycloak"
import {ResetFilters} from "./ResetFilters"
import FactCheckIcon from "@mui/icons-material/FactCheck"
import {useWidgetStore} from "@/providers/WidgetsContextProvider"
import {ETasksExecution} from "@/types/tasksExecution"
import {useLogsPermissions} from "@/resources/ElectionEvent/useLogsPermissions"
import {MessageField} from "./MessageField"
import {ThreeStateDatagridHeader} from "./ThreeStateDatagridHeader"
import {electoralLogChannel} from "./electoralLogChannel"
import {useZonedFormat, type IZonedFormat} from "@/hooks/useZonedFormat"
import {useContainerWidth} from "@/hooks/useContainerWidth"
import {
    ELECTORAL_LOG_DEFAULT_ZONE_FILTER,
    ELECTORAL_LOG_ZONE_FILTER,
} from "@/queries/ListElectoralLog"
import type {TFunction} from "i18next"
import {readinessDescription} from "./logs/readinessLog"
import {configurationPackageDescription} from "./logs/configurationPackageLog"
import {ballotLogDetails} from "./logs/ballotLogDetails"
import {logMessage} from "./logs/logMessage"
import {LogTime} from "./logs/LogTime"
import {ExportLogsDialog} from "./logs/ExportLogsDialog"
import {LogEntryDialog, type ILogRow} from "./logs/LogEntryDialog"
import {useLogRowZone, useLogZones, useTenantLogZones, type ILogZones} from "./logs/useLogZones"
import {TimeZonePicker} from "./timezones/TimeZonePicker"
import {LedgerStates, LedgerStatusField} from "./logs/LedgerStatus"
import {LOG_CHANNEL_NONE, LOG_FILTER_CHANNEL, LOG_FILTER_OUTCOME} from "./logs/messagingLogFilters"
import {
    ELogListLayout,
    ELogScope,
    LOG_EVENT_TYPES,
    LOG_FILTER_EVENT_TYPE,
    LOG_FILTER_LOG_TYPE,
    LOG_SCOPE_META,
    LOG_TYPES,
    TENANT_LOG_STATEMENT_KINDS,
    logListLayout,
    type ILogTarget,
} from "./logs/logScope"
import {MESSAGE_ATTEMPT_STATES, MESSAGE_CHANNELS} from "@/types/messaging"

/** Columns that start hidden under Columns, whatever the width. */
const OMIT_FIELDS = ["user_id", "message"]
const LOG_TYPE_ERROR = "ERROR"
const ONE_LINE = {
    display: "block",
    overflow: "hidden",
    textOverflow: "ellipsis",
    whiteSpace: "nowrap",
} as const

/** The shared range zone stays a react-admin filter, with the primary after reset. */
const LogTimeZoneFilter: React.FC<{
    source: string
    label: string
    zones: string[]
    primary: string
    alwaysOn?: boolean
}> = ({source, label, zones, primary}) => {
    const {field} = useInput<string | null>({source})
    return (
        <TimeZonePicker
            label={label}
            size="small"
            sx={{
                width: {xs: "calc(100vw - 32px)", sm: 280},
                maxWidth: "100%",
                minWidth: {xs: 0, sm: 260},
                flexShrink: 0,
            }}
            value={field.value || primary}
            zones={zones}
            primary={primary}
            onChange={(zone) => field.onChange(zone)}
        />
    )
}

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

interface AuditDialogProps {
    electionEventId: string
    open: boolean
    setOpen: (val: boolean) => void
}

const AuditDialog: React.FC<AuditDialogProps> = ({electionEventId, open, setOpen}) => {
    const {t} = useTranslation()
    const [auditElectoralLog] = useMutation(AUDIT_ELECTORAL_LOG, {
        context: {
            headers: {
                "x-hasura-role": IPermissions.ELECTORAL_LOG_AUDIT,
            },
        },
    })
    const [addWidget, setWidgetTaskId, updateWidgetFail] = useWidgetStore()
    const audit = async () => {
        const currWidget = addWidget(ETasksExecution.AUDIT_ELECTORAL_LOG, false)
        try {
            const {data, errors} = await auditElectoralLog({
                variables: {electionEventId},
            })
            const taskId = data?.audit_electoral_log?.task_execution.id
            if (errors || !taskId) {
                updateWidgetFail(currWidget.identifier)
                return
            }
            setWidgetTaskId(currWidget.identifier, taskId)
        } catch (error) {
            updateWidgetFail(currWidget.identifier)
        }
    }

    return (
        <Dialog
            variant="info"
            open={open}
            ok={String(t("logsScreen.auditDialog.confirm"))}
            cancel={String(t("common.label.cancel"))}
            title={String(t("logsScreen.auditDialog.title"))}
            handleClose={(result: boolean) => {
                setOpen(false)
                if (result) {
                    audit()
                }
            }}
        >
            <span>{t("logsScreen.auditDialog.description")}</span>
        </Dialog>
    )
}

const headField = (record: ILogRow, field: "event_type" | "log_type"): string | null => {
    const value = logMessage(record)?.statement?.head?.[field]
    return value ? String(value) : null
}

const messageUser = (record: ILogRow, field: "user_id" | "username"): string | null => {
    const value = logMessage(record)?.[field]
    if (value && value !== "null") return value
    const projected = field === "username" ? record.username : null
    return !projected || projected === "null" ? null : projected
}

const description = (t: TFunction, record: ILogRow): string =>
    readinessDescription(t, record.statement_kind) ??
    configurationPackageDescription(t, record.statement_kind, logMessage(record)) ??
    (logMessage(record)?.statement?.head?.description || "-")

/** Use the timestamp's existing height; the description never adds another line to the row. */
const LogRowDescription: React.FC<{
    record: ILogRow
    zones: ILogZones
    format: IZonedFormat
}> = ({record, zones, format}) => {
    const {t} = useTranslation()
    const zoneOf = useLogRowZone(zones)
    const seconds = record.statement_timestamp ?? record.created
    const hasMyTime =
        seconds !== null &&
        seconds !== undefined &&
        format.mine(seconds * 1000, zoneOf(logMessage(record)?.election_id)) !== null
    const text = description(t, record)
    return (
        <Box
            component="span"
            className="log-description"
            title={text}
            sx={{
                display: "-webkit-box",
                WebkitBoxOrient: "vertical",
                WebkitLineClamp: hasMyTime ? 2 : 1,
                overflow: "hidden",
                overflowWrap: "anywhere",
                whiteSpace: "normal",
            }}
        >
            {text}
        </Box>
    )
}

/** A narrow list's entry: two lines that open the same popup as a row. */
const CompactLogEntry: React.FC<{
    record: ILogRow
    zones: ILogZones
    format: IZonedFormat
    onOpen: (record: ILogRow) => void
}> = ({record, zones, format, onOpen}) => {
    const {t} = useTranslation()
    const zoneOf = useLogRowZone(zones)
    const seconds = record.statement_timestamp ?? record.created
    const logType = headField(record, "log_type")
    return (
        <Box
            component="li"
            className="log-compact-entry"
            sx={{borderBottom: "1px solid", borderColor: "divider"}}
        >
            <ButtonBase
                onClick={() => onOpen(record)}
                sx={{display: "block", width: "100%", textAlign: "start", px: 2, py: 1}}
            >
                <Typography
                    component="span"
                    variant="body2"
                    sx={{...ONE_LINE, m: 0, color: "text.secondary"}}
                >
                    {[
                        record.id,
                        seconds === null || seconds === undefined
                            ? null
                            : format.format(
                                  seconds * 1000,
                                  zoneOf(logMessage(record)?.election_id)
                              ),
                        messageUser(record, "username"),
                        record.statement_kind,
                        logType === LOG_TYPE_ERROR ? logType : null,
                    ]
                        .filter((part) => part !== null && part !== undefined && part !== "")
                        .join(" · ")}
                </Typography>
                <Typography component="span" variant="body2" sx={{...ONE_LINE, m: 0}}>
                    {description(t, record)}
                </Typography>
            </ButtonBase>
        </Box>
    )
}

/** The page of a narrow list (a tablet held upright, a phone): two lines per entry. */
const CompactLogEntries: React.FC<{
    zones: ILogZones
    format: IZonedFormat
    onOpen: (record: ILogRow) => void
}> = ({zones, format, onOpen}) => {
    const {t} = useTranslation()
    const {data, isPending} = useListContext<ILogRow>()
    if (isPending) return null
    if (!data?.length) {
        return (
            <Typography variant="body2" sx={{m: 0, p: 2}}>
                {t("logsScreen.empty")}
            </Typography>
        )
    }
    return (
        <Box component="ul" className="log-compact-list" sx={{listStyle: "none", m: 0, p: 0}}>
            {data.map((record) => (
                <CompactLogEntry
                    key={record.id}
                    record={record}
                    zones={zones}
                    format={format}
                    onOpen={onOpen}
                />
            ))}
        </Box>
    )
}

/** The popup of the entry that was clicked, in that entry's log zone. */
const OpenLogEntry: React.FC<{
    entry: ILogRow | null
    log: ILogTarget
    zones: ILogZones
    onClose: () => void
}> = ({entry, log, zones, onClose}) => {
    const zoneOf = useLogRowZone(zones)
    return (
        <LogEntryDialog
            entry={entry}
            log={log}
            zone={zoneOf(logMessage(entry)?.election_id)}
            onClose={onClose}
        />
    )
}

export interface ElectoralLogListProps {
    aside?: ReactElement
    filterToShow?: ElectoralLogFilters
    filterValue?: string
    /** The log listed: an election event's (the default) or the tenant's. */
    scope?: ELogScope
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
    scope = ELogScope.EVENT,
    electionEventId,
    showActions = true,
}) => {
    const record = useRecordContext<Sequent_Backend_Election_Event | Sequent_Backend_Election>()
    const {t} = useTranslation()
    const tenantLog = scope === ELogScope.TENANT
    const eventId = tenantLog ? undefined : electionEventId || record?.id || undefined
    const log: ILogTarget = tenantLog
        ? {kind: ELogScope.TENANT}
        : {kind: ELogScope.EVENT, electionEventId: eventId ?? ""}

    const {canExportLogs, canAuditLogs, canExportTenantLogs, showLogsColumns} = useLogsPermissions()
    // In the event's Logs tab the record is the event; elsewhere, the screen's event.
    const eventZones = useLogZones(record?.id && record.id === eventId ? record : eventId)
    const tenantZones = useTenantLogZones(tenantLog)
    const zones = tenantLog ? tenantZones : eventZones
    const format = useZonedFormat(zones.primary, {seconds: true})
    const [openAudit, setOpenAudit] = useState(false)
    const [openExport, setOpenExport] = useState(false)
    const [entry, setEntry] = useState<ILogRow | null>(null)
    const [listRef, width] = useContainerWidth<HTMLDivElement>()
    const layout = logListLayout(width)

    // Without a chosen zone, the range is read in the primary (also after a filter reset).
    const filterObject: {[key: string]: any} = {
        ...(tenantLog ? {} : {election_event_id: eventId}),
        [ELECTORAL_LOG_DEFAULT_ZONE_FILTER]: zones.primary,
    }

    if (filterToShow) {
        filterObject[filterToShow] = filterValue || undefined
    }

    // Range filters hold wall times in the chosen zone; the data provider
    // turns them into instants.
    const filters: Array<ReactElement> = [
        <LogRangeDateTimeInput
            defaultZone={zones.primary}
            key="created_from"
            source="created_from"
            label={String(t("logsScreen.filter.createdFrom"))}
            alwaysOn
        />,
        <LogRangeDateTimeInput
            defaultZone={zones.primary}
            key="created_to"
            source="created_to"
            label={String(t("logsScreen.filter.createdTo"))}
            alwaysOn
        />,
        <LogTimeZoneFilter
            key={ELECTORAL_LOG_ZONE_FILTER}
            source={ELECTORAL_LOG_ZONE_FILTER}
            label={String(t("logsScreen.filter.timeZone"))}
            zones={zones.choices}
            primary={zones.primary}
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
        <LogRangeDateTimeInput
            defaultZone={zones.primary}
            key="statement_timestamp_from"
            source="statement_timestamp_from"
            label={String(t("logsScreen.filter.statementTimestampFrom"))}
        />,
        <LogRangeDateTimeInput
            defaultZone={zones.primary}
            key="statement_timestamp_to"
            source="statement_timestamp_to"
            label={String(t("logsScreen.filter.statementTimestampTo"))}
        />,
        tenantLog ? (
            <SelectInput
                key={"statement_kind"}
                source={"statement_kind"}
                label={String(t("logsScreen.column.statement_kind"))}
                choices={TENANT_LOG_STATEMENT_KINDS.map((kind) => ({id: kind, name: kind}))}
                translateChoice={false}
            />
        ) : (
            <TextInput
                key={"statement_kind"}
                source={"statement_kind"}
                label={String(t("logsScreen.column.statement_kind"))}
            />
        ),
        <SelectInput
            key={LOG_FILTER_EVENT_TYPE}
            source={LOG_FILTER_EVENT_TYPE}
            label={String(t("logsScreen.column.event_type"))}
            choices={LOG_EVENT_TYPES.map((type) => ({id: type, name: type}))}
            translateChoice={false}
        />,
        <SelectInput
            key={LOG_FILTER_LOG_TYPE}
            source={LOG_FILTER_LOG_TYPE}
            label={String(t("logsScreen.column.log_type"))}
            choices={LOG_TYPES.map((type) => ({id: type, name: type}))}
            translateChoice={false}
        />,
        // Messages belong to an election event's log.
        ...(tenantLog
            ? []
            : [
                  <SelectInput
                      key={LOG_FILTER_CHANNEL}
                      source={LOG_FILTER_CHANNEL}
                      label={String(t("messaging.logs.channel"))}
                      choices={[
                          ...MESSAGE_CHANNELS.map((channel) => ({
                              id: channel,
                              name: String(t(`messaging.channel.${channel}`)),
                          })),
                          {
                              id: LOG_CHANNEL_NONE,
                              name: String(t("messaging.reason.NO_CONTACT_CHANNEL")),
                          },
                      ]}
                  />,
                  <SelectInput
                      key={LOG_FILTER_OUTCOME}
                      source={LOG_FILTER_OUTCOME}
                      label={String(t("messaging.logs.status"))}
                      choices={MESSAGE_ATTEMPT_STATES.map((state) => ({
                          id: state,
                          name: String(t(`messaging.state.${state}`)),
                      }))}
                  />,
              ]),
    ]

    // Each log, at each width, keeps its own choice of columns: Created
    // starts hidden where the list has no room for it.
    const preferenceKey = `electoral_log.${scope}.${layout}.datagrid`
    const omit = layout === ELogListLayout.WIDE ? OMIT_FIELDS : [...OMIT_FIELDS, "created"]
    const openEntry = (row: ILogRow) => setEntry(row)

    const rows =
        layout === ELogListLayout.COMPACT ? (
            <CompactLogEntries zones={zones} format={format} onOpen={openEntry} />
        ) : (
            <DatagridConfigurable
                key={preferenceKey}
                preferenceKey={preferenceKey}
                header={ThreeStateDatagridHeader}
                omit={omit}
                bulkActionButtons={false}
                rowClick={(_id, _resource, row) => {
                    openEntry(row as ILogRow)
                    return false
                }}
                sx={{
                    "& .RaDatagrid-headerCell": {whiteSpace: "nowrap"},
                    "& .RaDatagrid-rowCell": {verticalAlign: "top"},
                    "& .column-description": {width: "100%", maxWidth: 0},
                }}
            >
                <NumberField source="id" label={String(t("logsScreen.column.id"))} />
                <FunctionField
                    source="user_id"
                    label={String(t("logsScreen.column.user_id"))}
                    render={(row: ILogRow) => <span>{messageUser(row, "user_id") ?? "-"}</span>}
                />
                <FunctionField
                    source="username"
                    label={String(t("logsScreen.column.username"))}
                    render={(row: ILogRow) =>
                        ballotLogDetails(logMessage(row)) ? (
                            <Chip size="small" label={messageUser(row, "username")} />
                        ) : (
                            <span>{messageUser(row, "username") ?? "-"}</span>
                        )
                    }
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
                        <LogRowTime source="statement_timestamp" zones={zones} format={format} />
                    )}
                />
                <TextField
                    source="statement_kind"
                    label={String(t("logsScreen.column.statement_kind"))}
                />
                <FunctionField
                    source="event_type"
                    label={String(t("logsScreen.column.event_type"))}
                    render={(row: ILogRow) => headField(row, "event_type") ?? <span>-</span>}
                />
                <FunctionField
                    source="log_type"
                    label={String(t("logsScreen.column.log_type"))}
                    render={(row: ILogRow) => headField(row, "log_type") ?? <span>-</span>}
                />
                {tenantLog ? null : (
                    <FunctionField
                        source="channel"
                        sortable={false}
                        label={String(t("messaging.logs.channel"))}
                        render={(row: {message?: string}) => {
                            const channel = electoralLogChannel(row.message)
                            return channel ? (
                                <ChannelLabel
                                    channel={channel}
                                    label={t(`messaging.channel.${channel}`)}
                                />
                            ) : (
                                <span>-</span>
                            )
                        }}
                    />
                )}
                {tenantLog ? null : (
                    <FunctionField
                        source="status"
                        sortable={false}
                        label={String(t("messaging.logs.status"))}
                        render={() => <LedgerStatusField />}
                    />
                )}
                <FunctionField
                    source="description"
                    label={String(t("logsScreen.column.description"))}
                    render={(row: ILogRow) => (
                        <LogRowDescription record={row} zones={zones} format={format} />
                    )}
                />
                <FunctionField
                    source="message"
                    sortable={false}
                    label={String(t("logsScreen.column.message"))}
                    render={() => (
                        // Show More opens the text in place, not the entry.
                        <Box component="span" onClick={(event) => event.stopPropagation()}>
                            <MessageField source="message" />
                        </Box>
                    )}
                />
            </DatagridConfigurable>
        )

    return (
        <Box
            ref={listRef}
            className={`electoral-log-list electoral-log-list-${layout}`}
            sx={{
                "minWidth": 0,
                "& .RaList-main": {minWidth: 0},
                "& .RaFilterForm-filterFormInput": {maxWidth: "100%", minWidth: 0},
                "& .RaFilterForm-filterFormInput .MuiFormControl-root": {
                    minWidth: 0,
                    flexShrink: 1,
                },
                "& .RaFilterForm-filterFormInput input": {minWidth: 0},
                // The filters and the buttons share one line while both fit;
                // the buttons move below when the list is narrower than both.
                "& .RaList-actions": {flexWrap: "wrap", alignItems: "flex-end", rowGap: 1},
                "& .RaList-actions > form": {flex: "1 1 0", minWidth: "min(100%, 34rem)"},
                "& .RaList-actions > .list-actions": {marginLeft: "auto"},
                "& .RaList-actions .MuiToolbar-root": {flexWrap: "wrap", minHeight: 0},
            }}
        >
            <List
                resource="electoral_log"
                actions={
                    showActions && (
                        <ListActions
                            withColumns={
                                layout !== ELogListLayout.COMPACT && (tenantLog || showLogsColumns)
                            }
                            preferenceKey={preferenceKey}
                            withImport={false}
                            doExport={() => setOpenExport(true)}
                            withExport={tenantLog ? canExportTenantLogs : canExportLogs}
                            withFilter={true}
                            extraActions={
                                canAuditLogs && eventId
                                    ? [
                                          <Button
                                              key="audit"
                                              className="audit-electoral-log"
                                              onClick={() => setOpenAudit(true)}
                                              label={String(t("logsScreen.actions.audit"))}
                                          >
                                              <FactCheckIcon />
                                          </Button>,
                                      ]
                                    : []
                            }
                        />
                    )
                }
                filters={filters}
                filter={filterObject}
                queryOptions={{meta: {[LOG_SCOPE_META]: scope}}}
                storeKey={false}
                sort={{
                    field: "id",
                    order: "DESC",
                }}
                aside={aside}
            >
                <ResetFilters keepLinkedFilter />
                <LedgerStates>{rows}</LedgerStates>
                <OpenLogEntry
                    entry={entry}
                    log={log}
                    zones={zones}
                    onClose={() => setEntry(null)}
                />
            </List>
            <ExportLogsDialog
                log={log}
                open={openExport}
                onClose={() => setOpenExport(false)}
                zones={zones.choices}
                byElection={zones.byElection}
            />
            {eventId ? (
                <AuditDialog electionEventId={eventId} open={openAudit} setOpen={setOpenAudit} />
            ) : null}
        </Box>
    )
}
