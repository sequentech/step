// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {Suspense, useCallback, useMemo, useState} from "react"
import type {IMonitoringEditorApi} from "./api"
import {EMonitoringConfigKind} from "./types"
import type {IMonitoringWidgetContext} from "./MonitoringConfigureWidgetDialog"
import type {TLocalValidate} from "./yamlDraft"
import {sequentCoreValidator} from "./sequentCoreValidator"

const MonitoringConfigureWidgetDialog = React.lazy(
    () => import("./MonitoringConfigureWidgetDialog")
)
const MonitoringDashboardEditor = React.lazy(() => import("./MonitoringDashboardEditor"))

/**
 * The editor's entry points, shaped like the view's `MonitoringEditorActions`
 * so the dashboard's ⋯ menu and its Edit button can call them directly.
 */
export interface IMonitoringEditorEntryPoints {
    onConfigureWidget: (widgetId: string) => void
    onEditDashboard: (dashboardId: string) => void
}

export interface IMonitoringEditorOptions {
    api: IMonitoringEditorApi
    /** Where the widgets are being looked at: the dashboard shown, its scope and sources. */
    context: IMonitoringWidgetContext
    /** Per kind; sequent-core's WebAssembly checks by default. */
    localValidate?: Partial<Record<EMonitoringConfigKind, TLocalValidate>>
    /** A document was saved or the event reset to a preset: reload the view. */
    onChanged?: () => void
}

export interface IMonitoringEditor {
    actions: IMonitoringEditorEntryPoints
    /** `onConfigure(widgetId)`: opens Configure widget, for the dashboard in `context`. */
    onConfigure: (widgetId: string, context?: Partial<IMonitoringWidgetContext>) => void
    /** The dialogs; render it once, anywhere under the Apollo and i18n providers. */
    element: React.ReactNode
}

interface IOpenWidget {
    widgetId: string
    context?: Partial<IMonitoringWidgetContext>
}

const validatorFor = (
    kind: EMonitoringConfigKind,
    given?: Partial<Record<EMonitoringConfigKind, TLocalValidate>>
) => given?.[kind] ?? sequentCoreValidator(kind)

/** Opens the editor's dialogs from the dashboard view. Their code loads on first use. */
export const useMonitoringEditor = ({
    api,
    context,
    localValidate,
    onChanged,
}: IMonitoringEditorOptions): IMonitoringEditor => {
    const [widget, setWidget] = useState<IOpenWidget | null>(null)
    const [dashboardId, setDashboardId] = useState<string | null>(null)
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
    const actions = useMemo(
        () => ({
            onConfigureWidget: (id: string) => onConfigure(id),
            onEditDashboard: (id: string) => setDashboardId(id),
        }),
        [onConfigure]
    )
    const changed = () => onChanged?.()
    const element = (
        <Suspense fallback={null}>
            {dashboardId ? (
                <MonitoringDashboardEditor
                    open
                    api={api}
                    dashboardId={dashboardId}
                    localValidate={validators.dashboard}
                    themeValidate={validators.theme}
                    onConfigureWidget={(id) => onConfigure(id, {dashboardId})}
                    onClose={() => setDashboardId(null)}
                    onSaved={changed}
                    onReset={changed}
                />
            ) : null}
            {widget ? (
                <MonitoringConfigureWidgetDialog
                    open
                    api={api}
                    widgetId={widget.widgetId}
                    localValidate={validators.widget}
                    {...context}
                    {...widget.context}
                    onClose={() => setWidget(null)}
                    onSaved={changed}
                />
            ) : null}
        </Suspense>
    )
    return {actions, onConfigure, element}
}
