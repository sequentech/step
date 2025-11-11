// SPDX-FileCopyrightText: 2023-2025 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext, useEffect, useState} from "react"
import {useGetOne, useSidebarState} from "react-admin"
import {faPlusCircle} from "@fortawesome/free-solid-svg-icons"
import {IconButton} from "@sequentech/ui-essentials"
import {Box, Select} from "@mui/material"
import {AuthContext} from "../../../providers/AuthContextProvider"
import {useTenantStore} from "../../../providers/TenantContextProvider"
import {IPermissions} from "../../../types/keycloak"
import AccountCircleIcon from "@mui/icons-material/AccountCircle"
import {useTranslation} from "react-i18next"
import styled from "@emotion/styled"
import {CreateTenant} from "@/resources/Tenant/CreateTenant"

const SelectTenants: React.FC = () => {
    const [tenantId, setTenantId] = useTenantStore()
    const authContext = useContext(AuthContext)
    const {i18n} = useTranslation()
    const [isOpenSidebar] = useSidebarState()
    const [isNewTenantOpen, setIsNewTenantOpen] = useState(false)

    const showAddTenant = authContext.isAuthorized(true, null, IPermissions.TENANT_CREATE)

    const {data} = useGetOne("sequent_backend_tenant", {
        id: tenantId,
    })

    useEffect(() => {
        if (!tenantId && authContext.tenantId) {
            setTenantId(authContext.tenantId)
        }
        if (data?.length === 1) {
            setTenantId(data[0].id)
        }
    }, [data, tenantId, authContext.tenantId, setTenantId])

    return (
        <Container className="select-tenants" hasSingle={true}>
            <AccountCircleIcon />
            {isOpenSidebar && !!data && (
                <>
                    <SingleDataContainer
                        className="tenant-name"
                        style={{
                            textAlign: i18n.dir(i18n.language) === "rtl" ? "start" : "start",
                            marginLeft: 0,
                        }}
                    >
                        {data.slug}
                    </SingleDataContainer>
                    {showAddTenant ? (
                        <StyledIcon
                            icon={faPlusCircle as any}
                            onClick={() => setIsNewTenantOpen(true)}
                        />
                    ) : null}
                </>
            )}
            {isNewTenantOpen && (
                <CreateTenant isDrawerOpen={isNewTenantOpen} setIsDrawerOpen={setIsNewTenantOpen} />
            )}
        </Container>
    )
}

export default SelectTenants

const Container = styled(Box, {
    shouldForwardProp: (prop) => prop !== "hasSingle", // Prevent `hasSingle` from being passed to the DOM
})<{hasSingle: boolean}>`
    display: flex;
    align-items: center;
    padding-left: 16px;
    padding-right: 16px;
    & > *:not(:last-child) {
        margin-right: 12px;
    }
    padding-top: ${({hasSingle}) => (hasSingle ? "0.375rem" : "0.25rem")};
    padding-bottom: ${({hasSingle}) => (hasSingle ? "0.375rem" : "0.25rem")};
`

const SingleDataContainer = styled("p")`
    flex-grow: 1;
    margin-left: 0.625rem;
`

const StyledIcon = styled(IconButton)`
    &:hover {
        padding: unset !important;
    }
    font-size: 1rem;
    line-height: 1.5rem;
`
const StyledSelect = styled(Select)`
    flex-grow: 1;
    margin-left: 0;
    margin-right: 0;
    margin-top: 0 !important;
    margin-bottom: 0 !important;
`
