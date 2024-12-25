// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {Typography} from "@mui/material"
import React, {useContext, useState} from "react"
import ElectionHeader from "@/components/ElectionHeader"
import {useTranslation} from "react-i18next"
import {ResourceListStyles} from "@/components/styles/ResourceListStyles"
import {ElectoralLogList} from "@/components/ElectoralLogList"
import {useLogsPermissions} from "./useLogsPermissions"

export const ElectoralLog: React.FC = () => {
    const [tab, setTab] = useState(0)
    const {t} = useTranslation()

    const {canReadLogs} = useLogsPermissions()

    const handleChange = (event: React.SyntheticEvent, newValue: number) => {
        setTab(newValue)
    }

    if (!canReadLogs) {
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
