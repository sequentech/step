// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import Tab from "@mui/material/Tab"
import React, {useContext, useState} from "react"
import {ListUsers} from "../resources/User/ListUsers"
import {AuthContext} from "../providers/AuthContextProvider"
import {useTenantStore} from "../providers/TenantContextProvider"
import ElectionHeader from "../components/ElectionHeader"
import {useTranslation} from "react-i18next"
import {ListRoles} from "../resources/Roles/ListRoles"
import {IPermissions} from "../types/keycloak"
import {SidebarScreenStyles} from "@/components/styles/SidebarScreenStyles"
import {ResourceListStyles} from "@/components/styles/ResourceListStyles"
import {Typography} from "@mui/material"

export const UserAndRoles: React.FC = () => {
    const authContext = useContext(AuthContext)
    const [tenantId] = useTenantStore()
    const [value, setValue] = useState(0)
    const {t} = useTranslation()

    const showUsers = authContext.isAuthorized(true, tenantId, IPermissions.USER_READ)
    const showRoles = authContext.isAuthorized(true, tenantId, IPermissions.ROLE_READ)
    const handleChange = (event: React.SyntheticEvent, newValue: number) => {
        setValue(newValue)
    }

    const showUsersMenu = authContext.isAuthorized(true, tenantId, IPermissions.USERS_MENU)

    if ((!showUsers && !showRoles) || !showUsersMenu) {
        return (
            <ResourceListStyles.EmptyBox>
                <Typography variant="h4" paragraph>
                    {t("usersAndRolesScreen.noPermissions")}
                </Typography>
            </ResourceListStyles.EmptyBox>
        )
    }

    return (
        <>
            <ElectionHeader
                title="usersAndRolesScreen.common.title"
                subtitle="usersAndRolesScreen.common.subtitle"
            />
            <SidebarScreenStyles.Tabs
                value={value}
                onChange={handleChange}
                aria-label="Users and Roles tabs"
            >
                {showUsers ? <Tab label={String(t("usersAndRolesScreen.users.title"))} /> : null}
                {showRoles ? <Tab label={String(t("usersAndRolesScreen.roles.title"))} /> : null}
            </SidebarScreenStyles.Tabs>
            <SidebarScreenStyles.CustomTabPanel value={value} index={0}>
                <ListUsers />
            </SidebarScreenStyles.CustomTabPanel>
            <SidebarScreenStyles.CustomTabPanel value={value} index={1}>
                <ListRoles />
            </SidebarScreenStyles.CustomTabPanel>
        </>
    )
}
