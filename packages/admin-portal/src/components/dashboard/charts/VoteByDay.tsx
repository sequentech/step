import React from "react"
import Chart, {Props} from "react-apexcharts"
import {useRecordContext} from "react-admin"
import {GetCastVotesQuery, Sequent_Backend_Election_Event} from "@/gql/graphql"
import CardChart, {daysBefore, getWeekLegend} from "./Charts"
import {useTranslation} from "react-i18next"
import {useVotesHook} from "./use-votes-hook"

const now = new Date()

function aggregateByDay(votes: GetCastVotesQuery["sequent_backend_cast_vote"]): Array<number> {
    let values: Array<number> = []

    for (let i = 0; i < 7; i++) {
        let endDate = daysBefore(now, i)
        let startDate = daysBefore(now, i + 1)

        const filteredVotes = votes.filter((vote) => {
            let createdAt = new Date(vote.created_at)
            return createdAt < endDate && createdAt >= startDate
        })

        values.push(filteredVotes.length)
    }

    return values.reverse()
}

export default function VotesByDay({width, height}: {width: number; height: number}) {
    const {t} = useTranslation()
    const record = useRecordContext<Sequent_Backend_Election_Event>()

    const {loading, error, data} = useVotesHook({
        electionEventId: record.id,
        startDate: daysBefore(now, 7),
        endDate: now,
    })

    if (loading || error || !data) {
        return null
    }

    const state: Props = {
        options: {
            chart: {
                id: "barchart-votes",
            },
            xaxis: {
                categories: getWeekLegend(),
            },
        },
        series: [
            {
                name: "series-1",
                data: aggregateByDay(data?.sequent_backend_cast_vote),
            },
        ],
    }

    return (
        <CardChart title={t("dashboard.voteByDay")}>
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
