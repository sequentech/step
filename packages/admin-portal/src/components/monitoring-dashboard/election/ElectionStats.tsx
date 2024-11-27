// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext, useMemo} from "react"
import MarkEmailReadOutlinedIcon from "@mui/icons-material/MarkEmailReadOutlined"
import CheckCircleOutlineIcon from "@mui/icons-material/CheckCircleOutline"
import CancelOutlinedIcon from "@mui/icons-material/CancelOutlined"
import {AuthContext} from "@/providers/AuthContextProvider"
import {IPermissions} from "@/types/keycloak"
import {calcPrecentage} from "../election-event/stats/ElectionEventStats"
import Stats, {StatSection} from "../Stats"

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

export interface ElectionStatsProps {
    eligibleVotersCount: number | string
    enrolledVotersCount: number | string
    votedCount: number | string
    approvalStats: ApprovalStats
    authenticationStats: AuthenticationStats
}

const ElectionStats = (props: ElectionStatsProps) => {
    const {
        eligibleVotersCount,
        enrolledVotersCount,
        approvalStats,
        authenticationStats,
        votedCount,
    } = props

    const authContext = useContext(AuthContext)

    const showVotersVoted = authContext.isAuthorized(
        true,
        authContext.tenantId,
        IPermissions.MONITOR_VOTERS_WHO_VOTED
    )

    const showTotalEnrolledVoters = authContext.isAuthorized(
        true,
        authContext.tenantId,
        IPermissions.MONITOR_ENROLLED_OVERSEAS_VOTERS
    )

    const showAllApproveDisapproveVoters = authContext.isAuthorized(
        true,
        authContext.tenantId,
        IPermissions.MONITOR_ALL_APPROVE_DISAPPROVE_VOTERS
    )

    const showManuallyApproveDisapproveVoters = authContext.isAuthorized(
        true,
        authContext.tenantId,
        IPermissions.MONITOR_MANUALLY_APPROVE_DISAPPROVE_VOTERS
    )
    const showAutomaticApproveDisapproveVoters = authContext.isAuthorized(
        true,
        authContext.tenantId,
        IPermissions.MONITOR_AUTOMATIC_APPROVE_DISAPPROVE_VOTERS
    )

    const showAuthenticatedVoters = authContext.isAuthorized(
        true,
        authContext.tenantId,
        IPermissions.MONITOR_AUTHENTICATED_VOTERS
    )
    console.log("showAuthenticatedVoters:", showAuthenticatedVoters)

    const votersSection: StatSection = useMemo(() => {
        let total_auth_errors = 0
        if (
            typeof authenticationStats.invalidUsersErrorsCount === "number" &&
            typeof authenticationStats.invalidPasswordErrorsCount === "number"
        ) {
            total_auth_errors =
                authenticationStats.invalidUsersErrorsCount +
                authenticationStats.invalidPasswordErrorsCount
        }
        return {
            show:
                showTotalEnrolledVoters ||
                showAllApproveDisapproveVoters ||
                showManuallyApproveDisapproveVoters ||
                showAutomaticApproveDisapproveVoters ||
                showAuthenticatedVoters,
            title: "Voters",
            stats: [
                {
                    show: showTotalEnrolledVoters,
                    title: "Total Enrolled Overseas Voters",
                    items: [
                        {
                            icon: <CheckCircleOutlineIcon />,
                            count: enrolledVotersCount,
                            percentage: calcPrecentage(enrolledVotersCount, eligibleVotersCount),
                        },
                    ],
                },
                {
                    show: showAllApproveDisapproveVoters,
                    title: "Approve/Disapprove Voters",
                    items: [
                        {
                            icon: <CheckCircleOutlineIcon />,
                            count: approvalStats.approvedCount,
                            percentage: calcPrecentage(
                                approvalStats.approvedCount,
                                eligibleVotersCount
                            ),
                        },
                        {
                            icon: <CancelOutlinedIcon />,
                            count: approvalStats.disapprovedCount,
                            percentage: calcPrecentage(
                                approvalStats.disapprovedCount,
                                eligibleVotersCount
                            ),
                        },
                    ],
                },
                {
                    show: showManuallyApproveDisapproveVoters,
                    title: "Manually Approve/Disapprove Voters",
                    items: [
                        {
                            icon: <CheckCircleOutlineIcon />,
                            count: approvalStats.manualApprovedCount,
                            percentage: calcPrecentage(
                                approvalStats.manualApprovedCount,
                                eligibleVotersCount
                            ),
                        },
                        {
                            icon: <CancelOutlinedIcon />,
                            count: approvalStats.manualDisapprovedCount,
                            percentage: calcPrecentage(
                                approvalStats.manualDisapprovedCount,
                                eligibleVotersCount
                            ),
                        },
                    ],
                },
                {
                    show: showAutomaticApproveDisapproveVoters,
                    title: "Automatic Approve/Disapprove Voters",
                    items: [
                        {
                            icon: <CheckCircleOutlineIcon />,
                            count: approvalStats.automatedApprovedCount,
                            percentage: calcPrecentage(
                                approvalStats.automatedApprovedCount,
                                eligibleVotersCount
                            ),
                        },
                        {
                            icon: <CancelOutlinedIcon />,
                            count: approvalStats.automatedDisapprovedCount,
                            percentage: calcPrecentage(
                                approvalStats.automatedDisapprovedCount,
                                eligibleVotersCount
                            ),
                        },
                    ],
                },
                {
                    show: showAuthenticatedVoters,
                    title: "Total authenticated Voter",
                    items: [
                        {
                            icon: <CheckCircleOutlineIcon />,
                            count: authenticationStats.authenticatedCount,
                            percentage: calcPrecentage(
                                authenticationStats.authenticatedCount,
                                enrolledVotersCount
                            ),
                        },
                        {
                            icon: <CancelOutlinedIcon />,
                            count: authenticationStats.notAuthenticatedCount,
                            percentage: calcPrecentage(
                                authenticationStats.notAuthenticatedCount,
                                enrolledVotersCount
                            ),
                        },
                    ],
                },
            ],
        }
    }, [])

    const pollsSection: StatSection = useMemo(() => {
        return {
            show: showVotersVoted,
            title: "Polls",
            stats: [
                {
                    show: showVotersVoted,
                    title: "Total Voters who voted",
                    items: [
                        {
                            icon: <MarkEmailReadOutlinedIcon />,
                            count: votedCount,
                            percentage: calcPrecentage(votedCount, eligibleVotersCount),
                        },
                    ],
                },
            ],
        }
    }, [])

    const statsSections: StatSection[] = useMemo(
        () => [votersSection, pollsSection],
        [votersSection, pollsSection]
    )

    return (
        <>
            <Stats statsSections={statsSections} />
        </>
    )
}

export default ElectionStats
