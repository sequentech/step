// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {useContext} from "react"
import {AuthContext} from "@/providers/AuthContextProvider"
import {IPermissions} from "@/types/keycloak"
import {EMonitoringCapability} from "./types"

export enum EMonitoringLock {
    OPEN = "OPEN",
    LOCKED_DOWN = "LOCKED_DOWN",
}

export interface MonitoringPermissions {
    view: EMonitoringCapability
    /** Changing dashboards, widgets and themes: also needs event edit and an open event. */
    configure: EMonitoringCapability
}

const capability = (granted: boolean) =>
    granted ? EMonitoringCapability.GRANTED : EMonitoringCapability.DENIED

export function useMonitoringPermissions(
    lock: EMonitoringLock = EMonitoringLock.OPEN
): MonitoringPermissions {
    const auth = useContext(AuthContext)
    const has = (permission: IPermissions) => auth.isAuthorized(true, auth.tenantId, permission)
    const view = has(IPermissions.MONITORING_VIEW)
    return {
        view: capability(view),
        configure: capability(
            view &&
                has(IPermissions.MONITORING_CONFIGURE) &&
                has(IPermissions.ELECTION_EVENT_WRITE) &&
                lock === EMonitoringLock.OPEN
        ),
    }
}
