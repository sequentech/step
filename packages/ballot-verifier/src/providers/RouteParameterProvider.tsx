// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {useParams} from "react-router-dom"
import {TenantEventProvider} from "./TenantEventContext"

export const RouteParameterProvider: React.FC<{children: React.ReactNode}> = ({children}) => {
    const {tenantId, eventId} = useParams<{tenantId: string; eventId: string}>()
    console.log(`RouteParameterProvider: tenantId=${tenantId}, eventId=${eventId}`)

    return (
        <TenantEventProvider
            tenantId={tenantId ? tenantId : null}
            eventId={eventId ? eventId : null}
        >
            {children}
        </TenantEventProvider>
    )
}
