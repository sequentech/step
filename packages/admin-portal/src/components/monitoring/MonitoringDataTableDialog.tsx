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
    Typography,
} from "@mui/material"
import {useTranslation} from "react-i18next"
import type {MonitoringQueryTable, MonitoringTable} from "./types"
import {MonitoringDataTable} from "./MonitoringDataTable"
import {columnLabeler, dataSections} from "./lib/dataTables"
import {noticeText} from "./lib/notices"

export interface MonitoringDataTableDialogProps {
    open: boolean
    onClose: () => void
    title: string
    scope: string
    /** The first query's rows: all an older backend sends. */
    table?: MonitoringTable | null
    /** Every query's rows, in widget order. */
    tables?: MonitoringQueryTable[] | null
    /** The widget's queries, whose measures and labels name the columns. */
    queries?: Record<string, unknown>
    /** Notice codes, as the server sends them; shown in the viewer's language. */
    notices?: string[]
}

/**
 * View data: the rows behind the chart the viewer is looking at, a section per
 * query, with columns named in the viewer's language where the platform knows
 * the words.
 */
export function MonitoringDataTableDialog({
    open,
    onClose,
    title,
    scope,
    table,
    tables,
    queries,
    notices = [],
}: MonitoringDataTableDialogProps) {
    const {t, i18n} = useTranslation()
    const heading = t("monitoring.dataTable.title", {widget: title})
    const known = (key: string) => (i18n.exists(key) ? t(key) : undefined)
    const sections = dataSections({table, tables}, queries)
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
                            {noticeText(t, notice)}
                        </Alert>
                    ))}
                    {sections.map((section, index) => {
                        const id = `monitoring-data-query-${index}`
                        return (
                            <Stack
                                key={section.query ?? index}
                                component="section"
                                spacing={1}
                                aria-labelledby={section.query ? id : undefined}
                            >
                                {section.query ? (
                                    <Typography id={id} variant="subtitle1" component="h3">
                                        {section.query}
                                    </Typography>
                                ) : null}
                                <MonitoringDataTable
                                    table={section.table}
                                    caption={
                                        section.query ? `${heading} · ${section.query}` : heading
                                    }
                                    columnLabel={columnLabeler(section.definition, known)}
                                />
                            </Stack>
                        )
                    })}
                </Stack>
            </DialogContent>
            <DialogActions>
                <Button onClick={onClose}>{t("monitoring.dataTable.close")}</Button>
            </DialogActions>
        </Dialog>
    )
}
