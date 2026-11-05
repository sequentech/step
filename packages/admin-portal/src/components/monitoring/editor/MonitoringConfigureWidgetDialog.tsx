// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useCallback, useEffect, useMemo, useRef, useState} from "react"
import {useTranslation} from "react-i18next"
import {
    Alert,
    Box,
    CircularProgress,
    Dialog,
    DialogContent,
    DialogTitle,
    Tab,
    Tabs,
} from "@mui/material"
import {countBySeverity, type IEditorDiagnostic} from "@/components/monitoring/lib/diagnostics"
import {previewWidth, widthBucket} from "@/components/monitoring/lib/chartDocument"
import {DEFAULT_WIDGET_HEIGHT} from "@/components/monitoring/types"
import type {IMonitoringEditorApi} from "./api"
import {
    EMonitoringColorScheme,
    EMonitoringConfigKind,
    type IMonitoringScope,
    type IMonitoringSourceInfo,
} from "./types"
import {useYamlDraft} from "./useYamlDraft"
import {useRedrawOnWidth} from "./useRedrawOnWidth"
import type {TLocalValidate, TRenderPreview} from "./yamlDraft"
import {MonitoringYamlEditor, type IMonitoringYamlEditorHandle} from "./MonitoringYamlEditor"
import {MonitoringDiagnosticsList} from "./MonitoringDiagnosticsList"
import {MonitoringPreviewFooter} from "./MonitoringPreviewFooter"
import {MonitoringPreviewPane} from "./MonitoringPreviewPane"
import {MonitoringDataQueryForm} from "./MonitoringDataQueryForm"
import {MonitoringSelectorsForm} from "./MonitoringSelectorsForm"
import {MonitoringConflictDialog} from "./MonitoringConflictDialog"
import {MonitoringDiscardDialog} from "./MonitoringDiscardDialog"
import {asWidget} from "./formValues"
import {
    EDocumentLoad,
    authorName,
    useConfigDocument,
    type IDocumentMessages,
} from "./useConfigDocument"

export enum EConfigureTab {
    DATA_QUERY = "DATA_QUERY",
    SELECTORS = "SELECTORS",
    YAML = "YAML",
    PREVIEW = "PREVIEW",
}

/** Where the widget is being looked at, so the preview draws what the dashboard would. */
export interface IMonitoringWidgetContext {
    dashboardId: string
    electionId?: string | null
    scope?: IMonitoringScope
    /** "All authorized Posts", a Post, a country: shown in the footer. */
    scopeLabel?: string
    selectorValues?: Record<string, string>
    sources?: Record<string, IMonitoringSourceInfo>
    /** The card's width when opened from one; else the preview pane's is used. */
    width?: number
    colorScheme?: EMonitoringColorScheme
}

export interface MonitoringConfigureWidgetDialogProps extends IMonitoringWidgetContext {
    open: boolean
    widgetId: string
    api: IMonitoringEditorApi
    /** sequent-core's checks in the browser; without them only the server's show. */
    localValidate?: TLocalValidate
    onClose: () => void
    onSaved?: (revision: number) => void
}

const EDITOR_HEIGHT = 360
const NO_SOURCES: Record<string, IMonitoringSourceInfo> = {}
const WIDGET_MESSAGES: IDocumentMessages = {
    loadFailed: "monitoring.editor.configureWidget.loadFailed",
    saved: "monitoring.editor.configureWidget.saved",
    refused: "monitoring.editor.configureWidget.refused",
    validated: "monitoring.editor.configureWidget.validated",
    invalid: "monitoring.editor.configureWidget.invalid",
    requestFailed: "monitoring.editor.configureWidget.requestFailed",
}

/**
 * Configure widget: the widget's YAML, with forms for its query and its
 * selectors that edit that YAML, a live preview, and every problem found.
 */
