// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import Tabs from "@mui/material/Tabs"
import Tab from "@mui/material/Tab"

import {Typography} from "@mui/material"
import React, {useContext, useState} from "react"
import {AuthContext} from "@/providers/AuthContextProvider"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {CustomTabPanel} from "@/components/CustomTabPanel"
import ElectionHeader from "@/components/ElectionHeader"
import {useTranslation} from "react-i18next"
import {IPermissions} from "@/types/keycloak"
import {ResourceListStyles} from "@/components/styles/ResourceListStyles"
import {PgAuditList} from "@/resources/PgAudit/PgAuditList"
import {PgAuditTable} from "@/gql/graphql"
import {SidebarScreenStyles} from "@/components/styles/SidebarScreenStyles"

export const Logs: React.FC = () => {
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
            <ElectionHeader title={String(t("logsScreen.title"))} subtitle="logsScreen.subtitle" />
            <SidebarScreenStyles.Tabs value={tab} onChange={handleChange} aria-label="Log tabs">
                <Tab label={String(t("logsScreen.main.title"))} />
                <Tab label={String(t("logsScreen.iam.title"))} />
            </SidebarScreenStyles.Tabs>
            <SidebarScreenStyles.CustomTabPanel value={tab} index={0}>
                <PgAuditList auditTable={PgAuditTable.PgauditHasura} />
            </SidebarScreenStyles.CustomTabPanel>
            <SidebarScreenStyles.CustomTabPanel value={tab} index={1}>
                <PgAuditList auditTable={PgAuditTable.PgauditKeycloak} />
            </SidebarScreenStyles.CustomTabPanel>
        </>
    )
}
