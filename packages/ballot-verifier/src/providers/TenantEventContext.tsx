// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"

interface TenantEventContextValues {
    tenantId: string | null
    eventId: string | null
}

export const TenantEventContext = React.createContext<TenantEventContextValues>({
    tenantId: null,
    eventId: null,
})

// This component will be used to provide tenantId and eventId to the context
export const TenantEventProvider: React.FC<{
    tenantId: string | null
    eventId: string | null
    children: React.ReactNode
}> = ({tenantId, eventId, children}) => {
    console.log(`TenantEventProvider: tenantId=${tenantId}, eventId=${eventId}`)
    return (
        <TenantEventContext.Provider value={{tenantId, eventId}}>
            {children}
        </TenantEventContext.Provider>
    )
}
