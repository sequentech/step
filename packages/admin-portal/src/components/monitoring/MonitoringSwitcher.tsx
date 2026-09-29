// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {MenuItem, TextField} from "@mui/material"
import {useTranslation} from "react-i18next"
import type {MonitoringDashboardSummary} from "./types"

export interface MonitoringSwitcherProps {
    dashboards: MonitoringDashboardSummary[]
    dashboardId: string
    onChange: (dashboardId: string) => void
}

/** The dashboard ▾ menu: every dashboard of the event, in the configured order. */
export function MonitoringSwitcher({dashboards, dashboardId, onChange}: MonitoringSwitcherProps) {
    const {t} = useTranslation()
    return (
        <TextField
            id="monitoring-dashboard"
            select
            size="small"
            label={t("monitoring.header.dashboard")}
            value={dashboardId}
            onChange={(event) => onChange(event.target.value)}
            sx={{minWidth: 220, maxWidth: "100%"}}
        >
            {dashboards.map((dashboard) => (
                <MenuItem key={dashboard.id} value={dashboard.id}>
                    {dashboard.title}
                </MenuItem>
            ))}
        </TextField>
    )
}
