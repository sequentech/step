// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import Chart, {Props} from "react-apexcharts"
import CardChart from "./Charts"
import {useTranslation} from "react-i18next"
import {useNumberFormat} from "@sequentech/ui-core"
import {TotalVotersRow} from "./votersByChannelData"
import {getVotersByChannelChartOptions} from "./votersByChannelOptions"

interface VotersByChannelProps {
    data: TotalVotersRow[]
    width: number
    height: number
}

export const VotersByChannel: React.FC<VotersByChannelProps> = ({data, width, height}) => {
    const {t} = useTranslation()
    const {policy} = useNumberFormat()
    const visibleData = data.filter(({count}) => count > 0)

    const state: Props = {
        options: getVotersByChannelChartOptions({
            labels: visibleData.map((item) =>
                String(t(`common.channel.${item.channel.toLowerCase()}`))
            ),
            numberFormatPolicy: policy,
        }),
        series: visibleData.map((item) => item.count),
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