export const MonitoringConfigureWidgetDialog: React.FC<MonitoringConfigureWidgetDialogProps> = ({
    open,
    widgetId,
    api,
    localValidate,
    onClose,
    onSaved,
    dashboardId,
    electionId,
    scope,
    scopeLabel,
    selectorValues,
    sources = NO_SOURCES,
    width,
    colorScheme = EMonitoringColorScheme.LIGHT,
}) => {
    const {t, i18n} = useTranslation()
    const [tab, setTab] = useState(EConfigureTab.DATA_QUERY)
    const [confirmDiscard, setConfirmDiscard] = useState(false)
    const editor = useRef<IMonitoringYamlEditorHandle>(null)
    const [paneWidth, setPaneWidth] = useState<number | null>(null)
    const [largeWidth, setLargeWidth] = useState<number | null>(null)
    // Beside the forms the draft is drawn as its card is; on the Preview tab,
    // as large as the dialog allows.
    const drawnWidth =
        tab === EConfigureTab.PREVIEW && largeWidth
            ? widthBucket(largeWidth)
            : previewWidth(width, paneWidth)

    const renderPreview = useCallback<TRenderPreview>(
        (text) =>
            api.renderWidget({
                dashboard_id: dashboardId,
                widget_id: widgetId,
                election_id: electionId ?? null,
                scope: scope ?? {},
                selector_values: selectorValues ?? {},
                width: drawnWidth,
                color_scheme: colorScheme,
                locale: i18n.language,
                draft: {widget_yaml: text},
            }),
        [
            api,
            dashboardId,
            widgetId,
            electionId,
            scope,
            selectorValues,
            drawnWidth,
            colorScheme,
            i18n,
        ]
    )
    const draft = useYamlDraft({text: "", localValidate, renderPreview})
    const {controller} = draft

    const stored = useConfigDocument({
        api,
        kind: EMonitoringConfigKind.WIDGET,
        key: widgetId,
        open,
        controller,
        messages: WIDGET_MESSAGES,
        onSaved,
    })
    const {load, revision, busy, message, setMessage, conflict} = stored
    useRedrawOnWidth(controller, drawnWidth, load === EDocumentLoad.READY)

    useEffect(() => {
        if (open) setTab(EConfigureTab.DATA_QUERY)
    }, [open, widgetId])

    const counts = useMemo(() => countBySeverity(draft.diagnostics), [draft.diagnostics])
    const patch = useCallback(
        (change: (text: string) => string) => controller.patch(change),
        [controller]
    )

    const reveal = (diagnostic: IEditorDiagnostic) => {
        setTab(EConfigureTab.YAML)
        // The YAML panel is mounted but hidden; show it before moving the cursor.
        setTimeout(() => editor.current?.reveal(diagnostic.from, diagnostic.to), 0)
    }

    const cancel = () => {
        if (draft.dirty) setConfirmDiscard(true)
        else onClose()
    }

    const title = asWidget(draft.value).title
    const height = asWidget(draft.value).height
    const frameHeight = typeof height === "number" && height > 0 ? height : DEFAULT_WIDGET_HEIGHT
    const previewing = tab === EConfigureTab.PREVIEW
    const diagnosticsList = (
        <MonitoringDiagnosticsList
            diagnostics={draft.diagnostics}
            localUnavailable={draft.localUnavailable}
            onSelect={reveal}
        />
    )
    const table = draft.preview?.table

    return (
        <Dialog
            open={open}
            onClose={cancel}
            maxWidth="xl"
            fullWidth
            aria-labelledby="monitoring-configure-title"
        >
            <DialogTitle id="monitoring-configure-title">
                {t("monitoring.editor.configureWidget.title")}
                {title ? ` · ${title}` : ""}
            </DialogTitle>
            <DialogContent dividers>
                {load === EDocumentLoad.LOADING ? (
                    <Box sx={{display: "flex", justifyContent: "center", py: 6}}>
                        <CircularProgress
                            aria-label={t("monitoring.editor.configureWidget.loading")}
                        />
                    </Box>
                ) : null}
                {load === EDocumentLoad.FAILED ? (
                    <Alert severity="error">{stored.loadError}</Alert>
                ) : null}
                {load === EDocumentLoad.READY ? (
                    <Box
                        sx={{
                            display: "grid",
                            gap: 2,
                            gridTemplateColumns: previewing
                                ? "1fr"
                                : {xs: "1fr", md: "minmax(0, 3fr) minmax(0, 2fr)"},
                        }}
                    >
                        <Box sx={{minWidth: 0}}>
                            <Tabs
                                value={tab}
                                onChange={(_, next: EConfigureTab) => setTab(next)}
                                aria-label={t("monitoring.editor.configureWidget.title")}
                                sx={{mb: 2}}
                            >
                                <Tab
                                    value={EConfigureTab.DATA_QUERY}
                                    id="configure-tab-data"
                                    aria-controls="configure-panel-data"
                                    label={t("monitoring.editor.configureWidget.tabs.dataQuery")}
                                />
                                <Tab
                                    value={EConfigureTab.SELECTORS}
                                    id="configure-tab-selectors"
                                    aria-controls="configure-panel-selectors"
                                    label={t("monitoring.editor.configureWidget.tabs.selectors")}
                                />
                                <Tab
                                    value={EConfigureTab.YAML}
                                    id="configure-tab-yaml"
                                    aria-controls="configure-panel-yaml"
                                    label={t("monitoring.editor.configureWidget.tabs.yaml")}
                                />
                                <Tab
                                    value={EConfigureTab.PREVIEW}
                                    id="configure-tab-preview"
                                    aria-controls="configure-panel-preview"
                                    label={t("monitoring.editor.configureWidget.tabs.preview")}
                                />
                            </Tabs>
                            <div
                                role="tabpanel"
                                id="configure-panel-data"
                                aria-labelledby="configure-tab-data"
                                hidden={tab !== EConfigureTab.DATA_QUERY}
                            >
                                {tab === EConfigureTab.DATA_QUERY ? (
                                    <MonitoringDataQueryForm
                                        value={draft.value}
                                        formAccess={draft.formAccess}
                                        sources={sources}
                                        onPatch={patch}
                                        table={table}
                                    />
                                ) : null}
                            </div>
                            <div
                                role="tabpanel"
                                id="configure-panel-selectors"
                                aria-labelledby="configure-tab-selectors"
                                hidden={tab !== EConfigureTab.SELECTORS}
                            >
                                {tab === EConfigureTab.SELECTORS ? (
                                    <MonitoringSelectorsForm
                                        value={draft.value}
                                        formAccess={draft.formAccess}
                                        onPatch={patch}
                                    />
                                ) : null}
                            </div>
                            <div
                                role="tabpanel"
                                id="configure-panel-yaml"
                                aria-labelledby="configure-tab-yaml"
                                hidden={tab !== EConfigureTab.YAML}
                            >
                                <MonitoringYamlEditor
                                    ref={editor}
                                    label={t("monitoring.editor.yaml.label")}
                                    value={draft.text}
                                    onChange={(text) => controller.setText(text)}
                                    diagnostics={draft.diagnostics}
                                    minHeight={EDITOR_HEIGHT}
                                />
                            </div>
                            <div
                                role="tabpanel"
                                id="configure-panel-preview"
                                aria-labelledby="configure-tab-preview"
                                hidden={!previewing}
                            >
                                {previewing ? (
                                    <Box sx={{display: "flex", flexDirection: "column", gap: 2}}>
                                        <MonitoringPreviewPane
                                            title={title}
                                            preview={draft.preview}
                                            status={draft.previewStatus}
                                            error={draft.previewError}
                                            height={frameHeight}
                                            onWidth={setLargeWidth}
                                        />
                                        {diagnosticsList}
                                    </Box>
                                ) : null}
                            </div>
                        </Box>
                        {previewing ? null : (
                            <Box
                                sx={{display: "flex", flexDirection: "column", gap: 2, minWidth: 0}}
                            >
                                <MonitoringPreviewPane
                                    title={title}
                                    preview={draft.preview}
                                    status={draft.previewStatus}
                                    error={draft.previewError}
                                    height={frameHeight}
                                    onWidth={setPaneWidth}
                                />
                                {diagnosticsList}
                            </Box>
                        )}
                    </Box>
                ) : null}
                {message ? (
                    <Alert severity={message.tone} sx={{mt: 2}} onClose={() => setMessage(null)}>
                        {message.text}
                    </Alert>
                ) : null}
            </DialogContent>
            <Box sx={{px: 3, py: 1.5}}>
                <MonitoringPreviewFooter
                    scopeLabel={scopeLabel}
                    errors={counts.errors}
                    warnings={counts.warnings}
                    previewStatus={draft.previewStatus}
                    renderMs={draft.preview?.render_ms}
                    revision={revision.revision}
                    savedAt={revision.createdAt}
                    savedBy={authorName(revision.author)}
                    dirty={draft.dirty}
                    busy={busy}
                    saveLabel={t("monitoring.editor.configureWidget.save")}
                    saveBlocked={load !== EDocumentLoad.READY || !draft.dirty}
                    onCancel={cancel}
                    onValidate={load === EDocumentLoad.READY ? stored.validate : undefined}
                    onSave={() => void stored.save()}
                />
            </Box>
            <MonitoringDiscardDialog
                open={confirmDiscard}
                body={t("monitoring.editor.configureWidget.discardBody")}
                onKeepEditing={() => setConfirmDiscard(false)}
                onDiscard={() => {
                    setConfirmDiscard(false)
                    onClose()
                }}
            />
            {conflict ? (
                <MonitoringConflictDialog
                    open
                    mine={draft.text}
                    theirs={conflict.theirs}
                    theirsError={conflict.theirsError}
                    currentRevision={conflict.currentRevision}
                    author={conflict.author}
                    time={conflict.time}
                    onReload={stored.reload}
                    onKeepEditing={stored.keepEditing}
                />
            ) : null}
        </Dialog>
    )
}

export default MonitoringConfigureWidgetDialog
