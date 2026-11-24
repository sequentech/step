// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {useContext, useLayoutEffect} from "react"
import {AuthContext} from "@/providers/AuthContextProvider"
import {useTenantStore} from "@/providers/TenantContextProvider"

const eventName = "sequent:configuration-package-election-imported"

interface ImportScope {
    tenantId: string
    actor: string
}

export function notifyConfigurationPackageElectionImported(scope: ImportScope): void {
    window.dispatchEvent(new CustomEvent<ImportScope>(eventName, {detail: scope}))
}

/** Only the current operator's election queries should refresh. */
export function useConfigurationPackageElectionRefresh(
    refetch: () => Promise<unknown> | void
): void {
    const [tenantId] = useTenantStore()
    const {userId} = useContext(AuthContext)
    useLayoutEffect(() => {
        if (!tenantId || !userId) return
        const onImported = (event: Event) => {
            const scope = (event as CustomEvent<ImportScope>).detail
            if (scope?.tenantId === tenantId && scope.actor === userId) {
                void refetch()?.catch(() => undefined)
            }
        }
        window.addEventListener(eventName, onImported)
        return () => window.removeEventListener(eventName, onImported)
    }, [tenantId, userId, refetch])
}
