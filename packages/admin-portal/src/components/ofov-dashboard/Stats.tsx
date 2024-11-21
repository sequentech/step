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

type Metric = {
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

const Stats = (props: Metric) => {
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
                                info: enrolledVotersCount,
                                percentageInfo: calcPrecentage(
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
                                info: approvedVotersCount,
                                percentageInfo: calcPrecentage(
                                    approvedVotersCount,
                                    eligibleVotersCount
                                ),
                            },
                            {
                                icon: <CancelOutlinedIcon />,
                                info: disapprovedVotersCount,
                                percentageInfo: calcPrecentage(
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
                                info: enrolledVotersCount,
                                percentageInfo: calcPrecentage(
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
                                info: initializeCount,
                                percentageInfo: calcPrecentage(initializeCount, electionsCount),
                            },
                            {
                                icon: <CancelOutlinedIcon />,
                                info: notInitializeCount,
                                percentageInfo: calcPrecentage(notInitializeCount, electionsCount),
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
                                info: openVotesCount,
                                percentageInfo: calcPrecentage(openVotesCount, electionsCount),
                            },
                            {
                                icon: <CancelOutlinedIcon />,
                                info: notOpenedVotesCount,
                                percentageInfo: calcPrecentage(notOpenedVotesCount, electionsCount),
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
                                info: ClosedVotesCount,
                                percentageInfo: calcPrecentage(ClosedVotesCount, electionsCount),
                            },
                            {
                                icon: <CancelOutlinedIcon />,
                                info: notClosedVotesCount,
                                percentageInfo: calcPrecentage(notClosedVotesCount, electionsCount),
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
                                info: startCountingVotesCount,
                                percentageInfo: calcPrecentage(
                                    startCountingVotesCount,
                                    electionsCount
                                ),
                            },
                            {
                                icon: <CancelOutlinedIcon />,
                                info: notStartCountingVotesCount,
                                percentageInfo: calcPrecentage(
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
                                info: genereatedTallyCount,
                                percentageInfo: calcPrecentage(
                                    genereatedTallyCount,
                                    electionsCount
                                ),
                            },
                            {
                                icon: <CancelOutlinedIcon />,
                                info: notGenereatedTallyCount,
                                percentageInfo: calcPrecentage(
                                    notGenereatedTallyCount,
                                    electionsCount
                                ),
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
                                info: transmittedResultsCount,
                                percentageInfo: calcPrecentage(
                                    transmittedResultsCount,
                                    electionsCount
                                ),
                            },
                            {
                                icon: <CancelOutlinedIcon />,
                                info: notTransmittedResultsCounts,
                                percentageInfo: calcPrecentage(
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
