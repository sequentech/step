// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState} from "react"
import {
    Box,
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
import {Dialog} from "@sequentech/ui-essentials"
import {timeZoneOption, zoneLabel, zonedToInstant} from "@sequentech/ui-core"
import {EXPORT_ELECTION_EVENT_LOGS} from "@/queries/ExportElectionEventLogs"
import {IPermissions} from "@/types/keycloak"
import {useWidgetStore} from "@/providers/WidgetsContextProvider"
import {ETasksExecution} from "@/types/tasksExecution"
import {formatWallTime, useTimeZoneService} from "@/components/timezones/timeZoneService"

export enum ExportFormat {
    CSV = "CSV",
    // The PDF arrives zipped.
    PDF = "PDF",
}

export interface ExportLogsDialogProps {
    electionEventId: string
    open: boolean
    onClose: () => void
    /** The zones offered; the first is the event's primary. */
    zones: string[]
    /**
     * Rows are in their election's zone (the ELECTION policy): the dialog
     * offers "each row's election timezone", and starts there.
     */
    byElection?: boolean
}

/** The zone choice that keeps each row in its own log zone (no `time_zone` sent). */
const ROW_ZONES = "row-zones"

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
 * The Logs export: a range of Created (both ends included, to the minute),
 * the zone the range and the file's times are in, and CSV or PDF. The scope
 * and the SQL format belong to the audit export (EMS-AUDIT).
 */
export const ExportLogsDialog: React.FC<ExportLogsDialogProps> = ({
    electionEventId,
    open,
    onClose,
    zones,
    byElection = false,
}) => {
    const {t, i18n} = useTranslation()
    const lang = i18n.language
    const service = useTimeZoneService()
    const [from, setFrom] = useState("")
    const [to, setTo] = useState("")
    // Until someone chooses, the default follows the event's zones.
    const [chosen, setChosen] = useState<string | null>(null)
    const [format, setFormat] = useState(ExportFormat.CSV)
    const choice = chosen ?? (byElection ? ROW_ZONES : (zones[0] ?? ""))
    // Each row in its own zone: the range is read in the primary.
    const rowZones = choice === ROW_ZONES
    const zone = rowZones ? (zones[0] ?? "") : choice
    const [exportLogs] = useMutation(EXPORT_ELECTION_EVENT_LOGS, {
        context: {headers: {"x-hasura-role": IPermissions.LOGS_EXPORT}},
    })
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
        if (!validRange) return
        const widget = addWidget(ETasksExecution.EXPORT_ACTIVITY_LOGS_REPORT, true)
        try {
            const {data, errors} = await exportLogs({
                variables: {
                    electionEventId,
                    format,
                    createdFrom: start?.instant ?? null,
                    createdTo: end?.instant ?? null,
                    timeZone: rowZones ? null : zone || null,
                },
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
    const note = rowZones
        ? format === ExportFormat.CSV
            ? "logsScreen.exportdialog.zoneNoteRows"
            : "logsScreen.exportdialog.zoneNoteRowsPdf"
        : format === ExportFormat.CSV
          ? "logsScreen.exportdialog.zoneNote"
          : "logsScreen.exportdialog.zoneNotePdf"

    return (
        <Dialog
            variant="info"
            open={open}
            ok={String(t("common.label.export"))}
            cancel={String(t("common.label.cancel"))}
            title={String(t("logsScreen.exportdialog.title"))}
            okEnabled={() => validRange}
            handleClose={(result: boolean) => {
                onClose()
                if (result) download()
            }}
        >
            <Box sx={{display: "flex", flexDirection: "column", gap: 2, pt: 1}}>
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
                <TextField
                    select
                    label={t("logsScreen.exportdialog.timeZone")}
                    value={choice}
                    onChange={(event) => setChosen(event.target.value)}
                >
                    {byElection ? (
                        <MenuItem value={ROW_ZONES}>
                            {t("logsScreen.exportdialog.rowZones")}
                        </MenuItem>
                    ) : null}
                    {zones.map((choice) => (
                        <MenuItem key={choice} value={choice}>
                            {timeZoneOption(choice, {t, lang}).label}
                        </MenuItem>
                    ))}
                </TextField>
                <FormControl>
                    <FormLabel id="export-logs-format">
                        {t("logsScreen.exportdialog.format")}
                    </FormLabel>
                    <RadioGroup
                        row
                        aria-labelledby="export-logs-format"
                        value={format}
                        onChange={(event) => setFormat(event.target.value as ExportFormat)}
                    >
                        <FormControlLabel
                            value={ExportFormat.CSV}
                            control={<Radio />}
                            label={t("logsScreen.exportdialog.csv")}
                        />
                        <FormControlLabel
                            value={ExportFormat.PDF}
                            control={<Radio />}
                            label={t("logsScreen.exportdialog.pdf")}
                        />
                    </RadioGroup>
                </FormControl>
                {abbr ? (
                    <Typography variant="body2" className="export-logs-note">
                        {t(note, {abbr})}
                    </Typography>
                ) : null}
            </Box>
        </Dialog>
    )
}
