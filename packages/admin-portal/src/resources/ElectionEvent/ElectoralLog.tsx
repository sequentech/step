// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
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
import {ElectoralLogList} from "@/components/ElectoralLogList"

export const ElectoralLog: React.FC = () => {
    const authContext = useContext(AuthContext)
    const [tenantId] = useTenantStore()
    const [tab, setTab] = useState(0)
    const {t} = useTranslation()

    const logsRead = authContext.isAuthorized(true, tenantId, IPermissions.LOGS_READ)
    const handleChange = (event: React.SyntheticEvent, newValue: number) => {
        setTab(newValue)
    }

    if (!logsRead) {
        return (
            <ResourceListStyles.EmptyBox>
                <Typography variant="h4" paragraph>
                    {t("logsScreen.noPermissions")}
                </Typography>
            </ResourceListStyles.EmptyBox>
        )
    }

    return (
        <>
            <ElectionHeader title={t("logsScreen.title")} subtitle="logsScreen.subtitle" />
            <ElectoralLogList />
        </>
    )
}
