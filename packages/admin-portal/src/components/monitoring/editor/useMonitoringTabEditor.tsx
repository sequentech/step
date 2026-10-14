// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {useCallback, type ReactNode} from "react"
import {useApolloClient} from "@apollo/client"
import {MONITORING_GET_DASHBOARD} from "@/queries/MonitoringGetDashboard"
import {MONITORING_LIST_DASHBOARDS} from "@/queries/MonitoringListDashboards"
import {EMonitoringCapability} from "@/components/monitoring/types"
import type {MonitoringEditorActions} from "@/components/monitoring/MonitoringProvider"
import type {IMonitoringEditorApi} from "./api"
import {useMonitoringEditor} from "./useMonitoringEditor"
import {useMonitoringEditorApi} from "./useMonitoringEditorApi"

export interface IMonitoringTabEditorOptions {
    electionEventId: string
    electionId?: string | null
    /** The viewer's `monitoring-configure`; only a granted one gets the editor. */
    configure: EMonitoringCapability
    /** Stories hand in a fake; Harvest's actions otherwise. */
    api?: IMonitoringEditorApi
}

export interface IMonitoringTabEditor {
    /** For the view's `MonitoringProvider`; `undefined` for a viewer who cannot configure. */
    actions?: MonitoringEditorActions
    /** Render once under that Provider. */
    element: ReactNode
}

/**
 * The editor as the Dashboard tab wires it in: its entry points for a
 * viewer who may configure, its dialogs, and a reload of the view whenever
 * a document is saved or the event reset.
 */
export const useMonitoringTabEditor = ({
    electionEventId,
    electionId,
    configure,
    api,
}: IMonitoringTabEditorOptions): IMonitoringTabEditor => {
    const client = useApolloClient()
    const harvest = useMonitoringEditorApi(electionEventId)
    const onChanged = useCallback(() => {
        void client
            .refetchQueries({include: [MONITORING_GET_DASHBOARD, MONITORING_LIST_DASHBOARDS]})
            .catch(() => undefined)
    }, [client])
    const editor = useMonitoringEditor({
        api: api ?? harvest,
        view: {electionEventId, electionId},
        onChanged,
    })
    const granted = configure === EMonitoringCapability.GRANTED
    return {actions: granted ? editor.actions : undefined, element: granted ? editor.element : null}
}
