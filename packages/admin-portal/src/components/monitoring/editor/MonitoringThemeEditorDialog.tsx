// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useCallback, useMemo, useRef} from "react"
import {useTranslation} from "react-i18next"
import {
    Alert,
    Box,
    CircularProgress,
    Dialog,
    DialogContent,
    DialogTitle,
    Typography,
} from "@mui/material"
import {countBySeverity, type IEditorDiagnostic} from "@/components/monitoring/lib/diagnostics"
import type {IMonitoringEditorApi} from "./api"
import {EMonitoringColorScheme, EMonitoringConfigKind} from "./types"
import {useYamlDraft} from "./useYamlDraft"
import type {TLocalValidate, TRenderPreview} from "./yamlDraft"
import {MonitoringYamlEditor, type IMonitoringYamlEditorHandle} from "./MonitoringYamlEditor"
import {MonitoringDiagnosticsList} from "./MonitoringDiagnosticsList"
import {MonitoringPreviewFooter} from "./MonitoringPreviewFooter"
import {MonitoringPreviewPane} from "./MonitoringPreviewPane"
import {MonitoringConflictDialog} from "./MonitoringConflictDialog"
import {
    EDocumentLoad,
    authorName,
    useConfigDocument,
    type IDocumentMessages,
} from "./useConfigDocument"

export interface MonitoringThemeEditorDialogProps {
    open: boolean
    api: IMonitoringEditorApi
    themeKey: string
    /** How many widgets the theme styles: those of every dashboard that uses it. */
    widgetCount: number
    /** A widget to preview the draft theme on; without it there is no preview. */
    preview?: {dashboardId: string; widgetId: string; width?: number}
    colorScheme?: EMonitoringColorScheme
    localValidate?: TLocalValidate
    onClose: () => void
    onSaved?: (revision: number) => void
}

const THEME_MESSAGES: IDocumentMessages = {
    loadFailed: "monitoring.editor.document.loadFailed",
    saved: "monitoring.editor.theme.saved",
    refused: "monitoring.editor.document.refused",
    validated: "monitoring.editor.document.validated",
    invalid: "monitoring.editor.document.invalid",
    requestFailed: "monitoring.editor.document.requestFailed",
}
const PREVIEW_WIDTH = 480

/** The dbt Charts style of a dashboard, as YAML, previewed on one of its widgets. */
export const MonitoringThemeEditorDialog: React.FC<MonitoringThemeEditorDialogProps> = ({
    open,
    api,
    themeKey,
    widgetCount,
    preview,
    colorScheme = EMonitoringColorScheme.LIGHT,
    localValidate,
    onClose,
    onSaved,
}) => {
    const {t, i18n} = useTranslation()
    const editor = useRef<IMonitoringYamlEditorHandle>(null)
    const renderPreview = useCallback<TRenderPreview>(
        (text) =>
            api.renderWidget({
                dashboard_id: preview?.dashboardId ?? "",
                widget_id: preview?.widgetId ?? "",
                scope: {},
                selector_values: {},
                width: preview?.width ?? PREVIEW_WIDTH,
                color_scheme: colorScheme,
                locale: i18n.language,
                draft: {theme_yaml: text},
            }),
        [api, preview?.dashboardId, preview?.widgetId, preview?.width, colorScheme, i18n]
    )
    const draft = useYamlDraft({
        text: "",
        localValidate,
        renderPreview: preview ? renderPreview : undefined,
    })
    const stored = useConfigDocument({
        api,
        kind: EMonitoringConfigKind.THEME,
        key: themeKey,
        open,
        controller: draft.controller,
        messages: THEME_MESSAGES,
        onSaved,
    })
    const counts = useMemo(() => countBySeverity(draft.diagnostics), [draft.diagnostics])
    const ready = stored.load === EDocumentLoad.READY
    const reveal = (diagnostic: IEditorDiagnostic) =>
        editor.current?.reveal(diagnostic.from, diagnostic.to)

    return (
        <Dialog
            open={open}
            onClose={onClose}
            maxWidth="lg"
            fullWidth
            aria-labelledby="monitoring-theme-title"
        >
            <DialogTitle id="monitoring-theme-title">
                {t("monitoring.editor.theme.title")} · {themeKey}
                <Typography variant="body2" color="text.secondary" component="span" display="block">
                    {t("monitoring.editor.theme.subtitle")} ·{" "}
                    {t("monitoring.editor.theme.appliesTo", {count: widgetCount})}
                </Typography>
            </DialogTitle>
            <DialogContent dividers>
                {stored.load === EDocumentLoad.LOADING ? (
                    <Box sx={{display: "flex", justifyContent: "center", py: 6}}>
                        <CircularProgress aria-label={t("monitoring.editor.theme.title")} />
                    </Box>
                ) : null}
                {stored.load === EDocumentLoad.FAILED ? (
                    <Alert severity="error">{stored.loadError}</Alert>
                ) : null}
                {ready ? (
                    <Box
                        sx={{
                            display: "grid",
                            gap: 2,
                            gridTemplateColumns: {
                                xs: "1fr",
                                md: preview ? "minmax(0, 3fr) minmax(0, 2fr)" : "1fr",
                            },
                        }}
                    >
                        <MonitoringYamlEditor
                            ref={editor}
                            label={t("monitoring.editor.yaml.label")}
                            value={draft.text}
                            onChange={(text) => draft.controller.setText(text)}
                            diagnostics={draft.diagnostics}
                            minHeight={320}
                        />
                        <Box sx={{display: "flex", flexDirection: "column", gap: 2, minWidth: 0}}>
                            {preview ? (
                                <MonitoringPreviewPane
                                    preview={draft.preview}
                                    status={draft.previewStatus}
                                    error={draft.previewError}
                                />
                            ) : null}
                            <MonitoringDiagnosticsList
                                diagnostics={draft.diagnostics}
                                localUnavailable={draft.localUnavailable}
                                onSelect={reveal}
                            />
                        </Box>
                    </Box>
                ) : null}
                {stored.message ? (
                    <Alert
                        severity={stored.message.tone}
                        sx={{mt: 2}}
                        onClose={() => stored.setMessage(null)}
                    >
                        {stored.message.text}
                    </Alert>
                ) : null}
            </DialogContent>
            <Box sx={{px: 3, py: 1.5}}>
                <MonitoringPreviewFooter
                    errors={counts.errors}
                    warnings={counts.warnings}
                    previewStatus={draft.previewStatus}
                    renderMs={draft.preview?.render_ms}
                    revision={stored.revision.revision}
                    savedAt={stored.revision.createdAt}
                    savedBy={authorName(stored.revision.author)}
                    dirty={draft.dirty}
                    busy={stored.busy}
                    saveLabel={t("monitoring.editor.theme.apply")}
                    hasPreview={Boolean(preview)}
                    saveBlocked={!ready || !draft.dirty}
                    onCancel={onClose}
                    onValidate={ready ? stored.validate : undefined}
                    onSave={() => void stored.save()}
                />
            </Box>
            {stored.conflict ? (
                <MonitoringConflictDialog
                    open
                    mine={draft.text}
                    theirs={stored.conflict.theirs}
                    currentRevision={stored.conflict.currentRevision}
                    author={stored.conflict.author}
                    time={stored.conflict.time}
                    onReload={stored.reload}
                    onKeepEditing={stored.keepEditing}
                />
            ) : null}
        </Dialog>
    )
}

export default MonitoringThemeEditorDialog
