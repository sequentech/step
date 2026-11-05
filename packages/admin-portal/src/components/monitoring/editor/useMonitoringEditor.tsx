// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {Suspense, useCallback, useEffect, useMemo, useRef, useState} from "react"
import {useQuery} from "@apollo/client"
import {useTranslation} from "react-i18next"
import {Alert, Snackbar} from "@mui/material"
import {MONITORING_GET_DASHBOARD} from "@/queries/MonitoringGetDashboard"
import {useMonitoring} from "@/components/monitoring/MonitoringProvider"
import {
    EMonitoringViewMode,
    type MonitoringGetDashboardQuery,
    type MonitoringGetDashboardVariables,
} from "@/components/monitoring/types"
import {monitoringErrorMessage, type IMonitoringEditorApi} from "./api"
import {
    EDuplicateResult,
    EMonitoringConfigKind,
    EMonitoringSaveStatus,
    type TDuplicateOutcome,
} from "./types"
import type {IMonitoringWidgetContext} from "./MonitoringConfigureWidgetDialog"
import type {TLocalValidate} from "./yamlDraft"
import {sequentCoreValidator} from "./sequentCoreValidator"
import {previewScope} from "./viewContext"

const MonitoringConfigureWidgetDialog = React.lazy(
    () => import("./MonitoringConfigureWidgetDialog")
)
const MonitoringDashboardEditor = React.lazy(() => import("./MonitoringDashboardEditor"))

/**
 * The editor's entry points, shaped like the view's `MonitoringEditorActions`
 * so the dashboard's ⋯ menu and its Edit button can call them directly.
 */
export interface IMonitoringEditorEntryPoints {
    onConfigureWidget: (widgetId: string, width?: number) => void
    onDuplicateWidget: (widgetId: string) => void
    onEditDashboard: (dashboardId: string) => void
}

/** The event and election the view shows, to find its dashboard in Apollo's cache. */
export interface IMonitoringEditorView {
    electionEventId: string
    electionId?: string | null
}

export interface IMonitoringEditorOptions {
    api: IMonitoringEditorApi
    /**
     * Where the widgets are being looked at: the dashboard shown, its scope
     * and sources. Without it, the editor reads them from the
     * `MonitoringProvider` its element is rendered under and from the
     * dashboard the view last loaded for `view`.
     */
    context?: IMonitoringWidgetContext
    view?: IMonitoringEditorView
    /** Per kind; sequent-core's WebAssembly checks by default. */
    localValidate?: Partial<Record<EMonitoringConfigKind, TLocalValidate>>
    /** A document was saved or the event reset to a preset: reload the view. */
    onChanged?: () => void
}

export interface IMonitoringEditor {
    actions: IMonitoringEditorEntryPoints
    /** `onConfigure(widgetId)`: opens Configure widget, for the dashboard in `context`. */
    onConfigure: (widgetId: string, context?: Partial<IMonitoringWidgetContext>) => void
    /**
     * The dialogs and the editor's notices. Render it once, under the Apollo
     * and i18n providers and, without `context`, under the view's
     * `MonitoringProvider`, whose polling it pauses while `editing`.
     */
    element: React.ReactNode
    /** A dialog is open or a copy is being made. */
    editing: boolean
}

interface IOpenWidget {
    widgetId: string
    context?: Partial<IMonitoringWidgetContext>
}

enum ENoticeTone {
    SUCCESS = "success",
    WARNING = "warning",
    ERROR = "error",
}

interface INotice {
    tone: ENoticeTone
    text: string
}

type TTranslate = (key: string, options?: Record<string, unknown>) => string

const validatorFor = (
    kind: EMonitoringConfigKind,
    given?: Partial<Record<EMonitoringConfigKind, TLocalValidate>>
) => given?.[kind] ?? sequentCoreValidator(kind)

/**
 * The view's context, as the dashboard shown last drew it: the Provider's
 * choices, and the sources of the dashboard the view last loaded.
 */
