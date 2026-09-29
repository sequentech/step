// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useCallback, useEffect, useMemo, useRef, useState} from "react"
import {useTranslation} from "react-i18next"
import {
    Alert,
    Box,
    Button,
    CircularProgress,
    Dialog,
    DialogActions,
    DialogContent,
    DialogContentText,
    DialogTitle,
    Tab,
    Tabs,
} from "@mui/material"
import {countBySeverity, type IEditorDiagnostic} from "@/components/monitoring/lib/diagnostics"
import type {IMonitoringEditorApi} from "./api"
import {
    EMonitoringColorScheme,
    EMonitoringConfigKind,
    EMonitoringSaveChange,
    EMonitoringSaveStatus,
    EMonitoringValidationResult,
    type IMonitoringAuthor,
    type IMonitoringConfigDocument,
    type IMonitoringScope,
    type IMonitoringSourceInfo,
} from "./types"
import {useYamlDraft} from "./useYamlDraft"
import type {TLocalValidate, TRenderPreview} from "./yamlDraft"
import {MonitoringYamlEditor, type IMonitoringYamlEditorHandle} from "./MonitoringYamlEditor"
import {MonitoringDiagnosticsList} from "./MonitoringDiagnosticsList"
import {EEditorBusy, MonitoringPreviewFooter} from "./MonitoringPreviewFooter"
import {MonitoringPreviewPane} from "./MonitoringPreviewPane"
import {MonitoringDataQueryForm} from "./MonitoringDataQueryForm"
import {MonitoringSelectorsForm} from "./MonitoringSelectorsForm"
import {MonitoringConflictDialog} from "./MonitoringConflictDialog"
import {asWidget} from "./formValues"

export enum EConfigureTab {
    DATA_QUERY = "DATA_QUERY",
    SELECTORS = "SELECTORS",
    YAML = "YAML",
}

enum ELoad {
    LOADING = "LOADING",
    READY = "READY",
    FAILED = "FAILED",
}

enum EMessageTone {
    SUCCESS = "success",
    INFO = "info",
    ERROR = "error",
}

interface IMessage {
    tone: EMessageTone
    text: string
}

interface IRevision {
    revision: number | null
    author?: IMonitoringAuthor | null
    createdAt?: string | null
}

