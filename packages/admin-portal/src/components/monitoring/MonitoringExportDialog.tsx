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
} from "./types"
import {EExportRange, exportBound, exportRange} from "./lib/exportRange"

/** What is exported: a dashboard, or one of its widgets, at the revision shown. */
export interface MonitoringExportTarget {
    electionEventId: string
    electionId?: string | null
    dashboardId: string
    widgetId?: string | null
    scope: MonitoringScope
    selectorValues: Record<string, string>
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
}: MonitoringExportDialogProps) {
    const {t} = useTranslation()
    const [format, setFormat] = useState(initialFormat)
    const [from, setFrom] = useState("")
    const [to, setTo] = useState("")
    const [exportData] = useMutation<MonitoringExportMutation, MonitoringExportVariables>(
        MONITORING_EXPORT
    )
    const [addWidget, setWidgetTaskId, updateWidgetFail] = useWidgetStore()

    useEffect(() => {
        if (open) setFormat(initialFormat)
    }, [open, initialFormat])

    const range = exportRange(from, to)

    const start = async () => {
        const widget = addWidget(ETasksExecution.EXPORT_MONITORING_DATA, true)
        try {
            const {data, errors} = await exportData({
                variables: {
                    ...target,
                    format,
                    from: exportBound(from),
                    to: exportBound(to),
                },
            })
            const taskId = data?.monitoringExport.task_execution.id
            if (errors?.length || !taskId) {
                updateWidgetFail(widget.identifier)
                return
            }
            setWidgetTaskId(widget.identifier, taskId)
        } catch {
            updateWidgetFail(widget.identifier)
        }
    }

    return (
        <Dialog
            variant="info"
            open={open}
            title={t("monitoring.export.title")}
            ok={t("monitoring.export.export")}
            cancel={t("monitoring.export.cancel")}
            okEnabled={() => range === EExportRange.VALID}
            handleClose={(confirmed: boolean) => {
                onClose()
                if (confirmed) void start()
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
                            range !== EExportRange.VALID
                                ? t("monitoring.export.invalidRange")
                                : undefined
                        }
                        fullWidth
                    />
                </Stack>
                <Typography variant="body2" color="text.secondary">
                    {t("monitoring.export.timeZoneHelp", {timeZone})}
                </Typography>
            </Stack>
        </Dialog>
    )
}
