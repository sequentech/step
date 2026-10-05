// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useState} from "react"
import {
    FormControl,
    FormControlLabel,
    FormLabel,
    Radio,
    RadioGroup,
    Stack,
    TextField,
    Typography,
} from "@mui/material"
import {useMutation} from "@apollo/client"
import {useTranslation} from "react-i18next"
import {Dialog} from "@sequentech/ui-essentials"
import {useWidgetStore} from "@/providers/WidgetsContextProvider"
import {ETasksExecution} from "@/types/tasksExecution"
import {MONITORING_EXPORT} from "@/queries/MonitoringExport"
import {
    EMonitoringExportFormat,
    type MonitoringExportMutation,
    type MonitoringExportVariables,
    type MonitoringScope,
    type MonitoringWidget,
} from "./types"
import {EExportRange, exportBound, exportRange} from "./lib/exportRange"
import {EMonitoringErrorCode, monitoringErrorCode} from "./lib/errors"
import {exportFailure} from "./lib/exportErrors"
import {TimeZonePicker} from "@/components/timezones/TimeZonePicker"
import {formatWallTime, useTimeZoneService} from "@/components/timezones/timeZoneService"

/** What is exported: a dashboard, or one of its widgets, at the revision shown. */
export interface MonitoringExportTarget {
    electionEventId: string
    electionId?: string | null
    dashboardId: string
    widgetId?: string | null
    scope: MonitoringScope
    selectorValues: Record<string, string>
    /** A dashboard export: each widget's values as the dashboard draws it, by widget id. */
    widgetSelectorValues?: Record<string, Record<string, string>> | null
    snapshotRevision: number
}

export interface MonitoringExportDialogProps {
    open: boolean
    onClose: () => void
    /** The dashboard, or the widget, being exported. */
    title: string
    scope: string
    timeZone: string
    target: MonitoringExportTarget
    initialFormat?: EMonitoringExportFormat
    /** The update shown is no longer kept: the dashboard asks for the current one. */
    onSnapshotPruned?: () => void
    /** The widgets exported, which name a refused pick. */
    widgets?: MonitoringWidget[]
}

/**
 * Export monitoring data as CSV or SQL over a range. As with the electoral log
 * export, the file is made by a task that the task widget follows and downloads.
 */
export function MonitoringExportDialog({
    open,
    onClose,
    title,
    scope,
    timeZone,
    target,
    initialFormat = EMonitoringExportFormat.CSV,
    onSnapshotPruned,
    widgets = [],
}: MonitoringExportDialogProps) {
    const {t} = useTranslation()
    const service = useTimeZoneService()
    const [chosenZone, setChosenZone] = useState(timeZone)
    const [format, setFormat] = useState(initialFormat)
    const [from, setFrom] = useState("")
    const [to, setTo] = useState("")
    const [sending, setSending] = useState(false)
    const [failure, setFailure] = useState<string | undefined>()
    const [exportData] = useMutation<MonitoringExportMutation, MonitoringExportVariables>(
        MONITORING_EXPORT
    )
    const [addWidget, setWidgetTaskId] = useWidgetStore()

    useEffect(() => {
        if (!open) return
        setFormat(initialFormat)
        setFailure(undefined)
        setChosenZone(timeZone)
    }, [open, initialFormat, timeZone])

    const range = exportRange(from, to, chosenZone)
    const boundNote = (value: string) => {
        if (!value) return undefined
        try {
            if (service.zonedToInstant(value.slice(0, 16), chosenZone).kind === "gap") {
                return t("lifecycle.import.error.dstGap", {
                    dateTime: formatWallTime(value, service.text),
                    city: service.zoneCity(chosenZone, service.text),
                })
            }
            return service.zonedTimeNote(value.slice(0, 16), chosenZone, service.text)
        } catch {
            return t("lifecycle.import.error.invalidDateTime")
        }
    }

    /** The dialog stays open until the export is started, so a refusal can be told. */
    const start = async () => {
        if (range !== EExportRange.VALID) return
        setSending(true)
        setFailure(undefined)
        try {
            const {data} = await exportData({
                variables: {
                    ...target,
                    format,
                    from: exportBound(from, chosenZone),
                    to: exportBound(to, chosenZone),
                },
            })
            const taskId = data?.monitoringExport.task_execution?.id
            if (!taskId) {
                setFailure(t("monitoring.errors.unknown"))
                return
            }
            const widget = addWidget(ETasksExecution.EXPORT_MONITORING_DATA, true)
            setWidgetTaskId(widget.identifier, taskId)
            onClose()
        } catch (error) {
            if (monitoringErrorCode(error) === EMonitoringErrorCode.SNAPSHOT_PRUNED) {
                onSnapshotPruned?.()
            }
            setFailure(exportFailure(error, t, widgets))
        } finally {
            setSending(false)
        }
    }

    return (
        <Dialog
            variant="info"
            open={open}
            title={t("monitoring.export.title")}
            ok={t("monitoring.export.export")}
            cancel={t("monitoring.export.cancel")}
            okEnabled={() => range === EExportRange.VALID && !sending}
            errorMessage={failure}
            handleClose={(confirmed: boolean) => {
                if (confirmed) void start()
                else onClose()
            }}
        >
            <Stack spacing={2} sx={{pt: 1}}>
                <Typography variant="body2">
                    {title} · {scope}
                </Typography>
                <FormControl>
                    <FormLabel id="monitoring-export-format">
                        {t("monitoring.export.format")}
                    </FormLabel>
                    <RadioGroup
                        row
                        aria-labelledby="monitoring-export-format"
                        value={format}
                        onChange={(event) =>
                            setFormat(event.target.value as EMonitoringExportFormat)
                        }
                    >
                        <FormControlLabel
                            value={EMonitoringExportFormat.CSV}
                            control={<Radio />}
                            label={t("monitoring.export.csv")}
                        />
                        <FormControlLabel
                            value={EMonitoringExportFormat.SQL}
                            control={<Radio />}
                            label={t("monitoring.export.sql")}
                        />
                    </RadioGroup>
                </FormControl>
                <Stack direction={{xs: "column", sm: "row"}} spacing={2}>
                    <TextField
                        id="monitoring-export-from"
                        type="datetime-local"
                        label={t("monitoring.export.from")}
                        value={from}
                        onChange={(event) => setFrom(event.target.value)}
                        slotProps={{inputLabel: {shrink: true}}}
                        error={!!from && exportRange(from, "", chosenZone) !== EExportRange.VALID}
                        helperText={boundNote(from)}
                        fullWidth
                    />
                    <TextField
                        id="monitoring-export-to"
                        type="datetime-local"
                        label={t("monitoring.export.to")}
                        value={to}
                        onChange={(event) => setTo(event.target.value)}
                        slotProps={{inputLabel: {shrink: true}}}
                        error={range !== EExportRange.VALID}
                        helperText={
                            boundNote(to) ??
                            (range === EExportRange.END_NOT_AFTER_START
                                ? t("monitoring.export.invalidRange")
                                : undefined)
                        }
                        fullWidth
                    />
                </Stack>
                <TimeZonePicker
                    label={t("lifecycle.input.timezone")}
                    value={chosenZone}
                    onChange={(zone) => {
                        if (zone) setChosenZone(zone)
                    }}
                />
                <Typography variant="body2" color="text.secondary">
                    {t("monitoring.export.timeZoneHelp", {
                        timeZone: service.zoneLabel(chosenZone, service.text),
                    })}
                </Typography>
            </Stack>
        </Dialog>
    )
}
