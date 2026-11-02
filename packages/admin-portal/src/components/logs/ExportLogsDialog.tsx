// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState} from "react"
import {
    Box,
    Checkbox,
    FormControl,
    FormControlLabel,
    FormLabel,
    MenuItem,
    Radio,
    RadioGroup,
    TextField,
    Typography,
} from "@mui/material"
import {useTranslation} from "react-i18next"
import {useMutation} from "@apollo/client"
import {useGetList} from "react-admin"
import {Dialog} from "@sequentech/ui-essentials"
import {zoneLabel, zonedToInstant} from "@sequentech/ui-core"
import type {Sequent_Backend_Area, Sequent_Backend_Election} from "@/gql/graphql"
import {EXPORT_ELECTION_EVENT_LOGS} from "@/queries/ExportElectionEventLogs"
import {EXPORT_TENANT_LOGS} from "@/queries/ExportTenantLogs"
import {IPermissions} from "@/types/keycloak"
import {useWidgetStore} from "@/providers/WidgetsContextProvider"
import {ETasksExecution} from "@/types/tasksExecution"
import {useAliasRenderer} from "@/hooks/useAliasRenderer"
import {formatWallTime, useTimeZoneService} from "@/components/timezones/timeZoneService"
import {TimeZonePicker} from "@/components/timezones/TimeZonePicker"
import {
    EExportLogsFormat,
    EExportLogsScope,
    eventExportVariables,
    exportLogsScopes,
    exportScopeReady,
    isAccountScope,
    tenantExportScope,
    tenantExportVariables,
    type IExportLogsChoice,
} from "./exportLogsRequest"
import {ELogScope, type ILogTarget} from "./logScope"

export interface ExportLogsDialogProps {
    /** The log exported: an election event's or the tenant's. */
    log: ILogTarget
    open: boolean
    onClose: () => void
    /** The zones offered; the first is the log's primary. */
    zones: string[]
    /**
     * Rows are in their election's zone (the ELECTION policy): the dialog
     * offers "each row's election timezone", and starts there.
     */
    byElection?: boolean
}

/** The zone choice that keeps each row in its own log zone (no `time_zone` sent). */
const ROW_ZONES = "row-zones"
const PICKER_PAGE = {page: 1, perPage: 1000}
const FORMATS = [EExportLogsFormat.CSV, EExportLogsFormat.SQL, EExportLogsFormat.PDF]
const FORMAT_LABELS: Record<EExportLogsFormat, string> = {
    [EExportLogsFormat.CSV]: "logsScreen.exportdialog.csv",
    [EExportLogsFormat.SQL]: "logsScreen.exportdialog.sql",
    [EExportLogsFormat.PDF]: "logsScreen.exportdialog.pdf",
}

/** Incomplete/invalid bounds must not become an omitted export range. */
const resolveBound = (local: string, zone: string): ReturnType<typeof zonedToInstant> | null => {
    if (!local || !zone) return null
    try {
        return zonedToInstant(local.slice(0, 16), zone)
    } catch {
        return null
    }
}

/**
 * The Logs export: what it covers (the whole log, or one election, area or
 * account of it), a range of Created (both ends included, to the minute), the
 * zone the range and the file's times are in, and CSV, SQL or PDF.
 */
