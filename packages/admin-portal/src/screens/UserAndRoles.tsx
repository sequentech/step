// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Box} from "@mui/material"
import Tabs from "@mui/material/Tabs"
import Tab from "@mui/material/Tab"
import React, {useContext, useState} from "react"
import {ListUsers} from "../resources/User/ListUsers"
import {AuthContext} from "../providers/AuthContextProvider"
import {IPermissions} from "sequent-core"
import {useTenantStore} from "../providers/TenantContextProvider"
import {TabbedShowLayout} from "react-admin"
import {CustomTabPanel} from "../components/CustomTabPanel"
import ElectionHeader from "../components/ElectionHeader"
import {useTranslation} from "react-i18next"

export const UserAndRoles: React.FC = () => {
    const authContext = useContext(AuthContext)
    const [tenantId] = useTenantStore()
    const [value, setValue] = useState(0)
    const {t} = useTranslation()

    const showUsers = authContext.isAuthorized(true, tenantId, "user-read")
    const handleChange = (event: React.SyntheticEvent, newValue: number) => {
        setValue(newValue)
    }

    return (
        <>
            <ElectionHeader
                title={t("usersAndRolesScreen.common.title")}
                subtitle="usersAndRolesScreen.common.subtitle"
            />
            <Tabs value={value} onChange={handleChange} aria-label="Users and Roles tabs">
                <Tab label={t("usersAndRolesScreen.users.title")} />
                <Tab label={t("usersAndRolesScreen.roles.title")} />
            </Tabs>
            <CustomTabPanel value={value} index={0}>
                <ListUsers />
            </CustomTabPanel>
            <CustomTabPanel value={value} index={1}>
                hey
            </CustomTabPanel>
        </>
    )
}
