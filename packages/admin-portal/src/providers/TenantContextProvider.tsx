// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {createContext, useContext, useEffect, useState} from "react"
import {Sequent_Backend_Tenant} from "@/gql/graphql"
import {ITenantSettings} from "@sequentech/ui-core"
import {triggerOverrideTranslations} from "@/services/i18n"

interface TenantContextProps {
    tenantId: string | null
    setTenantId: (tenantId: string | null) => void
    tenant?: Sequent_Backend_Tenant
    setTenant: (tenant: Sequent_Backend_Tenant | undefined) => void
}

const defaultTenantContext: TenantContextProps = {
    tenantId: "",
    setTenantId: () => undefined,
    tenant: undefined,
    setTenant: () => undefined,
}

export const TenantContext = createContext<TenantContextProps>(defaultTenantContext)

interface TenantContextProviderProps {
    /**
     * The elements wrapped by the tenant context.
     */
    children: JSX.Element
}

export const TenantContextProvider = (props: TenantContextProviderProps) => {
    const [tenantId, setTenantId] = useState<string | null>(
        localStorage.getItem("selected-tenant-id") || null
    )

    const setTenantIdWrapper = (tenantId: string | null): void => {
        if (null === tenantId) {
            localStorage.removeItem("selected-tenant-id")
        } else {
            localStorage.setItem("selected-tenant-id", tenantId)
        }
        setTenantId(tenantId)
    }
    const [tenant, setTenant] = useState<Sequent_Backend_Tenant | undefined>(undefined)

    // Overwrites translations based on the settings config
    useEffect(() => {
        const i18nSettings = (tenant?.settings as ITenantSettings | undefined)?.i18n

        if (i18nSettings) {
            triggerOverrideTranslations(i18nSettings)
        }
    }, [tenant?.settings?.i18n])

    const setTenantWrapper = (newTenant: Sequent_Backend_Tenant | undefined) => {
        setTenant(newTenant)
        if (newTenant?.id && newTenant.id !== tenantId) {
            setTenantId(newTenant.id)
        }
    }

    // Setup the context provider
    return (
        <TenantContext.Provider
            value={{
                tenantId: tenantId,
                setTenantId: setTenantIdWrapper,
                tenant,
                setTenant: setTenantWrapper,
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
