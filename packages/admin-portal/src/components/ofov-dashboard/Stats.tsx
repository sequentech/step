// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useMemo} from "react"
import CheckCircleOutlineIcon from "@mui/icons-material/CheckCircleOutline"
import CancelOutlinedIcon from "@mui/icons-material/CancelOutlined"
import {Box, Typography} from "@mui/material"
import StatItem from "./StatItem"
import styled from "@emotion/styled"

const Container = styled(Box)`
    display: flex;
    flex-direction: column;
    gap: 8px
    align-items: center;
`

const SectionContainer = styled(Box)`
    display: flex;
    flex-direction: column;
    gap: 8px;
`

const StatsContainer = styled(Box)`
    display: flex;
    gap: 12px;
    align-items: flex-start;
`

export interface StatsProps {
    eligibleVotersCount: number | string
    enrolledVotersCount: number | string
    electionsCount: number | string
    approvedVotersCount: number | string
    disapprovedVotersCount: number | string
    disapprovedResons: string[]
    openVotesCount: number | string
    notOpenedVotesCount: number | string
    ClosedVotesCount: number | string
    notClosedVotesCount: number | string
    startCountingVotesCount: number | string
    notStartCountingVotesCount: number | string
    initializeCount: number | string
    notInitializeCount: number | string
    genereatedTallyCount: number | string
    notGenereatedTallyCount: number | string
    transmittedResultsCount: number | string
    notTransmittedResultsCounts: number | string
}

