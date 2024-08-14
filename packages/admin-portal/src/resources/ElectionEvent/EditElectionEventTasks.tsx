// SPDX-FileCopyrightText: 2024 Eduardo Robles <dev@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {Typography} from "@mui/material"
import React, {useContext, useState} from "react"
import {AuthContext} from "@/providers/AuthContextProvider"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {CustomTabPanel} from "@/components/CustomTabPanel"
import ElectionHeader from "@/components/ElectionHeader"
import {useTranslation} from "react-i18next"
import {IPermissions} from "@/types/keycloak"
import {ResourceListStyles} from "@/components/styles/ResourceListStyles"
import {ListTasks} from "../Tasks/ListTasks"

export const EditElectionEventTasks: React.FC = () => {
    const authContext = useContext(AuthContext)
    const [tenantId] = useTenantStore()
    const [tab, setTab] = useState(0)
    const {t} = useTranslation()

    // const logsRead = authContext.isAuthorized(true, tenantId, IPermissions.TASKS_READ)
    const handleChange = (event: React.SyntheticEvent, newValue: number) => {
        setTab(newValue)
    }


    return (
        <>
            <ElectionHeader title={t("tasksScreen.title")} subtitle="tasksScreen.subtitle" />
            <ListTasks />
        </>
    )
}
