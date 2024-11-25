// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useMemo} from "react"
import CheckCircleOutlineIcon from "@mui/icons-material/CheckCircleOutline"
import CancelOutlinedIcon from "@mui/icons-material/CancelOutlined"
import HourglassEmptyOutlinedIcon from "@mui/icons-material/HourglassEmptyOutlined"
import {Box, Typography} from "@mui/material"
import StatItem from "../../StatItem"
import styled from "@emotion/styled"
import useVotersStats from "./useVotersStats"
import useTestingStats from "./useTestingStats"
import useTallyStats from "./useTallyStats"
import usePollsStats from "./usePollsStats"

const Container = styled(Box)`
    display: flex;
    flex-direction: column;
    gap: 12px;
`

const SectionContainer = styled(Box)`
    display: flex;
    flex-direction: column;
    gap: 8px;
`

const StatsContainer = styled(Box)`
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(250px, 1fr));
    align-items: flex-start;
    gap: 12px;
    justify-items: start
    justify-content: start;
`

export interface TransmissionStats {
    transmittedCount: number | string
    halfTransmittedCount: number | string
    notTransmittedCount: number | string
}
export interface AuthenticationStats {
    authenticatedCount: number | string
    notAuthenticatedCount: number | string
    invalidUsersErrorsCount: number | string
    invalidPasswordErrorsCount: number | string
}

export interface ApprovalStats {
    approvedCount: number | string
    disapprovedCount: number | string
    manualApprovedCount: number | string
    manualDisapprovedCount: number | string
    automatedApprovedCount: number | string
    automatedDisapprovedCount: number | string
}

export interface VotingStats {
    votedCount: number | string
    votedTestsElectionsCount: number | string
}
export interface StatsProps {
    eligibleVotersCount: number | string
    enrolledVotersCount: number | string
    electionsCount: number | string
    openVotesCount: number | string
    notOpenedVotesCount: number | string
    closedVotesCount: number | string
    notClosedVotesCount: number | string
    startCountingVotesCount: number | string
    notStartCountingVotesCount: number | string
    initializeCount: number | string
    notInitializeCount: number | string
    genereatedTallyCount: number | string
    notGenereatedTallyCount: number | string
    votingStats: VotingStats
    approvalStats: ApprovalStats
    transmissionStats: TransmissionStats
    authenticationStats: AuthenticationStats
}

export const calcPrecentage = (part: number | string, total: number | string) =>
    total === 0 || typeof total === "string" || typeof part === "string"
        ? undefined
        : ((part / total) * 100.0).toFixed(2)

const Stats = (props: StatsProps) => {
    const {
        eligibleVotersCount,
        enrolledVotersCount,
        electionsCount,
        openVotesCount,
        notOpenedVotesCount,
        closedVotesCount,
        notClosedVotesCount,
        startCountingVotesCount,
        notStartCountingVotesCount,
        initializeCount,
        notInitializeCount,
        genereatedTallyCount,
        notGenereatedTallyCount,
        approvalStats,
        authenticationStats,
        votingStats,
        transmissionStats,
    } = props

    const {votersSection} = useVotersStats({
        eligibleVotersCount,
        enrolledVotersCount,
        approvalStats,
        authenticationStats,
    })

    const {pollsSection} = usePollsStats({
        eligibleVotersCount,
        enrolledVotersCount,
        electionsCount,
        openVotesCount,
        notOpenedVotesCount,
        closedVotesCount,
        notClosedVotesCount,
        initializeCount,
        notInitializeCount,
        votingStats,
    })

    const {tallySection} = useTallyStats({
        electionsCount,
        startCountingVotesCount,
        notStartCountingVotesCount,
        genereatedTallyCount,
        notGenereatedTallyCount,
        transmissionStats,
    })

    const {testSection} = useTestingStats({
        enrolledVotersCount,
        votingStats,
    })

    const sectionsStats = useMemo(
        () => [votersSection, pollsSection, tallySection, testSection],
        [votersSection, pollsSection, tallySection, testSection]
    )

    return (
        <Container>
            {sectionsStats.map((section) => (
                <>
                    {section.show && (
                        <SectionContainer key={section.title}>
                            <Typography sx={{fontSize: "16px", fontWeight: "500"}}>
                                {section.title}
                            </Typography>
                            <StatsContainer>
                                {section.stats.map((stat) => (
                                    <>{stat.show && <StatItem key={stat.title} {...stat} />}</>
                                ))}
                            </StatsContainer>
                        </SectionContainer>
                    )}
                </>
            ))}
        </Container>
    )
}

export default Stats
