import React from "react"
import {useQuery} from "@apollo/client"
import {Typography} from "@mui/material"
import Chart, {Props} from "react-apexcharts"
import {theme} from "@sequentech/ui-essentials"
import {useRecordContext} from "react-admin"
import {GetCastVotesQuery, Sequent_Backend_Election_Event} from "@/gql/graphql"
import {GET_CAST_VOTES} from "@/queries/GetCastVotes"
import {aggregateByDay, daysBefore, getWeekLegend, Separator, StyledPaper} from "../Charts"

const now = new Date()

export default function VotesByDay({width}: {width:number}) {
    const record = useRecordContext<Sequent_Backend_Election_Event>()

    const {loading, error, data} = useQuery<GetCastVotesQuery>(GET_CAST_VOTES, {
        variables: {
            electionEventId: record.id,
            tenantId: record.tenant_id,
            startDate: daysBefore(now, 7).toISOString(),
            endDate: now.toISOString(),
        },
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
        <StyledPaper>
            <Chart
                options={state.options}
                series={state.series}
                type="bar"
                width={width}
                height={250}
            />

            <Separator />

            <Typography fontSize="16px" color={theme.palette.customGrey.main}>
                Votes by day
            </Typography>
        </StyledPaper>
    )
}
