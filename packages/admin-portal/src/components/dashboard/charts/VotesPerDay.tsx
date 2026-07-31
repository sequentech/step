// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import Chart, {Props} from "react-apexcharts"
import CardChart, {getWeekLegend} from "./Charts"
import {CastVotesPerDay} from "@/gql/graphql"
import {useTranslation} from "react-i18next"
import {CircularProgress} from "@mui/material"
import {toVotesPerDayChartData} from "./votesPerDayData"
import {getVotesPerDayChartOptions} from "./votesPerDayOptions"

export interface VotersPerDayProps {
    data: CastVotesPerDay[] | null
    width: number
    height: number
    endDate: Date
}

export const VotesPerDay: React.FC<VotersPerDayProps> = ({data, width, height, endDate}) => {
    const {t} = useTranslation()

    if (!data) {
        return <CircularProgress />
    }

    const chartData = toVotesPerDayChartData(data)
    const state: Props = {
        options: getVotesPerDayChartOptions(getWeekLegend(endDate)),
        series: chartData.series.map(({channel, data: channelData}) => ({
            name: String(t(`common.channel.${channel.toLowerCase()}`)),
            data: channelData,
        })),
    }

    return (
        <CardChart title={String(t("dashboard.voteByDay"))}>
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
