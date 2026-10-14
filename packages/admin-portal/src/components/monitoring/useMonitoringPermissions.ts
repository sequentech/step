// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {useContext} from "react"
import {EElectionEventLockedDown} from "@sequentech/ui-core"
import {AuthContext} from "@/providers/AuthContextProvider"
import {IPermissions} from "@/types/keycloak"
import {EMonitoringCapability} from "./types"

export enum EMonitoringLock {
    OPEN = "OPEN",
    LOCKED_DOWN = "LOCKED_DOWN",
    /** The event has not loaded yet: nothing offers a change until it has. */
    UNKNOWN = "UNKNOWN",
}

/** The lock of an election event, or UNKNOWN while it is still loading. */
export function monitoringLock(
    event: {presentation?: {locked_down?: unknown} | null} | null | undefined
): EMonitoringLock {
    if (!event) return EMonitoringLock.UNKNOWN
    return event.presentation?.locked_down === EElectionEventLockedDown.LOCKED_DOWN
        ? EMonitoringLock.LOCKED_DOWN
        : EMonitoringLock.OPEN
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
