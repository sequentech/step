// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import Chart, {Props} from "react-apexcharts"
import CardChart from "./Charts"
import {useTranslation} from "react-i18next"

export enum VotingChanel {
    Online = "Online",
    Paper = "Paper",
    Telephone = "Telephone",
    Postal = "Postal",
}

export interface TotalVotersRow {
    count: number
    channel: VotingChanel
}

interface VotersByChannelProps {
    data: TotalVotersRow[]
    width: number
    height: number
}

export const VotersByChannel: React.FC<VotersByChannelProps> = ({data, width, height}) => {
    const {t} = useTranslation()

    const state: Props = {
        options: {
            labels: data.map((item) => item.channel.toString()),
            plotOptions: {
                pie: {
                    donut: {
                        labels: {
                            show: true,
                            total: {
                                showAlways: true,
                                show: true,
                            },
                        },
                    },
                },
            },
        },
        series: data.map((item) => item.count),
    }

    return (
        <CardChart title={t("dashboard.votersByChannels")}>
            <Chart
                options={state.options}
                series={state.series}
                type="donut"
                width={width}
                height={height}
            />
        </CardChart>
    )
}