const useViewContext = (
    given: IMonitoringWidgetContext | undefined,
    view: IMonitoringEditorView | undefined
): IMonitoringWidgetContext | undefined => {
    const {state} = useMonitoring()
    const dashboardId = state.dashboardId ?? ""
    const electionId = view?.electionId ?? null
    const {data} = useQuery<MonitoringGetDashboardQuery, MonitoringGetDashboardVariables>(
        MONITORING_GET_DASHBOARD,
        {
            variables: {electionEventId: view?.electionEventId ?? "", electionId, dashboardId},
            fetchPolicy: "cache-only",
            skip: Boolean(given) || !view || !dashboardId,
        }
    )
    return useMemo(() => {
        if (given) return given
        if (!dashboardId) return undefined
        const response = data?.monitoringGetDashboard
        return {
            dashboardId,
            electionId,
            scope: previewScope(state.dashboardValues, response?.dashboard, electionId),
            sources: response?.sources,
        }
    }, [given, dashboardId, data, state.dashboardValues, electionId])
}

/** Pauses the view's polling while the editor works on the dashboard it shows. */
const usePausePolling = (editing: boolean) => {
    const {setMode} = useMonitoring()
    const setModeRef = useRef(setMode)
    setModeRef.current = setMode
    useEffect(() => {
        if (!editing) return
        setModeRef.current(EMonitoringViewMode.EDIT)
        return () => setModeRef.current(EMonitoringViewMode.VIEW)
    }, [editing])
}

interface IEditorElementProps {
    api: IMonitoringEditorApi
    given?: IMonitoringWidgetContext
    view?: IMonitoringEditorView
    /** Filled with the context shown, for the actions that run outside a dialog. */
    contextRef: React.MutableRefObject<IMonitoringWidgetContext | undefined>
    editing: boolean
    widget: IOpenWidget | null
    dashboardId: string | null
    validators: Record<"widget" | "dashboard" | "theme", TLocalValidate>
    notice: INotice | null
    onConfigure: (widgetId: string, context?: Partial<IMonitoringWidgetContext>) => void
    onCloseWidget: () => void
    onCloseDashboard: () => void
    onCloseNotice: () => void
    onChanged: () => void
}

const EditorElement: React.FC<IEditorElementProps> = ({
    api,
    given,
    view,
    contextRef,
    editing,
    widget,
    dashboardId,
    validators,
    notice,
    onConfigure,
    onCloseWidget,
    onCloseDashboard,
    onCloseNotice,
    onChanged,
}) => {
    const context = useViewContext(given, view)
    useEffect(() => {
        contextRef.current = context
    }, [contextRef, context])
    usePausePolling(editing)
    const widgetContext = {...context, ...widget?.context}
    const widgetDashboard = widgetContext.dashboardId
    return (
        <>
            <Suspense fallback={null}>
                {dashboardId ? (
                    <MonitoringDashboardEditor
                        open
                        api={api}
                        dashboardId={dashboardId}
                        localValidate={validators.dashboard}
                        themeValidate={validators.theme}
                        onConfigureWidget={(id) => onConfigure(id, {dashboardId})}
                        onClose={onCloseDashboard}
                        onSaved={onChanged}
                        onReset={onChanged}
                    />
                ) : null}
                {widget && widgetDashboard ? (
                    <MonitoringConfigureWidgetDialog
                        open
                        api={api}
                        widgetId={widget.widgetId}
                        localValidate={validators.widget}
                        {...widgetContext}
                        dashboardId={widgetDashboard}
                        onClose={onCloseWidget}
                        onSaved={onChanged}
                    />
                ) : null}
            </Suspense>
            <Snackbar open={Boolean(notice)} autoHideDuration={8000} onClose={onCloseNotice}>
                {notice ? (
                    <Alert severity={notice.tone} onClose={onCloseNotice}>
                        {notice.text}
                    </Alert>
                ) : undefined}
            </Snackbar>
        </>
    )
}

