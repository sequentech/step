// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {Button, Stack, Typography} from "@mui/material"
import {useTranslation} from "react-i18next"
import {type MonitoringDashboardSummary, type MonitoringSnapshot} from "./types"
import {formatDateTime} from "./lib/format"
import {MonitoringSwitcher} from "./MonitoringSwitcher"

export interface MonitoringHeaderProps {
    /** The dashboard's title: the heading its widgets' titles sit under. */
    title: string
    dashboards: MonitoringDashboardSummary[]
    dashboardId: string
    onSelectDashboard: (dashboardId: string) => void
    widgetCount: number
    requirements: string[]
    snapshot: MonitoringSnapshot | null
    timeZone: string
    /** How often the dashboard asks for new figures. */
    refreshMs: number
    /** Absent while there is no snapshot to export. */
    onExport?: () => void
    /** Given only to viewers who may configure. */
    onEditDashboard?: () => void
}

/** {dashboard} ▾ · {n} widgets · {requirement IDs} · Updated {time} ({zone}) · every {n} s · Export · Edit dashboard */
export function MonitoringHeader({
    title,
    dashboards,
    dashboardId,
    onSelectDashboard,
    widgetCount,
    requirements,
    snapshot,
    timeZone,
    refreshMs,
    onExport,
    onEditDashboard,
}: MonitoringHeaderProps) {
    const {t, i18n} = useTranslation()
    const facts = [
        t("monitoring.header.widgets", {count: widgetCount}),
        requirements.join(", "),
        snapshot
            ? t("monitoring.header.updated", {
                  time: formatDateTime(snapshot.as_of, timeZone, i18n.language),
                  // The event's zone, which is not necessarily the viewer's.
                  timeZone,
              })
            : t("monitoring.header.notUpdated"),
        t("monitoring.header.refresh", {seconds: Math.round(refreshMs / 1000)}),
    ].filter(Boolean)
    return (
        <Stack spacing={1}>
            <Typography variant="h5" component="h2">
                {title}
            </Typography>
            <Stack
                direction={{xs: "column", md: "row"}}
                spacing={2}
                alignItems={{xs: "stretch", md: "center"}}
                useFlexGap
                flexWrap="wrap"
            >
                <MonitoringSwitcher
                    dashboards={dashboards}
                    dashboardId={dashboardId}
                    onChange={onSelectDashboard}
                />
                <Typography variant="body2" color="text.secondary" sx={{flexGrow: 1}}>
                    {facts.join(" · ")}
                </Typography>
                <Stack direction="row" spacing={1}>
                    <Button variant="outlined" onClick={onExport} disabled={!onExport}>
                        {t("monitoring.header.export")}
                    </Button>
                    {onEditDashboard ? (
                        <Button variant="contained" onClick={onEditDashboard}>
                            {t("monitoring.header.editDashboard")}
                        </Button>
                    ) : null}
                </Stack>
            </Stack>
        </Stack>
    )
}
