// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {Stack, Typography} from "@mui/material"
import {useTranslation} from "react-i18next"
import type {MonitoringSnapshot} from "./types"
import {formatDateTime} from "./lib/format"

export interface MonitoringFooterProps {
    /** What the figures are of: "All authorized Posts", "Dubai PCG". */
    scopeLabel: string
    snapshot: MonitoringSnapshot | null
    timeZone: string
    /** The monitoring requirements the dashboard answers. */
    requirements: string[]
}

/** {scope} · Data through {time} ({zone}) ··· {requirement IDs} */
export function MonitoringFooter({
    scopeLabel,
    snapshot,
    timeZone,
    requirements,
}: MonitoringFooterProps) {
    const {t, i18n} = useTranslation()
    const through = snapshot
        ? t("monitoring.footer.dataThrough", {
              time: formatDateTime(snapshot.as_of, timeZone, i18n.language),
              timeZone,
          })
        : t("monitoring.header.notUpdated")
    return (
        <Stack
            direction={{xs: "column", sm: "row"}}
            justifyContent="space-between"
            spacing={1}
            component="footer"
        >
            <Typography variant="caption" color="text.secondary">
                {[scopeLabel, through].filter(Boolean).join(" · ")}
            </Typography>
            {requirements.length ? (
                <Typography variant="caption" color="text.secondary">
                    {requirements.join(", ")}
                </Typography>
            ) : null}
        </Stack>
    )
}
