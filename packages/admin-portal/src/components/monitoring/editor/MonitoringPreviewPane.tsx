// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useMemo} from "react"
import {useTranslation} from "react-i18next"
import {Alert, Box, LinearProgress, Paper, Typography} from "@mui/material"
import {EMonitoringRenderState, type IMonitoringRenderResponse} from "./types"
import {EPreviewStatus} from "./yamlDraft"
import {previewDocument} from "./previewDocument"

export interface MonitoringPreviewPaneProps {
    title?: string
    preview?: IMonitoringRenderResponse
    status: EPreviewStatus
    error?: string
    /** Frame height: the widget's `height`, else 280 like the dashboard. */
    height?: number
}

export const DEFAULT_PREVIEW_HEIGHT = 280

/** A draft rendered by the server, in a frame that runs nothing and fetches nothing. */
export const MonitoringPreviewPane: React.FC<MonitoringPreviewPaneProps> = ({
    title,
    preview,
    status,
    error,
    height = DEFAULT_PREVIEW_HEIGHT,
}) => {
    const {t} = useTranslation()
    const document = useMemo(
        () =>
            preview?.state === EMonitoringRenderState.RENDERED && preview.svg
                ? previewDocument(preview.svg)
                : undefined,
        [preview?.state, preview?.svg]
    )
    const label = title ?? t("monitoring.editor.preview.title")
    return (
        <Paper
            variant="outlined"
            component="section"
            aria-label={label}
            sx={{p: 1.5, display: "flex", flexDirection: "column", gap: 1}}
        >
            <Typography variant="subtitle2" component="h3">
                {label}
            </Typography>
            <Box sx={{height: 4}}>
                {status === EPreviewStatus.RENDERING ? (
                    <LinearProgress aria-label={t("monitoring.editor.footer.rendering")} />
                ) : null}
            </Box>
            {status === EPreviewStatus.FAILED ? (
                <Alert severity="error">
                    {t("monitoring.editor.preview.failed", {reason: error ?? ""})}
                </Alert>
            ) : null}
            {document ? (
                <iframe
                    title={label}
                    sandbox=""
                    srcDoc={document}
                    style={{width: "100%", height, border: 0}}
                />
            ) : preview && preview.state !== EMonitoringRenderState.RENDERED ? (
                <Alert
                    severity={preview.state === EMonitoringRenderState.INVALID ? "error" : "info"}
                >
                    <strong>{t(`monitoring.editor.preview.state.${preview.state}`)}</strong>
                    {preview.reason ? ` · ${preview.reason}` : ""}
                    {preview.state === EMonitoringRenderState.NOT_CONNECTED
                        ? ` ${t("monitoring.editor.preview.notConnectedTail")}`
                        : ""}
                </Alert>
            ) : !preview && status !== EPreviewStatus.RENDERING ? (
                <Typography variant="body2" color="text.secondary">
                    {t("monitoring.editor.preview.empty")}
                </Typography>
            ) : null}
            {preview?.notices?.map((notice) => (
                <Typography key={notice} variant="caption" color="text.secondary">
                    {notice}
                </Typography>
            ))}
        </Paper>
    )
}

export default MonitoringPreviewPane
