// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {useContext} from "react"
import {EElectionEventLockedDown} from "@sequentech/ui-core"
import type {Sequent_Backend_Election_Event} from "@/gql/graphql"
import {AuthContext} from "@/providers/AuthContextProvider"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {IPermissions} from "@/types/keycloak"

export enum EEventDataMode {
    LOADING = "loading",
    UNAVAILABLE = "unavailable",
    EDIT = "edit",
    LOCKED_DOWN = "locked-down",
}

/** Shared by the tab selector and its content so links cannot bypass the gates. */
export function useElectionEventDataMode(event?: Sequent_Backend_Election_Event): EEventDataMode {
    const auth = useContext(AuthContext)
    const [tenantId] = useTenantStore()
    if (!event) return EEventDataMode.LOADING
    if (
        !tenantId ||
        event.tenant_id !== tenantId ||
        !auth.isAuthorized(true, tenantId, IPermissions.ELECTION_EVENT_DATA_TAB)
    )
        return EEventDataMode.UNAVAILABLE

    const lockdown = event.presentation?.locked_down
    if (lockdown === EElectionEventLockedDown.LOCKED_DOWN) {
        return auth.isAuthorized(true, tenantId, IPermissions.READINESS_CHECK)
            ? EEventDataMode.LOCKED_DOWN
            : EEventDataMode.UNAVAILABLE
    }
    // Older events have no lockdown field. Unknown states must not open the editor.
    return lockdown == null || lockdown === EElectionEventLockedDown.NOT_LOCKED_DOWN
        ? EEventDataMode.EDIT
        : EEventDataMode.UNAVAILABLE
}
