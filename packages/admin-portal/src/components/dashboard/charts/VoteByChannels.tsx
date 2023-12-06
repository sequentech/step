import React from "react"
import Chart, {Props} from "react-apexcharts"
import CardChart from "./Charts"
import {useTranslation} from "react-i18next"
import {useVotesHook} from "./use-votes-hook"

export default function VotesByChannel({
    electionEventId,
    electionId,
    width,
    height,
}: {
    electionEventId: string
    electionId?: string
    width: number
    height: number
}) {
    const {t} = useTranslation()

    const {data} = useVotesHook({
        electionEventId,
        electionId,
    })

    const votesCount = data?.["sequent_backend_cast_vote"]?.length ?? 0

    const series = [votesCount, 0, 0, 0]

    const state: Props = {
        options: {
            labels: ["Online", "Paper", "IVR", "Postal"],
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
        series,
    }

    return (
        <CardChart title={t("dashboard.voteByChannels")}>
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
