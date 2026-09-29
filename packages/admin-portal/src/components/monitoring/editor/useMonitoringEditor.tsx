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

/**
 * The editor's entry points, shaped like the view's `MonitoringEditorActions`
 * so the dashboard's ⋯ menu can call them directly.
 */
export interface IMonitoringEditorEntryPoints {
    onConfigureWidget: (widgetId: string) => void
}

export interface IMonitoringEditorOptions {
    api: IMonitoringEditorApi
    /** Where the widgets are being looked at: the dashboard shown, its scope and sources. */
    context: IMonitoringWidgetContext
    localValidate?: TLocalValidate
    onSaved?: () => void
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

/** Opens the editor's dialogs from the dashboard view. The dialog code loads on first use. */
export const useMonitoringEditor = ({
    api,
    context,
    localValidate,
    onSaved,
}: IMonitoringEditorOptions): IMonitoringEditor => {
    const [open, setOpen] = useState<IOpenWidget | null>(null)
    const validate = useMemo(
        () => localValidate ?? sequentCoreValidator(EMonitoringConfigKind.WIDGET),
        [localValidate]
    )
    const onConfigure = useCallback(
        (widgetId: string, override?: Partial<IMonitoringWidgetContext>) =>
            setOpen({widgetId, context: override}),
        []
    )
    const actions = useMemo(
        () => ({onConfigureWidget: (id: string) => onConfigure(id)}),
        [onConfigure]
    )
    const element = open ? (
        <Suspense fallback={null}>
            <MonitoringConfigureWidgetDialog
                open
                api={api}
                widgetId={open.widgetId}
                localValidate={validate}
                {...context}
                {...open.context}
                onClose={() => setOpen(null)}
                onSaved={() => onSaved?.()}
            />
        </Suspense>
    ) : null
    return {actions, onConfigure, element}
}
