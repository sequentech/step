import React from "react"
import Chart, {Props} from "react-apexcharts"
import CardChart from "./Charts"
import {useTranslation} from "react-i18next"

export default function VotesByChannel({width, height}: {width: number; height: number}) {
    const {t} = useTranslation()

    const state: Props = {
        options: {
            labels: ["Online", "Paper", "IVR", "Postal"],
        },
        series: [65, 45, 34, 12],
    }

    return (
        <CardChart title={t("voteByChannel")}>
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
