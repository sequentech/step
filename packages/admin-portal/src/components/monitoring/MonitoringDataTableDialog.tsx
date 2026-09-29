// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {
    Alert,
    Button,
    Dialog,
    DialogActions,
    DialogContent,
    DialogTitle,
    Stack,
} from "@mui/material"
import {useTranslation} from "react-i18next"
import type {MonitoringTable} from "./types"
import {MonitoringDataTable} from "./MonitoringDataTable"

export interface MonitoringDataTableDialogProps {
    open: boolean
    onClose: () => void
    title: string
    scope: string
    table: MonitoringTable
    notices?: string[]
}

/** View data: the rows behind the chart the viewer is looking at. */
export function MonitoringDataTableDialog({
    open,
    onClose,
    title,
    scope,
    table,
    notices = [],
}: MonitoringDataTableDialogProps) {
    const {t} = useTranslation()
    const heading = t("monitoring.dataTable.title", {widget: title})
    return (
        <Dialog
            open={open}
            onClose={onClose}
            maxWidth="md"
            fullWidth
            aria-labelledby="monitoring-data-title"
        >
            <DialogTitle id="monitoring-data-title">
                {heading}
                <Stack component="span" sx={{typography: "body2", color: "text.secondary"}}>
                    {scope}
                </Stack>
            </DialogTitle>
            <DialogContent>
                <Stack spacing={2}>
                    {notices.map((notice) => (
                        <Alert key={notice} severity="info">
                            {notice}
                        </Alert>
                    ))}
                    <MonitoringDataTable table={table} caption={heading} />
                </Stack>
            </DialogContent>
            <DialogActions>
                <Button onClick={onClose}>{t("monitoring.dataTable.close")}</Button>
            </DialogActions>
        </Dialog>
    )
}
