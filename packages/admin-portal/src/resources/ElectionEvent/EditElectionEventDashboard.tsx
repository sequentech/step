// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Box, Paper, Typography} from "@mui/material"
import Chart, {Props} from "react-apexcharts"
import {
    GetCastVotesQuery,
    GetElectionEventStatsQuery,
    Sequent_Backend_Election_Event,
} from "../../gql/graphql"
import {IconButton, theme} from "@sequentech/ui-essentials"
import {ShowBase, TabbedShowLayout, TextField, useRecordContext} from "react-admin"
import {
    faBriefcase,
    faCalendar,
    faClock,
    faCommentSms,
    faEnvelope,
    faGlobe,
    faUsers,
} from "@fortawesome/free-solid-svg-icons"

    import ElectionHeader from '../../components/ElectionHeader'
import {GET_CAST_VOTES} from "../../queries/GetCastVotes"
import {GET_ELECTION_EVENT_STATS} from "../../queries/GetElectionEventStats"
import React from "react"
import {ReportDialog} from "../../components/ReportDialog"
import {styled} from "@mui/material/styles"
import {useQuery} from "@apollo/client"
import { EditElectionEventData } from './EditElectionEventData'

const CardList = styled(Box)`
    display: flex;
    flex-direction: row;
    gap: 24px;
    margin: 20px 0;
`

const CardContainer = styled(Box)<{selected?: boolean}>`
    display: flex;
    flex-direction: column;
    padding: 16px;
    border-radius: 4px;
    border: 1px solid ${theme.palette.customGrey.light};
    color: ${({selected}) => (selected ? theme.palette.white : theme.palette.customGrey.main)};
    justify-content: center;
    text-align: center;
    width: 160px;
    height: 140px;
    ${({selected}) =>
        selected ? "background: linear-gradient(180deg, #0FADCF 0%, #0F054B 100%); " : ""}
`

export const ChartsContainer = styled(Box)`
    display: flex;
    flex-direction: row;
    flex-wrap: wrap;
    gap: 24px;
`

const BarChartPaper = styled(Paper)`
    padding: 16px 16px 25px 16px;
`

const Separator = styled(Box)`
    border-top: 1px solid ${theme.palette.customGrey.light};
    margin: 16px 0;
`

export const PieChart: React.FC = () => {
    const state: Props = {
        options: {
            labels: ["Online", "Paper", "IVR", "Postal"],
        },
        series: [65, 45, 34, 12],
    }

    return (
        <BarChartPaper>
            <Chart
                options={state.options}
                series={state.series}
                type="donut"
                width={370}
                height={250}
            />
            <Separator />
            <Typography fontSize="16px" color={theme.palette.customGrey.main}>
                Votes by Channel
            </Typography>
        </BarChartPaper>
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

const getWeekLegend = (): Array<string> => {
    const legend = ["M", "T", "W", "T", "F", "S", "S"]
    const dayOfWeek = now.getDay() // 0-6 day of week

    return [...legend.slice(dayOfWeek, 7), ...legend.slice(0, dayOfWeek)]
}

export const BarChart: React.FC = () => {
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
        <BarChartPaper>
            <Chart
                options={state.options}
                series={state.series}
                type="bar"
                width={370}
                height={200}
            />
            <Typography fontSize="16px" color={theme.palette.customGrey.main}>
                Votes by Day
            </Typography>
            <Separator />
            <Typography fontSize="14px" color={theme.palette.customGrey.main}>
                <IconButton icon={faClock} fontSize="14px" />
                Election started 23/12/2022 at 12:00 pm
            </Typography>
        </BarChartPaper>
    )
}

export const ElectionStats: React.FC = () => {
    const record = useRecordContext<Sequent_Backend_Election_Event>()

    const {loading, error, data} = useQuery<GetElectionEventStatsQuery>(GET_ELECTION_EVENT_STATS, {
        variables: {
            electionEventId: record.id,
            tenantId: record.tenant_id,
        },
    })

    if (loading || error || !data) {
        return null
    }

    return (
        <CardList>
            <CardContainer>
                <IconButton icon={faBriefcase} fontSize="38px" />
                <Typography fontSize="24px">5</Typography>
                <Typography fontSize="12px">TRUSTEES</Typography>
            </CardContainer>
            <CardContainer selected={true}>
                <IconButton icon={faUsers} fontSize="38px" />
                <Typography fontSize="24px">
                    {data.sequent_backend_cast_vote_aggregate.aggregate?.count}
                </Typography>
                <Typography fontSize="12px">VOTERS</Typography>
            </CardContainer>
            <CardContainer>
                <IconButton icon={faUsers} fontSize="38px" />
                <Typography fontSize="24px">
                    {data.sequent_backend_election_aggregate.aggregate?.count}
                </Typography>
                <Typography fontSize="12px">ELECTIONS</Typography>
            </CardContainer>
            <CardContainer>
                <IconButton icon={faGlobe} fontSize="38px" />
                <Typography fontSize="24px">
                    {data.sequent_backend_area_aggregate.aggregate?.count}
                </Typography>
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
    )
}