const duplicateNotice = (outcome: TDuplicateOutcome, t: TTranslate): INotice => {
    switch (outcome.result) {
        case EDuplicateResult.DONE:
            return {
                tone: ENoticeTone.SUCCESS,
                text: t("monitoring.editor.duplicate.done", {id: outcome.id}),
            }
        case EDuplicateResult.NOT_SAVED:
            return {
                tone: ENoticeTone.ERROR,
                text: t("monitoring.editor.duplicate.failed", {
                    reason: outcome.problem ?? t("monitoring.editor.configureWidget.refused"),
                }),
            }
        case EDuplicateResult.NOT_PLACED:
            return {
                tone: ENoticeTone.WARNING,
                text: t("monitoring.editor.duplicate.notPlaced", {
                    id: outcome.id,
                    reason:
                        outcome.problem ??
                        (outcome.status === EMonitoringSaveStatus.CONFLICT
                            ? t("monitoring.editor.conflict.title")
                            : t("monitoring.editor.dashboard.refused")),
                }),
            }
    }
}

const failureReason = (error: unknown, t: TTranslate) => {
    const known = monitoringErrorMessage(error)
    if (known) return t(known)
    return error instanceof Error ? error.message : String(error)
}

/** Opens the editor's dialogs from the dashboard view. Their code loads on first use. */
export const useMonitoringEditor = ({
    api,
    context,
    view,
    localValidate,
    onChanged,
}: IMonitoringEditorOptions): IMonitoringEditor => {
    const {t} = useTranslation()
    const [widget, setWidget] = useState<IOpenWidget | null>(null)
    const [dashboardId, setDashboardId] = useState<string | null>(null)
    const [duplicating, setDuplicating] = useState(false)
    const [notice, setNotice] = useState<INotice | null>(null)
    const contextRef = useRef<IMonitoringWidgetContext | undefined>(context)
    const busy = useRef(false)
    const changedRef = useRef(onChanged)
    changedRef.current = onChanged
    const changed = useCallback(() => changedRef.current?.(), [])
    const tRef = useRef<TTranslate>(t)
    tRef.current = t

    const validators = useMemo(
        () => ({
            widget: validatorFor(EMonitoringConfigKind.WIDGET, localValidate),
            dashboard: validatorFor(EMonitoringConfigKind.DASHBOARD, localValidate),
            theme: validatorFor(EMonitoringConfigKind.THEME, localValidate),
        }),
        [localValidate]
    )
    const onConfigure = useCallback(
        (widgetId: string, override?: Partial<IMonitoringWidgetContext>) =>
            setWidget({widgetId, context: override}),
        []
    )

    /** Copies the widget onto the dashboard shown; one copy at a time. */
    const duplicate = useCallback(
        async (widgetId: string) => {
            const target = contextRef.current?.dashboardId
            if (!target || busy.current) return
            busy.current = true
            setDuplicating(true)
            try {
                const {duplicateWidgetOnDashboard} = await import("./duplicateWidget")
                const outcome = await duplicateWidgetOnDashboard(api, {
                    dashboardId: target,
                    widgetId,
                    copyTitle: (title) =>
                        tRef.current("monitoring.editor.duplicate.copyTitle", {title}),
                })
                setNotice(duplicateNotice(outcome, tRef.current))
                if (outcome.result !== EDuplicateResult.NOT_SAVED) changed()
            } catch (error) {
                setNotice({
                    tone: ENoticeTone.ERROR,
                    text: tRef.current("monitoring.editor.duplicate.failed", {
                        reason: failureReason(error, tRef.current),
                    }),
                })
            } finally {
                busy.current = false
                setDuplicating(false)
            }
        },
        [api, changed]
    )

    const actions = useMemo(
        () => ({
            onConfigureWidget: (id: string, width?: number) =>
                onConfigure(id, width ? {width} : undefined),
            onDuplicateWidget: (id: string) => void duplicate(id),
            onEditDashboard: (id: string) => setDashboardId(id),
        }),
        [onConfigure, duplicate]
    )
    const editing = Boolean(widget || dashboardId || duplicating)
    const element = (
        <EditorElement
            api={api}
            given={context}
            view={view}
            contextRef={contextRef}
            editing={editing}
            widget={widget}
            dashboardId={dashboardId}
            validators={validators}
            notice={notice}
            onConfigure={onConfigure}
            onCloseWidget={() => setWidget(null)}
            onCloseDashboard={() => setDashboardId(null)}
            onCloseNotice={() => setNotice(null)}
            onChanged={changed}
        />
    )
    return {actions, onConfigure, element, editing}
}
