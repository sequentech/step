// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import Chart, {Props} from "react-apexcharts"
import CardChart from "./Charts"
import {useTranslation} from "react-i18next"
import {TotalVotersRow} from "./votersByChannelData"

interface VotersByChannelProps {
    data: TotalVotersRow[]
    width: number
    height: number
}

export const VotersByChannel: React.FC<VotersByChannelProps> = ({data, width, height}) => {
    const {t} = useTranslation()

    const state: Props = {
        options: {
            labels: data.map((item) => String(t(`common.channel.${item.channel.toLowerCase()}`))),
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
        <CardChart title={String(t("dashboard.votersByChannels"))}>
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