export const ExportLogsDialog: React.FC<ExportLogsDialogProps> = ({
    log,
    open,
    onClose,
    zones,
    byElection = false,
}) => {
    const {t, i18n} = useTranslation()
    const lang = i18n.language
    const service = useTimeZoneService()
    const aliasRenderer = useAliasRenderer()
    const tenantLog = log.kind === ELogScope.TENANT
    const electionEventId = log.kind === ELogScope.EVENT ? log.electionEventId : ""
    const scopes = exportLogsScopes(log.kind)
    const [from, setFrom] = useState("")
    const [to, setTo] = useState("")
    // Until someone chooses, the default follows the log's zones.
    const [chosen, setChosen] = useState<string | null>(null)
    const [format, setFormat] = useState(EExportLogsFormat.CSV)
    const [scope, setScope] = useState(scopes[0])
    const [username, setUsername] = useState("")
    const [withElectionEvents, setWithElectionEvents] = useState(false)
    const [electionId, setElectionId] = useState<string | null>(null)
    const [areaId, setAreaId] = useState<string | null>(null)
    const choice = chosen ?? (byElection ? ROW_ZONES : (zones[0] ?? ""))
    // Each row in its own zone: the range is read in the primary.
    const rowZones = choice === ROW_ZONES
    const zone = rowZones ? (zones[0] ?? "") : choice
    const [exportEventLogs] = useMutation(EXPORT_ELECTION_EVENT_LOGS, {
        context: {headers: {"x-hasura-role": IPermissions.LOGS_EXPORT}},
    })
    const [exportTenantLogs] = useMutation(EXPORT_TENANT_LOGS, {
        context: {headers: {"x-hasura-role": IPermissions.TENANT_LOGS_EXPORT}},
    })
    const {data: elections} = useGetList<Sequent_Backend_Election>(
        "sequent_backend_election",
        {
            filter: {election_event_id: electionEventId},
            pagination: PICKER_PAGE,
            sort: {field: "created_at", order: "ASC"},
        },
        {enabled: open && !!electionEventId && scope === EExportLogsScope.ELECTION}
    )
    const {data: areas} = useGetList<Sequent_Backend_Area>(
        "sequent_backend_area",
        {
            filter: {election_event_id: electionEventId},
            pagination: PICKER_PAGE,
            sort: {field: "name", order: "ASC"},
        },
        {enabled: open && !!electionEventId && scope === EExportLogsScope.AREA}
    )
    const [addWidget, setWidgetTaskId, updateWidgetFail] = useWidgetStore()
    const start = resolveBound(from, zone)
    const end = resolveBound(to, zone)
    const invalidFrom = !!from && (!start || start.kind === "gap")
    const invalidTo = !!to && (!end || end.kind === "gap")
    const validRange =
        !!zone &&
        !invalidFrom &&
        !invalidTo &&
        (!start || !end || Date.parse(start.instant) <= Date.parse(end.instant))
    const request: IExportLogsChoice = {
        scope: tenantLog ? tenantExportScope(scope, withElectionEvents) : scope,
        username,
        electionId,
        areaId,
    }
    const ready = validRange && exportScopeReady(request)
    const boundNote = (local: string, resolved: typeof start) => {
        if (!local) return undefined
        if (!resolved) return t("lifecycle.import.error.invalidDateTime")
        if (resolved.kind === "gap")
            return t("lifecycle.import.error.dstGap", {
                dateTime: formatWallTime(local, service.text),
                city: service.zoneCity(zone, service.text),
            })
        return service.zonedTimeNote(local, zone, service.text)
    }

    const download = async () => {
        if (!ready) return
        const range = {
            createdFrom: start?.instant ?? null,
            createdTo: end?.instant ?? null,
            timeZone: rowZones ? null : zone || null,
        }
        const widget = addWidget(
            tenantLog
                ? ETasksExecution.EXPORT_TENANT_LOGS
                : ETasksExecution.EXPORT_ACTIVITY_LOGS_REPORT,
            true
        )
        try {
            if (tenantLog) {
                const {data, errors} = await exportTenantLogs({
                    variables: tenantExportVariables(request, format, range),
                })
                if (errors) {
                    updateWidgetFail(widget.identifier)
                    return
                }
                setWidgetTaskId(widget.identifier, data?.export_tenant_logs?.task_execution.id)
                return
            }
            const {data, errors} = await exportEventLogs({
                variables: eventExportVariables(electionEventId, request, format, range),
            })
            if (errors) {
                updateWidgetFail(widget.identifier)
                return
            }
            setWidgetTaskId(widget.identifier, data?.export_election_event_logs?.task_execution.id)
        } catch {
            updateWidgetFail(widget.identifier)
        }
    }

    // The label at the range's start (its offset there), else now.
    const abbr = zone
        ? zoneLabel(zone, {t, lang}, start ? new Date(start.instant) : new Date())
        : ""
    const printed = format === EExportLogsFormat.PDF
    const note = rowZones
        ? printed
            ? "logsScreen.exportdialog.zoneNoteRowsPdf"
            : "logsScreen.exportdialog.zoneNoteRows"
        : printed
          ? "logsScreen.exportdialog.zoneNotePdf"
          : "logsScreen.exportdialog.zoneNote"
    const scopeLabel = String(t("logsScreen.exportdialog.scope.label"))
    const usernameLabel = String(t("logsScreen.exportdialog.username"))
    const electionLabel = String(t("logsScreen.exportdialog.election"))
    const areaLabel = String(t("logsScreen.exportdialog.area"))

    return (
        <Dialog
            variant="info"
            open={open}
            fullWidth
            maxWidth="sm"
            ok={String(t("common.label.export"))}
            cancel={String(t("common.label.cancel"))}
            title={String(t("logsScreen.exportdialog.title"))}
            okEnabled={() => ready}
            handleClose={(result: boolean) => {
                onClose()
                if (result) download()
            }}
        >
            <Box sx={{display: "flex", flexDirection: "column", gap: 2, pt: 1}}>
                <TextField
                    select
                    label={scopeLabel}
                    value={scope}
                    onChange={(event) => setScope(event.target.value as EExportLogsScope)}
                    className="export-logs-scope"
                >
                    {scopes.map((option) => (
                        <MenuItem key={option} value={option}>
                            {t(`logsScreen.exportdialog.scope.${log.kind}.${option}`)}
                        </MenuItem>
                    ))}
                </TextField>
                {scope === EExportLogsScope.ELECTION ? (
                    <TextField
                        select
                        required
                        label={electionLabel}
                        value={electionId ?? ""}
                        onChange={(event) => setElectionId(event.target.value || null)}
                    >
                        {(elections ?? []).map((election) => (
                            <MenuItem key={election.id} value={election.id}>
                                {aliasRenderer(election)}
                            </MenuItem>
                        ))}
                    </TextField>
                ) : null}
                {scope === EExportLogsScope.AREA ? (
                    <TextField
                        select
                        required
                        label={areaLabel}
                        value={areaId ?? ""}
                        onChange={(event) => setAreaId(event.target.value || null)}
                    >
                        {(areas ?? []).map((area) => (
                            <MenuItem key={area.id} value={area.id}>
                                {area.name}
                            </MenuItem>
                        ))}
                    </TextField>
                ) : null}
                {isAccountScope(scope) ? (
                    <TextField
                        required
                        label={usernameLabel}
                        value={username}
                        onChange={(event) => setUsername(event.target.value)}
                        autoComplete="off"
                    />
                ) : null}
                {tenantLog && isAccountScope(scope) ? (
                    <FormControlLabel
                        control={
                            <Checkbox
                                checked={withElectionEvents}
                                onChange={(event) => setWithElectionEvents(event.target.checked)}
                            />
                        }
                        label={t("logsScreen.exportdialog.withElectionEvents")}
                    />
                ) : null}
                <Box sx={{display: "flex", gap: 2, flexWrap: "wrap"}}>
                    <TextField
                        type="datetime-local"
                        label={t("logsScreen.exportdialog.from")}
                        value={from}
                        error={invalidFrom}
                        helperText={boundNote(from, start)}
                        onChange={(event) => setFrom(event.target.value)}
                        InputLabelProps={{shrink: true}}
                        inputProps={{"aria-label": String(t("logsScreen.exportdialog.from"))}}
                        sx={{flex: 1, minWidth: 200}}
                    />
                    <TextField
                        type="datetime-local"
                        label={t("logsScreen.exportdialog.to")}
                        value={to}
                        error={invalidTo}
                        helperText={boundNote(to, end)}
                        onChange={(event) => setTo(event.target.value)}
                        InputLabelProps={{shrink: true}}
                        inputProps={{"aria-label": String(t("logsScreen.exportdialog.to"))}}
                        sx={{flex: 1, minWidth: 200}}
                    />
                </Box>
                <TimeZonePicker
                    label={t("logsScreen.exportdialog.timeZone")}
                    value={choice}
                    zones={zones}
                    primary={zones[0]}
                    onChange={(zone) => {
                        if (zone) setChosen(zone)
                    }}
                    additionalOptions={
                        byElection
                            ? [
                                  {
                                      zone: ROW_ZONES,
                                      label: t("logsScreen.exportdialog.rowZones"),
                                      detail: "",
                                      offsetMinutes: 0,
                                  },
                              ]
                            : []
                    }
                />
                <FormControl>
                    <FormLabel id="export-logs-format">
                        {t("logsScreen.exportdialog.format")}
                    </FormLabel>
                    <RadioGroup
                        row
                        aria-labelledby="export-logs-format"
                        value={format}
                        onChange={(event) => setFormat(event.target.value as EExportLogsFormat)}
                    >
                        {FORMATS.map((option) => (
                            <FormControlLabel
                                key={option}
                                value={option}
                                control={<Radio />}
                                label={t(FORMAT_LABELS[option])}
                            />
                        ))}
                    </RadioGroup>
                </FormControl>
                {abbr ? (
                    <Typography variant="body2" className="export-logs-note">
                        <span>{t(note, {abbr})}</span>{" "}
                        {request.scope === EExportLogsScope.ACCOUNT_WITH_EVENTS ? (
                            <>
                                <span>{t("logsScreen.exportdialog.rowsNameLog")}</span>{" "}
                            </>
                        ) : null}
                        <span>{t("logsScreen.exportdialog.recorded")}</span>
                    </Typography>
                ) : null}
            </Box>
        </Dialog>
    )
}