const Stats = (props: StatsProps) => {
    const {
        eligibleVotersCount,
        enrolledVotersCount,
        electionsCount,
        approvedVotersCount,
        disapprovedVotersCount,
        disapprovedResons,
        openVotesCount,
        notOpenedVotesCount,
        ClosedVotesCount,
        notClosedVotesCount,
        startCountingVotesCount,
        notStartCountingVotesCount,
        initializeCount,
        notInitializeCount,
        genereatedTallyCount,
        notGenereatedTallyCount,
        transmittedResultsCount,
        notTransmittedResultsCounts,
    } = props

    const calcPrecentage = (part: number | string, total: number | string) =>
        total === 0 || typeof total === "string" || typeof part === "string"
            ? undefined
            : ((part / total) * 100.0).toFixed(2)

    const sectionsStats = useMemo(
        () => [
            {
                title: "Pre-registration and issuance of voting credentials",
                stats: [
                    {
                        title: "Total overseas voters",
                        items: [
                            {
                                icon: <CheckCircleOutlineIcon />,
                                count: enrolledVotersCount,
                                percentage: calcPrecentage(
                                    enrolledVotersCount,
                                    eligibleVotersCount
                                ),
                            },
                        ],
                    },
                    {
                        title: "Number of approved vs disapproved applications",
                        items: [
                            {
                                icon: <CheckCircleOutlineIcon />,
                                count: approvedVotersCount,
                                percentage: calcPrecentage(
                                    approvedVotersCount,
                                    eligibleVotersCount
                                ),
                            },
                            {
                                icon: <CancelOutlinedIcon />,
                                count: disapprovedVotersCount,
                                percentage: calcPrecentage(
                                    disapprovedVotersCount,
                                    eligibleVotersCount
                                ),
                            },
                        ],
                    },
                    {
                        title: "Number of issued voting credentials",
                        items: [
                            {
                                icon: <CheckCircleOutlineIcon />,
                                count: enrolledVotersCount,
                                percentage: calcPrecentage(
                                    enrolledVotersCount,
                                    eligibleVotersCount
                                ),
                            },
                        ],
                    },
                ],
            },
            {
                title: "System initialization",
                stats: [
                    {
                        title: "Total Posts initialized the system",
                        items: [
                            {
                                icon: <CheckCircleOutlineIcon />,
                                count: initializeCount,
                                percentage: calcPrecentage(initializeCount, electionsCount),
                            },
                            {
                                icon: <CancelOutlinedIcon />,
                                count: notInitializeCount,
                                percentage: calcPrecentage(notInitializeCount, electionsCount),
                            },
                        ],
                    },
                ],
            },
            {
                title: "Opening of polls",
                stats: [
                    {
                        title: "Total Posts which already opened voting",
                        items: [
                            {
                                icon: <CheckCircleOutlineIcon />,
                                count: openVotesCount,
                                percentage: calcPrecentage(openVotesCount, electionsCount),
                            },
                            {
                                icon: <CancelOutlinedIcon />,
                                count: notOpenedVotesCount,
                                percentage: calcPrecentage(notOpenedVotesCount, electionsCount),
                            },
                        ],
                    },
                ],
            },
            {
                title: "Closing of polls",
                stats: [
                    {
                        title: "Total Posts which already closed polls",
                        items: [
                            {
                                icon: <CheckCircleOutlineIcon />,
                                count: ClosedVotesCount,
                                percentage: calcPrecentage(ClosedVotesCount, electionsCount),
                            },
                            {
                                icon: <CancelOutlinedIcon />,
                                count: notClosedVotesCount,
                                percentage: calcPrecentage(notClosedVotesCount, electionsCount),
                            },
                        ],
                    },
                ],
            },
            {
                title: "Vote counting",
                stats: [
                    {
                        title: "Total Posts which already started counting",
                        items: [
                            {
                                icon: <CheckCircleOutlineIcon />,
                                count: startCountingVotesCount,
                                percentage: calcPrecentage(startCountingVotesCount, electionsCount),
                            },
                            {
                                icon: <CancelOutlinedIcon />,
                                count: notStartCountingVotesCount,
                                percentage: calcPrecentage(
                                    notStartCountingVotesCount,
                                    electionsCount
                                ),
                            },
                        ],
                    },
                ],
            },
            {
                title: "Election Returns generation",
                stats: [
                    {
                        title: "Total Posts which already generated ERs",
                        items: [
                            {
                                icon: <CheckCircleOutlineIcon />,
                                count: genereatedTallyCount,
                                percentage: calcPrecentage(genereatedTallyCount, electionsCount),
                            },
                            {
                                icon: <CancelOutlinedIcon />,
                                count: notGenereatedTallyCount,
                                percentage: calcPrecentage(notGenereatedTallyCount, electionsCount),
                            },
                        ],
                    },
                ],
            },
            {
                title: "Transmission of results",
                stats: [
                    {
                        title: "which already transmitted results",
                        items: [
                            {
                                icon: <CheckCircleOutlineIcon />,
                                count: transmittedResultsCount,
                                percentage: calcPrecentage(transmittedResultsCount, electionsCount),
                            },
                            {
                                icon: <CancelOutlinedIcon />,
                                count: notTransmittedResultsCounts,
                                percentage: calcPrecentage(
                                    notTransmittedResultsCounts,
                                    electionsCount
                                ),
                            },
                        ],
                    },
                ],
            },
        ],
        [
            eligibleVotersCount,
            enrolledVotersCount,
            electionsCount,
            approvedVotersCount,
            disapprovedVotersCount,
            disapprovedResons,
            openVotesCount,
            notOpenedVotesCount,
            ClosedVotesCount,
            notClosedVotesCount,
            startCountingVotesCount,
            notStartCountingVotesCount,
            initializeCount,
            notInitializeCount,
            genereatedTallyCount,
            notGenereatedTallyCount,
            transmittedResultsCount,
            notTransmittedResultsCounts,
        ]
    )

    return (
        <Container>
            {sectionsStats.map((section) => (
                <SectionContainer key={section.title}>
                    <Typography sx={{fontSize: "16px", fontWeight: "500"}}>
                        {section.title}
                    </Typography>
                    <StatsContainer>
                        {section.stats.map((stat) => (
                            <StatItem key={stat.title} {...stat} />
                        ))}
                    </StatsContainer>
                </SectionContainer>
            ))}
        </Container>
    )
}

export default Stats
