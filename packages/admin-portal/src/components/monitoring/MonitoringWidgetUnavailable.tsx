// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {Box, Stack, Typography} from "@mui/material"
import {useTranslation} from "react-i18next"
import {
    EColumnKind,
    EWidgetFailure,
    EWidgetRenderState,
    POSITION_COLUMN,
    type MonitoringProblem,
    type MonitoringTable,
    type MonitoringUnavailableState,
} from "./types"
import {MonitoringDataTable} from "./MonitoringDataTable"
import {MonitoringKpi} from "./MonitoringKpi"

export interface MonitoringWidgetUnavailableProps {
    state: MonitoringUnavailableState
    /** A NOT_CONNECTED producer, as the server names it. */
    reason?: string | null
    /** Why an INVALID definition cannot be shown. */
    problem?: string
    diagnostics?: MonitoringProblem[]
    /** RENDER_FAILED still shows the figures, as a table. */
    table?: MonitoringTable | null
    title: string
}

const WORDS: Record<MonitoringUnavailableState, {title: string; help: string}> = {
    [EWidgetRenderState.NOT_CONNECTED]: {
        title: "monitoring.unavailable.notConnected",
        help: "monitoring.unavailable.notConnectedHelp",
    },
    [EWidgetRenderState.NO_SNAPSHOT]: {
        title: "monitoring.unavailable.noSnapshot",
        help: "monitoring.unavailable.noSnapshotHelp",
    },
    [EWidgetRenderState.SCOPE_PENDING]: {
        title: "monitoring.unavailable.scopePending",
        help: "monitoring.unavailable.scopePendingHelp",
    },
    [EWidgetRenderState.RENDER_FAILED]: {
        title: "monitoring.unavailable.renderFailed",
        help: "monitoring.unavailable.renderFailedHelp",
    },
    [EWidgetRenderState.INVALID]: {
        title: "monitoring.unavailable.invalid",
        help: "monitoring.unavailable.invalidHelp",
    },
    [EWidgetFailure.REQUEST_FAILED]: {
        title: "monitoring.unavailable.requestFailed",
        help: "monitoring.unavailable.requestFailedHelp",
    },
}

/** A table of one row reads better as figures. */
function Fallback({table, title}: {table: MonitoringTable; title: string}) {
    const figures = table.columns
        .map((column, index) => ({column, value: table.rows[0]?.[index]}))
        .filter(({column}) => column.name !== POSITION_COLUMN && column.kind !== EColumnKind.TEXT)
    if (table.rows.length === 1 && figures.length) {
        return (
            <Stack direction="row" flexWrap="wrap" useFlexGap spacing={2}>
                {figures.map(({column, value}) => (
                    <MonitoringKpi
                        key={column.name}
                        label={column.name}
                        value={value}
                        kind={column.kind}
                    />
                ))}
            </Stack>
        )
    }
    return <MonitoringDataTable table={table} caption={title} />
}

/** Why a widget shows no chart: never a zero standing in for a figure nobody counted. */
export function MonitoringWidgetUnavailable({
    state,
    reason,
    problem,
    diagnostics = [],
    table,
    title,
}: MonitoringWidgetUnavailableProps) {
    const {t} = useTranslation()
    const words = WORDS[state]
    const reasonText = reason ? t(`monitoring.reasons.${reason}`, {defaultValue: reason}) : ""
    const errors = diagnostics.map((diagnostic) => diagnostic.message)
    return (
        <Stack spacing={1} role="status" sx={{py: 2}}>
            <Typography variant="subtitle1" component="p">
                {t(words.title, {reason: reasonText || t("monitoring.unavailable.unknownReason")})}
            </Typography>
            <Typography variant="body2" color="text.secondary">
                {t(words.help)}
            </Typography>
            {problem || errors.length ? (
                <Box component="ul" sx={{m: 0, pl: 3, typography: "body2"}}>
                    {[problem, ...errors].filter(Boolean).map((message) => (
                        <li key={message}>{message}</li>
                    ))}
                </Box>
            ) : null}
            {state === EWidgetRenderState.RENDER_FAILED && table ? (
                <Fallback table={table} title={title} />
            ) : null}
        </Stack>
    )
}
