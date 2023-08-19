// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Box, Typography} from "@mui/material"
import React, {useEffect} from "react"
import {Show, TextField, useRecordContext} from "react-admin"
import Chart, {Props} from "react-apexcharts"
import {styled} from "@mui/material/styles"
import {IconButton, theme} from "@sequentech/ui-essentials"
import {
    faBriefcase,
    faUsers,
    faGlobe,
    faEnvelope,
    faCommentSms,
    faCalendar,
} from "@fortawesome/free-solid-svg-icons"
import {useQuery} from "@apollo/client"
import {
    GetCastVotesQuery,
    Sequent_Backend_Cast_Vote,
    Sequent_Backend_Election_Event,
} from "../../gql/graphql"
import {GET_CAST_VOTES} from "../../queries/GetCastVotes"

const CardList = styled(Box)`
    display: flex;
    flex-direction: row;
    gap: 24px;
    margin: 20px 0;
`

const CardContainer = styled(Box)<{selected?: string}>`
    display: flex;
    flex-direction: column;
    padding: 16px;
    border-radius: 4px;
    border: 1px solid ${theme.palette.customGrey.light};
    color: ${({selected}) =>
        "true" === selected ? theme.palette.white : theme.palette.customGrey.main};
    justify-content: center;
    text-align: center;
    width: 160px;
    height: 140px;
    ${({selected}) =>
        "true" === selected
            ? "background: linear-gradient(180deg, #0FADCF 0%, #0F054B 100%); "
            : ""}
`

const ChartsContainer = styled(Box)`
    display: flex;
    flex-direction: row;
    flex-wrap: wrap;
`

const PieChart: React.FC = () => {
    const state: Props = {
        options: {
            labels: ["Online", "Paper", "IVR", "Postal"],
        },
        series: [65, 45, 34, 12],
    }

    return (
        <Chart
            options={state.options}
            series={state.series}
            type="donut"
            width={500}
            height={320}
        />
    )
}

const daysBefore = (date: Date, days: number): Date => {
    let before = new Date(date)
    before.setDate(before.getDate() - days)
    return before
}
const now = new Date()

const aggregateByDay = (votes?: GetCastVotesQuery["sequent_backend_cast_vote"]): Array<number> => {
    if (!votes) {
        return [35, 50, 49, 60, 70, 91, 125]
    }
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

const BarChart: React.FC = () => {
    const record = useRecordContext<Sequent_Backend_Election_Event>()

    const {loading, error, data} = useQuery<GetCastVotesQuery>(GET_CAST_VOTES, {
        variables: {
            electionEventId: record.id,
            tenantId: record.tenant_id,
            startDate: daysBefore(now, 7).toISOString(),
            endDate: now.toISOString(),
        },
    })

    useEffect(() => {
        if (!loading && !error) {
            console.log(data)
        }
    }, [loading, error, data])

    const state: Props = {
        options: {
            chart: {
                id: "apexchart-example",
            },
            xaxis: {
                categories: ["M", "T", "W", "T", "F", "S", "S"],
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
        <Chart options={state.options} series={state.series} type="bar" width={500} height={320} />
    )
}

export const ShowElectionEvent: React.FC = () => {
    const state: Props = {
        options: {
            labels: ["A", "B", "C", "D", "E"],
        },
        series: [44, 55, 41, 17, 15],
    }

    return (
        <Show>
            <Box sx={{padding: "16px"}}>
                <TextField source="name" fontSize="24px" fontWeight="bold" />
                <CardList>
                    <CardContainer>
                        <IconButton icon={faBriefcase} fontSize="38px" />
                        <Typography fontSize="24px">5</Typography>
                        <Typography fontSize="12px">TRUSTEES</Typography>
                    </CardContainer>
                    <CardContainer selected="true">
                        <IconButton icon={faUsers} fontSize="38px" />
                        <Typography fontSize="24px">128</Typography>
                        <Typography fontSize="12px">VOTERS</Typography>
                    </CardContainer>
                    <CardContainer>
                        <IconButton icon={faUsers} fontSize="38px" />
                        <Typography fontSize="24px">10</Typography>
                        <Typography fontSize="12px">ELECTIONS</Typography>
                    </CardContainer>
                    <CardContainer>
                        <IconButton icon={faGlobe} fontSize="38px" />
                        <Typography fontSize="24px">10</Typography>
                        <Typography fontSize="12px">AREAS</Typography>
                    </CardContainer>
                    <CardContainer>
                        <IconButton icon={faEnvelope} fontSize="38px" />
                        <Typography fontSize="24px">50</Typography>
                        <Typography fontSize="12px">EMAILS SENT</Typography>
                    </CardContainer>
                    <CardContainer>
                        <IconButton icon={faCommentSms} fontSize="38px" />
                        <Typography fontSize="24px">50</Typography>
                        <Typography fontSize="12px">SMS SENT</Typography>
                    </CardContainer>
                    <CardContainer>
                        <IconButton icon={faCalendar} fontSize="38px" />
                        <Typography fontSize="24px">Scheduled</Typography>
                        <Typography fontSize="12px">CALENDAR</Typography>
                    </CardContainer>
                </CardList>
                <ChartsContainer>
                    <BarChart />
                    <BarChart />
                    <PieChart />
                </ChartsContainer>
            </Box>
        </Show>
    )
}
