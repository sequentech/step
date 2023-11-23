// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {createContext, useContext, useState} from "react"
import { DEFAULT_TENANT } from "./AuthContextProvider"

interface TenantContext {
    tenantId: string | null
    setTenantId: (tenantId: string | null) => void
}

const defaultTenantContext: TenantContext = {
    tenantId: DEFAULT_TENANT,
    setTenantId: () => undefined,
}

export const TenantContext = createContext<TenantContext>(defaultTenantContext)

interface TenantContextProviderProps {
    /**
     * The elements wrapped by the tenant context.
     */
    children: JSX.Element
}

export const TenantContextProvider = (props: TenantContextProviderProps) => {
    const [tenant, setTenant] = useState<string | null>(localStorage.getItem("selected-tenant-id") || DEFAULT_TENANT)

    const setTenantId = (tenantId: string | null): void => {
        localStorage.setItem("selected-tenant-id", tenantId || "")
        setTenant(tenantId)
    }

    // Setup the context provider
    return (
        <TenantContext.Provider
            value={{
                tenantId: tenant,
                setTenantId,
            }}
        >
            {props.children}
        </TenantContext.Provider>
    )
}

export const useTenantStore: () => [string | null, (tenantId: string | null) => void] = () => {
    const {tenantId, setTenantId} = useContext(TenantContext)

    return [tenantId, setTenantId]
}
