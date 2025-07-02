// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useMemo} from "react"
import Stats from "../../Stats"
import useVotersStats from "./useVotersStats"
import useTestingStats from "./useTestingStats"
import useTallyStats from "./useTallyStats"
import usePollsStats from "./usePollsStats"

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
export interface ElectionEventStatsProps {
    eligibleVotersCount: number | string
    enrolledVotersCount: number | string
    electionsCount: number | string
    startedVoteCount: number | string
    notStartedVotesCount: number | string
    openVotesCount: number | string
    notOpenVotesCount: number | string
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

export const calcTotalApplications = (approved: number | string, disapproved: number | string) =>
    typeof approved === "string" || typeof disapproved === "string" ? 0 : disapproved + approved

export const calcPrecentage = (part: number | string, total: number | string) =>
    total === 0 || typeof total === "string" || typeof part === "string"
        ? "0"
        : ((part / total) * 100.0).toFixed(2)

const ElectionEventStats = (props: ElectionEventStatsProps) => {
    const {
        eligibleVotersCount,
        enrolledVotersCount,
        electionsCount,
        startedVoteCount,
        notStartedVotesCount,
        openVotesCount,
        notOpenVotesCount,
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
        startedVoteCount,
        notStartedVotesCount,
        openVotesCount,
        notOpenVotesCount,
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

    const statsSections = useMemo(
        () => [votersSection, pollsSection, tallySection, testSection],
        [votersSection, pollsSection, tallySection, testSection]
    )

    return <Stats statsSections={statsSections} />
}

export default ElectionEventStats
