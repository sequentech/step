// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {type ReactNode} from "react"
import {Box, Divider, IconButton, Stack, Tooltip, Typography} from "@mui/material"
import DownloadIcon from "@mui/icons-material/Download"
import EditIcon from "@mui/icons-material/Edit"
import RefreshIcon from "@mui/icons-material/Refresh"
import {Button} from "react-admin"
import {useTranslation} from "react-i18next"
import {type MonitoringDashboardSummary, type MonitoringSnapshot} from "./types"
import {useTimeZoneService} from "@/components/timezones/timeZoneService"
import {formatDateTime} from "./lib/format"
import {MonitoringSwitcher} from "./MonitoringSwitcher"

export interface MonitoringHeaderProps {
    /** The preset the event's configuration came from; absent before a reset. */
    presetTitle?: string | null
    dashboards: MonitoringDashboardSummary[]
    dashboardId: string
    onSelectDashboard: (dashboardId: string) => void
    snapshot: MonitoringSnapshot | null
    timeZone: string
    /** How often the dashboard asks for new figures. */
    refreshMs: number
    /** Asks for new figures now. */
    onRefresh?: () => void
    /** Absent while there is no snapshot to export. */
    onExport?: () => void
    /** Given only to viewers who may configure. */
    onEditDashboard?: () => void
    /** The dashboard selectors, shown beside the section. */
    children?: ReactNode
}

/**
 * {preset} Dashboard preset · Export · Edit dashboard
 * Section ▾ · Region ▾ · Post ▾ · Country ▾ · ● Updated {time} ({zone}) · every {n} s ↻
 */
export function MonitoringHeader({
    presetTitle,
    dashboards,
    dashboardId,
    onSelectDashboard,
    snapshot,
    timeZone,
    refreshMs,
    onRefresh,
    onExport,
    onEditDashboard,
    children,
}: MonitoringHeaderProps) {
    const {t, i18n} = useTranslation()
    const service = useTimeZoneService()
    const status = [
        snapshot
            ? t("monitoring.header.updated", {
                  time: formatDateTime(snapshot.as_of, timeZone, i18n.language, {
                      timeStyle: "short",
                  }),
                  // The event's zone, which is not necessarily the viewer's.
                  timeZone: service.zoneLabel(timeZone, service.text, new Date(snapshot.as_of)),
              })
            : t("monitoring.header.notUpdated"),
        t("monitoring.header.refresh", {seconds: Math.round(refreshMs / 1000)}),
    ].join(" · ")
    return (
        <Stack spacing={2}>
            <Stack direction="row" alignItems="center" spacing={1.5} useFlexGap flexWrap="wrap">
                {presetTitle ? (
                    <>
                        <Typography
                            variant="h6"
                            component="p"
                            sx={{color: "brandColor", fontWeight: 600}}
                        >
                            {presetTitle}
                        </Typography>
                        <Typography variant="caption" color="text.secondary">
                            {t("monitoring.header.preset")}
                        </Typography>
                    </>
                ) : null}
                <Box sx={{flexGrow: 1}} />
                {/* The portal's top action buttons: outlined, joined, as in ListActions. */}
                <div className="list-actions">
                    <Button
                        onClick={onExport}
                        disabled={!onExport}
                        label={t("monitoring.header.export")}
                    >
                        <DownloadIcon />
                    </Button>
                    {onEditDashboard ? (
                        <Button
                            onClick={onEditDashboard}
                            label={t("monitoring.header.editDashboard")}
                        >
                            <EditIcon />
                        </Button>
                    ) : null}
                </div>
            </Stack>
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
                {children}
                <Box sx={{flexGrow: 1}} />
                <Stack direction="row" alignItems="center" spacing={0.5}>
                    <Box
                        aria-hidden
                        sx={{
                            width: 8,
                            height: 8,
                            borderRadius: "50%",
                            bgcolor: snapshot ? "success.main" : "text.disabled",
                        }}
                    />
                    <Typography
                        variant="caption"
                        sx={{color: snapshot ? "success.dark" : "text.secondary"}}
                    >
                        {status}
                    </Typography>
                    {onRefresh ? (
                        <Tooltip title={t("monitoring.header.reload")}>
                            <IconButton
                                size="small"
                                aria-label={t("monitoring.header.reload")}
                                onClick={onRefresh}
                            >
                                <RefreshIcon fontSize="small" />
                            </IconButton>
                        </Tooltip>
                    ) : null}
                </Stack>
            </Stack>
            <Divider />
        </Stack>
    )
}