interface IConflict {
    currentRevision: number
    author?: IMonitoringAuthor | null
    time?: string | null
    theirs?: string
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

const PREVIEW_WIDTH = 480
const EDITOR_HEIGHT = 360
const NO_SOURCES: Record<string, IMonitoringSourceInfo> = {}
const reason = (error: unknown) => (error instanceof Error ? error.message : String(error))

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
    width = PREVIEW_WIDTH,
    colorScheme = EMonitoringColorScheme.LIGHT,
}) => {
    const {t, i18n} = useTranslation()
    const [tab, setTab] = useState(EConfigureTab.DATA_QUERY)
    const [load, setLoad] = useState(ELoad.LOADING)
    const [loadError, setLoadError] = useState("")
    const [revision, setRevision] = useState<IRevision>({revision: null})
    /** The revision a save replaces; moves on when the author has seen a newer one. */
    const [expected, setExpected] = useState<number | null>(null)
    const [busy, setBusy] = useState(EEditorBusy.IDLE)
    const [message, setMessage] = useState<IMessage | null>(null)
    const [confirmDiscard, setConfirmDiscard] = useState(false)
    const [conflict, setConflict] = useState<IConflict | null>(null)
    const editor = useRef<IMonitoringYamlEditorHandle>(null)

    const renderPreview = useCallback<TRenderPreview>(
        (text) =>
            api.renderWidget({
                dashboard_id: dashboardId,
                widget_id: widgetId,
                election_id: electionId ?? null,
                scope: scope ?? {},
                selector_values: selectorValues ?? {},
                width,
                color_scheme: colorScheme,
                locale: i18n.language,
                draft: {widget_yaml: text},
            }),
        [api, dashboardId, widgetId, electionId, scope, selectorValues, width, colorScheme, i18n]
    )
    const draft = useYamlDraft({text: "", localValidate, renderPreview})
    const {controller} = draft

    const adopt = useCallback(
        (document: IMonitoringConfigDocument) => {
            controller.reset(document.yaml)
            setRevision({
                revision: document.revision,
                author: document.author,
                createdAt: document.created_at,
            })
            setExpected(document.revision)
        },
        [controller]
    )

    useEffect(() => {
        if (!open) return
        let current = true
        setLoad(ELoad.LOADING)
        setMessage(null)
        setTab(EConfigureTab.DATA_QUERY)
        api.getConfig({kind: EMonitoringConfigKind.WIDGET, key: widgetId}).then(
            (document) => {
                if (!current) return
                adopt(document)
                setLoad(ELoad.READY)
            },
            (error) => {
                if (!current) return
                setLoadError(reason(error))
                setLoad(ELoad.FAILED)
            }
        )
        return () => {
            current = false
        }
    }, [open, api, widgetId, adopt])

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

    const validate = async () => {
        setBusy(EEditorBusy.VALIDATING)
        setMessage(null)
        try {
            const response = await api.validateConfig({
                kind: EMonitoringConfigKind.WIDGET,
                key: widgetId,
                yaml: draft.text,
            })
            if (response.preview) controller.acceptPreview(response.preview)
            controller.setServerProblems(response.problems)
            setMessage(
                response.result === EMonitoringValidationResult.VALID
                    ? {
                          tone: EMessageTone.SUCCESS,
                          text: t("monitoring.editor.configureWidget.validated"),
                      }
                    : {
                          tone: EMessageTone.ERROR,
                          text: t("monitoring.editor.configureWidget.invalid"),
                      }
            )
        } catch (error) {
            setMessage({
                tone: EMessageTone.ERROR,
                text: t("monitoring.editor.configureWidget.requestFailed", {reason: reason(error)}),
            })
        } finally {
            setBusy(EEditorBusy.IDLE)
        }
    }

    const save = async () => {
        setBusy(EEditorBusy.SAVING)
        setMessage(null)
        const yaml = draft.text
        try {
            const outcome = await api.saveConfig({
                kind: EMonitoringConfigKind.WIDGET,
                key: widgetId,
                yaml,
                expected_revision: expected,
                change: EMonitoringSaveChange.UPSERT,
            })
            if (outcome.status === EMonitoringSaveStatus.SAVED) {
                controller.markSaved()
                setRevision({revision: outcome.revision, createdAt: new Date().toISOString()})
                setExpected(outcome.revision)
                setMessage({
                    tone: EMessageTone.SUCCESS,
                    text: t("monitoring.editor.configureWidget.saved", {
                        revision: outcome.revision,
                    }),
                })
                onSaved?.(outcome.revision)
            } else if (outcome.status === EMonitoringSaveStatus.CONFLICT) {
                setConflict({
                    currentRevision: outcome.current_revision,
                    author: outcome.author,
                    time: outcome.time,
                })
                api.getConfig({kind: EMonitoringConfigKind.WIDGET, key: widgetId}).then(
                    (document) =>
                        setConflict((previous) =>
                            previous ? {...previous, theirs: document.yaml} : previous
                        ),
                    (error) =>
                        setMessage({
                            tone: EMessageTone.ERROR,
                            text: t("monitoring.editor.configureWidget.requestFailed", {
                                reason: reason(error),
                            }),
                        })
                )
            } else {
                controller.setServerProblems(outcome.problems)
                setMessage({
                    tone: EMessageTone.ERROR,
                    text: t("monitoring.editor.configureWidget.refused"),
                })
            }
        } catch (error) {
            setMessage({
                tone: EMessageTone.ERROR,
                text: t("monitoring.editor.configureWidget.requestFailed", {reason: reason(error)}),
            })
        } finally {
            setBusy(EEditorBusy.IDLE)
        }
    }

    const reload = async () => {
        if (!conflict) return
        setConflict(null)
        try {
            adopt(await api.getConfig({kind: EMonitoringConfigKind.WIDGET, key: widgetId}))
        } catch (error) {
            setMessage({
                tone: EMessageTone.ERROR,
                text: t("monitoring.editor.configureWidget.requestFailed", {reason: reason(error)}),
            })
        }
    }

    const keepEditing = () => {
        // The author has seen the newer revision; saving now replaces it knowingly.
        if (conflict) setExpected(conflict.currentRevision)
        setConflict(null)
    }

    const cancel = () => {
        if (draft.dirty) setConfirmDiscard(true)
        else onClose()
    }

    const title = asWidget(draft.value).title
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
                {load === ELoad.LOADING ? (
                    <Box sx={{display: "flex", justifyContent: "center", py: 6}}>
                        <CircularProgress
                            aria-label={t("monitoring.editor.configureWidget.loading")}
                        />
                    </Box>
                ) : null}
                {load === ELoad.FAILED ? (
                    <Alert severity="error">
                        {t("monitoring.editor.configureWidget.loadFailed", {reason: loadError})}
                    </Alert>
                ) : null}
                {load === ELoad.READY ? (
                    <Box
                        sx={{
                            display: "grid",
                            gap: 2,
                            gridTemplateColumns: {xs: "1fr", md: "minmax(0, 3fr) minmax(0, 2fr)"},
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
                        </Box>
                        <Box sx={{display: "flex", flexDirection: "column", gap: 2, minWidth: 0}}>
                            <MonitoringPreviewPane
                                title={title}
                                preview={draft.preview}
                                status={draft.previewStatus}
                                error={draft.previewError}
                            />
                            <MonitoringDiagnosticsList
                                diagnostics={draft.diagnostics}
                                localUnavailable={draft.localUnavailable}
                                onSelect={reveal}
                            />
                        </Box>
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
                    savedBy={revision.author ? revision.author.name || revision.author.id : null}
                    dirty={draft.dirty}
                    busy={busy}
                    saveLabel={t("monitoring.editor.configureWidget.save")}
                    saveBlocked={load !== ELoad.READY || !draft.dirty}
                    onCancel={cancel}
                    onValidate={load === ELoad.READY ? validate : undefined}
                    onSave={save}
                />
            </Box>
            <Dialog
                open={confirmDiscard}
                onClose={() => setConfirmDiscard(false)}
                aria-labelledby="monitoring-discard-title"
            >
                <DialogTitle id="monitoring-discard-title">
                    {t("monitoring.editor.configureWidget.discardTitle")}
                </DialogTitle>
                <DialogContent>
                    <DialogContentText>
                        {t("monitoring.editor.configureWidget.discardBody")}
                    </DialogContentText>
                </DialogContent>
                <DialogActions>
                    <Button onClick={() => setConfirmDiscard(false)}>
                        {t("monitoring.editor.configureWidget.keepEditing")}
                    </Button>
                    <Button
                        color="error"
                        onClick={() => {
                            setConfirmDiscard(false)
                            onClose()
                        }}
                    >
                        {t("monitoring.editor.configureWidget.discard")}
                    </Button>
                </DialogActions>
            </Dialog>
            {conflict ? (
                <MonitoringConflictDialog
                    open
                    mine={draft.text}
                    theirs={conflict.theirs}
                    currentRevision={conflict.currentRevision}
                    author={conflict.author}
                    time={conflict.time}
                    onReload={reload}
                    onKeepEditing={keepEditing}
                />
            ) : null}
        </Dialog>
    )
}

export default MonitoringConfigureWidgetDialog
