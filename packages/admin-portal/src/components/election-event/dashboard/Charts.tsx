// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {styled} from "@mui/material/styles"
import {useQuery} from "@apollo/client"
import {Box, Paper, Typography} from "@mui/material"
import Chart, {Props} from "react-apexcharts"
import {IconButton, theme} from "@sequentech/ui-essentials"
import {useRecordContext} from "react-admin"
import {GetCastVotesQuery, Sequent_Backend_Election_Event} from "@/gql/graphql"
import {GET_CAST_VOTES} from "@/queries/GetCastVotes"
import {faClock} from "@fortawesome/free-solid-svg-icons"

const now = new Date()

function daysBefore(date: Date, days: number): Date {
    let before = new Date(date)
    before.setDate(before.getDate() - days)
    return before
}

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

function getWeekLegend(): Array<string> {
    const legend = ["M", "T", "W", "T", "F", "S", "S"]
    const dayOfWeek = now.getDay() // 0-6 day of week

    return [...legend.slice(dayOfWeek, 7), ...legend.slice(0, dayOfWeek)]
}

const StyledPaper = styled(Paper)`
    padding: 16px 16px 25px 16px;
`

const Separator = styled(Box)`
    border-top: 1px solid ${theme.palette.customGrey.light};
    margin: 16px 0;
`

const cardWidth = 470

export function VotesByChannel() {
    const state: Props = {
        options: {
            labels: ["Online", "Paper", "IVR", "Postal"],
        },
        series: [65, 45, 34, 12],
    }

    return (
        <StyledPaper>
            <Chart
                options={state.options}
                series={state.series}
                type="donut"
                width={cardWidth}
                height={250}
            />
            <Separator />
            <Typography fontSize="16px" color={theme.palette.customGrey.main}>
                Votes by Channel
            </Typography>
        </StyledPaper>
    )
}

export function VotesByDay() {
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
                width={cardWidth}
                height={250}
            />

            <Typography fontSize="16px" color={theme.palette.customGrey.main}>
                Votes by day
            </Typography>
            {
                // <Typography
                //     sx={{display: "flex"}}
                //     fontSize="14px"
                //     color={theme.palette.customGrey.main}
                // >
                // <IconButton icon={faClock} fontSize="14px" sx={{marginRight: "4px"}} />
                // <span>Election started 23/12/2022 at 12:00 pm</span>
                //{" "}
                // </Typography>
            }
        </StyledPaper>
    )
}
