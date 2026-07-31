// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import Chart, {Props} from "react-apexcharts"
import CardChart from "./Charts"
import {CastVotesPerDay} from "@/gql/graphql"
import {useTranslation} from "react-i18next"
import {CircularProgress, MenuItem, Select, Stack, Tooltip} from "@mui/material"
import {toVotesPerDayChartData} from "./votesPerDayData"
import {getVotesPerDayChartOptions} from "./votesPerDayOptions"
import {
    getVotesTimeRangeOptions,
    VotesTimeRange,
    VotesTimeResolution,
    VotesTimeSelection,
    withVotesTimeResolution,
} from "./votesTimeRange"

export interface VotersPerDayProps {
    data: CastVotesPerDay[] | null
    width: number
    height: number
    selection: VotesTimeSelection
    onSelectionChange: (selection: VotesTimeSelection) => void
}

const compactSelectSx = {
    "height": 28,
    "fontSize": "0.75rem",
    "& .MuiSelect-select": {
        paddingTop: "3px",
        paddingBottom: "3px",
        paddingLeft: "8px",
        paddingRight: "24px !important",
    },
}

const compactMenuProps = {
    PaperProps: {
        sx: {
            "marginTop": "4px",
            "& .MuiMenuItem-root": {
                minHeight: 30,
                fontSize: "0.75rem",
            },
        },
    },
}

export const VotesPerDay: React.FC<VotersPerDayProps> = ({
    data,
    width,
    height,
    selection,
    onSelectionChange,
}) => {
    const {t, i18n} = useTranslation()
    const rangeOptions = getVotesTimeRangeOptions(selection.resolution)

    const controls = (
        <Stack direction="row" spacing={0.5}>
            <Tooltip title={String(t("dashboard.timeResolution"))} placement="top">
                <Select
                    value={selection.resolution}
                    size="small"
                    sx={{...compactSelectSx, minWidth: 72}}
                    inputProps={{
                        "aria-label": String(t("dashboard.timeResolution")),
                    }}
                    MenuProps={compactMenuProps}
                    onChange={(event) =>
                        onSelectionChange(
                            withVotesTimeResolution(event.target.value as VotesTimeResolution)
                        )
                    }
                >
                    {(["minute", "hour", "day"] as VotesTimeResolution[]).map((resolution) => (
                        <MenuItem key={resolution} value={resolution}>
                            {String(t(`dashboard.${resolution}`))}
                        </MenuItem>
                    ))}
                </Select>
            </Tooltip>
            <Tooltip title={String(t("dashboard.timeRange"))} placement="top">
                <Select
                    value={selection.range}
                    size="small"
                    sx={{...compactSelectSx, minWidth: 58}}
                    inputProps={{
                        "aria-label": String(t("dashboard.timeRange")),
                    }}
                    MenuProps={compactMenuProps}
                    onChange={(event) =>
                        onSelectionChange({
                            ...selection,
                            range: event.target.value as VotesTimeRange,
                        })
                    }
                >
                    {rangeOptions.map(({value, label}) => (
                        <MenuItem key={value} value={value}>
                            {label}
                        </MenuItem>
                    ))}
                </Select>
            </Tooltip>
        </Stack>
    )

    if (!data) {
        return (
            <CardChart title={String(t("dashboard.votesOverTime"))} actions={controls}>
                <CircularProgress size={24} />
            </CardChart>
        )
    }

    const chartData = toVotesPerDayChartData(data)
    const state: Props = {
        options: getVotesPerDayChartOptions({
            buckets: chartData.buckets,
            resolution: selection.resolution,
            locale: i18n.resolvedLanguage ?? i18n.language,
        }),
        series: chartData.series.map(({channel, data: channelData}) => ({
            name: String(t(`common.channel.${channel.toLowerCase()}`)),
            data: channelData,
        })),
    }

    return (
        <CardChart title={String(t("dashboard.votesOverTime"))} actions={controls}>
            <Chart
                options={state.options}
                series={state.series}
                type="bar"
                width={width}
                height={height}
            />
        </CardChart>
    )
}
