// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {Box, Typography} from "@mui/material"
import {useTranslation} from "react-i18next"
import {ElectoralLogList} from "@/components/ElectoralLogList"
import {ELogScope} from "@/components/logs/logScope"
import {ResourceListStyles} from "@/components/styles/ResourceListStyles"
import {useLogsPermissions} from "@/resources/ElectionEvent/useLogsPermissions"

/**
 * Settings > Logs: the tenant log, laid out like an election event's Logs
 * tab. It holds what happens outside election events: sign-ins, staff
 * accounts and roles, settings and the events' own lifecycle.
 */
export const SettingsLogs: React.FC = () => {
    const {t} = useTranslation()
    const {canReadTenantLogs} = useLogsPermissions()

    if (!canReadTenantLogs) {
        return (
            <ResourceListStyles.EmptyBox>
                <Typography variant="h4" paragraph>
                    {t("logsScreen.tenant.noPermissions")}
                </Typography>
            </ResourceListStyles.EmptyBox>
        )
    }

    return (
        <Box className="settings-logs" sx={{minWidth: 0}}>
            <Typography component="h2" variant="h5" sx={{fontWeight: 700}}>
                {t("logsScreen.title")}
            </Typography>
            <Typography variant="body2" sx={{color: "text.secondary", mb: 2}}>
                {t("logsScreen.tenant.subtitle")}
            </Typography>
            <ElectoralLogList scope={ELogScope.TENANT} />
        </Box>
    )
}
