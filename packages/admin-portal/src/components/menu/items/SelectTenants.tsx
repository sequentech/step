// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext, useEffect} from "react"
import {useGetList, useRefresh, useSidebarState} from "react-admin"
import {faThLarge, faPlusCircle} from "@fortawesome/free-solid-svg-icons"
import {IconButton} from "@sequentech/ui-essentials"
import {MenuItem, Select, SelectChangeEvent} from "@mui/material"
import {Link} from "react-router-dom"
import {cn} from "../../../lib/utils"
import {AuthContext} from "../../../providers/AuthContextProvider"
import {useTenantStore} from "../../../providers/TenantContextProvider"
import {IPermissions} from "../../../types/keycloak"
import AccountCircleIcon from "@mui/icons-material/AccountCircle"
import {useTranslation} from "react-i18next"
import styled from "@emotion/styled"
import { colors } from "@/constants/colors"

const SelectTenants: React.FC = () => {
    const refresh = useRefresh()
    const [tenantId, setTenantId] = useTenantStore()
    const authContext = useContext(AuthContext)
    const {i18n} = useTranslation()
    const [isOpenSidebar] = useSidebarState()

    const showAddTenant = authContext.isAuthorized(true, null, IPermissions.TENANT_CREATE)

    const {data, total} = useGetList("sequent_backend_tenant", {
        pagination: {page: 1, perPage: 10},
        sort: {field: "updated_at", order: "DESC"},
        filter: {is_active: true},
    })

    useEffect(() => {
        if (!tenantId && authContext.tenantId) {
            setTenantId(authContext.tenantId)
        }
        if (data?.length === 1) {
            setTenantId(data[0].id)
        }
    }, [data, tenantId, authContext.tenantId, setTenantId])

    const hasSingle = total === 1

    const handleChange = (event: SelectChangeEvent<unknown>) => {
        const tenantId: string = event.target.value as string
        setTenantId(tenantId)
        refresh()
    }



    return (
        <Container hasSingle={hasSingle}>
            <AccountCircleIcon />
            {isOpenSidebar && !!data && (
                <>
                    {hasSingle ? (
                        <SingleDataContainer
                            style={{
                                textAlign: i18n.dir(i18n.language) === "rtl" ? "start" : "start",
                            }}
                        >
                            {data[0].slug}
                        </SingleDataContainer>
                    ) : (
                        <Select
                            labelId="tenant-select-label"
                            id="tenant-select"
                            value={tenantId}
                            onChange={handleChange}
                            className="grow mx-0 !-my-0"
                        >
                            {data?.map((tenant) => (
                                <MenuItem key={tenant.id} value={tenant.id}>
                                    {tenant.slug}
                                </MenuItem>
                            ))}
                        </Select>
                    )}
                    {showAddTenant ? (
                        <Link to="/sequent_backend_tenant/create">
                            <IconButton
                                className="text-brand-color text-base"
                                icon={faPlusCircle}
                            />
                        </Link>
                    ) : null}
                </>
            )}
        </Container>
    )
}

export default SelectTenants

const Container = styled.div<{hasSingle: boolean}>`
display: flex;
align-items: center;
padding-left: 1rem;
padding-right: 1rem;
& > *:not(:last-child) {
  margin-right: 1rem;
}
padding-top: ${({ hasSingle }) => (hasSingle ? '0.375rem' : '0.25rem')};
padding-bottom: ${({ hasSingle }) => (hasSingle ? '0.375rem' : '0.25rem')};
`;

const SingleDataContainer = styled.p`
flex-grow: 1;
margin-left: 0.625rem;
`

const StyledIcon = styled(IconButton)`
color: ${colors.brandColor}
font-size: 1rem
`
