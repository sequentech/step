// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {ListSubheader, MenuItem, TextField} from "@mui/material"
import {useTranslation} from "react-i18next"
import type {MonitoringDashboardSummary} from "./types"

/**
 * A section's heading in the menu. The select hands every child the props
 * of an option; a heading keeps none of them, so it is neither announced
 * nor chosen, and the keyboard skips it as it does a `ListSubheader`.
 */
function SectionHeading({children}: {children: React.ReactNode}) {
    return (
        <ListSubheader role="presentation" sx={{lineHeight: "32px"}}>
            {children}
        </ListSubheader>
    )
}
SectionHeading.muiSkipListHighlight = true

export interface MonitoringSwitcherProps {
    dashboards: MonitoringDashboardSummary[]
    dashboardId: string
    onChange: (dashboardId: string) => void
}

/**
 * The dashboard ▾ menu: every dashboard of the event, in the configured
 * order, each section's under its heading.
 */
export function MonitoringSwitcher({dashboards, dashboardId, onChange}: MonitoringSwitcherProps) {
    const {t} = useTranslation()
    const items: React.ReactNode[] = []
    let section: string | null | undefined
    for (const dashboard of dashboards) {
        const next = dashboard.section ?? null
        if (next && next !== section) {
            items.push(<SectionHeading key={`section:${next}`}>{next}</SectionHeading>)
        }
        section = next
        items.push(
            <MenuItem key={dashboard.id} value={dashboard.id} sx={next ? {pl: 3} : undefined}>
                {dashboard.title}
            </MenuItem>
        )
    }
    return (
        <TextField
            id="monitoring-dashboard"
            select
            size="small"
            label={t("monitoring.header.dashboard")}
            value={dashboardId}
            onChange={(event) => onChange(event.target.value)}
            fullWidth={false}
            sx={{width: {xs: "100%", md: 260}}}
        >
            {items}
        </TextField>
    )
}
