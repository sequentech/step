// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import Chart, {Props} from "react-apexcharts"
import CardChart, {getWeekLegend} from "./Charts"
import {CastVotesPerDay} from "@/gql/graphql"
import {useTranslation} from "react-i18next"
import {CircularProgress} from "@mui/material"

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

    const state: Props = {
        options: {
            chart: {
                id: "barchart-votes",
            },
            xaxis: {
                categories: getWeekLegend(endDate),
            },
        },
        series: [
            {
                name: "series-1",
                data: data.map((item) => item.day_count),
            },
        ],
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
