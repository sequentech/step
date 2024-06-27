// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext, useEffect} from "react"
import {useGetOne, useRefresh, useSidebarState} from "react-admin"
import {faThLarge, faPlusCircle} from "@fortawesome/free-solid-svg-icons"
import {IconButton, adminTheme} from "@sequentech/ui-essentials"
import {Box, MenuItem, Select, SelectChangeEvent} from "@mui/material"
import {Link} from "react-router-dom"
import {AuthContext} from "../../../providers/AuthContextProvider"
import {useTenantStore} from "../../../providers/TenantContextProvider"
import {IPermissions} from "../../../types/keycloak"
import AccountCircleIcon from "@mui/icons-material/AccountCircle"
import {useTranslation} from "react-i18next"
import styled from "@emotion/styled"

const SelectTenants: React.FC = () => {
    const refresh = useRefresh()
    const [tenantId, setTenantId] = useTenantStore()
    const authContext = useContext(AuthContext)
    const {i18n} = useTranslation()
    const [isOpenSidebar] = useSidebarState()

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
        <Container hasSingle={true}>
            <AccountCircleIcon />
            {isOpenSidebar && !!data && (
                <>
                    <SingleDataContainer
                        style={{
                            textAlign: i18n.dir(i18n.language) === "rtl" ? "start" : "start",
                        }}
                    >
                        {data.slug}
                    </SingleDataContainer>
                    {showAddTenant ? (
                        <Link to="/sequent_backend_tenant/create">
                            <StyledIcon icon={faPlusCircle} />
                        </Link>
                    ) : null}
                </>
            )}
        </Container>
    )
}

export default SelectTenants

const Container = styled(Box)<{hasSingle: boolean}>`
    display: flex;
    align-items: center;
    padding-left: 1rem;
    padding-right: 1rem;
    & > *:not(:last-child) {
        margin-right: 1rem;
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
