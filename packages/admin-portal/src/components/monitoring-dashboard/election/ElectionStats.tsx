// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext, useMemo} from "react"
import MarkEmailReadOutlinedIcon from "@mui/icons-material/MarkEmailReadOutlined"
import CheckCircleOutlineIcon from "@mui/icons-material/CheckCircleOutline"
import CancelOutlinedIcon from "@mui/icons-material/CancelOutlined"
import {AuthContext} from "@/providers/AuthContextProvider"
import {IPermissions} from "@/types/keycloak"
import {calcPrecentage, calcTotalApplications} from "../election-event/stats/ElectionEventStats"
import Stats, {StatSection} from "../Stats"
import {useTranslation} from "react-i18next"

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

    const {t} = useTranslation()
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
            title: t("monitoringDashboardScreen.voters.title"),
            stats: [
                {
                    show: showTotalEnrolledVoters,
                    title: t("monitoringDashboardScreen.voters.enrolledOverseasVoters"),
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
                    title: t("monitoringDashboardScreen.voters.approvalStatus"),
                    items: [
                        {
                            icon: <CheckCircleOutlineIcon />,
                            count: approvalStats.approvedCount,
                            percentage: calcPrecentage(
                                approvalStats.approvedCount,
                                calcTotalApplications(
                                    approvalStats.approvedCount,
                                    approvalStats.disapprovedCount
                                )
                            ),
                        },
                        {
                            icon: <CancelOutlinedIcon />,
                            count: approvalStats.disapprovedCount,
                            percentage: calcPrecentage(
                                approvalStats.disapprovedCount,
                                calcTotalApplications(
                                    approvalStats.approvedCount,
                                    approvalStats.disapprovedCount
                                )
                            ),
                        },
                    ],
                },
                {
                    show: showManuallyApproveDisapproveVoters,
                    title: t("monitoringDashboardScreen.voters.manuallyApproval"),
                    items: [
                        {
                            icon: <CheckCircleOutlineIcon />,
                            count: approvalStats.manualApprovedCount,
                            percentage: calcPrecentage(
                                approvalStats.manualApprovedCount,
                                calcTotalApplications(
                                    approvalStats.manualApprovedCount,
                                    approvalStats.manualDisapprovedCount
                                )
                            ),
                        },
                        {
                            icon: <CancelOutlinedIcon />,
                            count: approvalStats.manualDisapprovedCount,
                            percentage: calcPrecentage(
                                approvalStats.manualDisapprovedCount,
                                calcTotalApplications(
                                    approvalStats.manualApprovedCount,
                                    approvalStats.manualDisapprovedCount
                                )
                            ),
                        },
                    ],
                },
                {
                    show: showAutomaticApproveDisapproveVoters,
                    title: t("monitoringDashboardScreen.voters.automaticallyApproval"),
                    items: [
                        {
                            icon: <CheckCircleOutlineIcon />,
                            count: approvalStats.automatedApprovedCount,
                            percentage: calcPrecentage(
                                approvalStats.automatedApprovedCount,
                                calcTotalApplications(
                                    approvalStats.automatedApprovedCount,
                                    approvalStats.automatedDisapprovedCount
                                )
                            ),
                        },
                        {
                            icon: <CancelOutlinedIcon />,
                            count: approvalStats.automatedDisapprovedCount,
                            percentage: calcPrecentage(
                                approvalStats.automatedDisapprovedCount,
                                calcTotalApplications(
                                    approvalStats.automatedApprovedCount,
                                    approvalStats.automatedDisapprovedCount
                                )
                            ),
                        },
                    ],
                },
                {
                    show: showAuthenticatedVoters,
                    title: t("monitoringDashboardScreen.voters.authenticatedVoters"),
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
    }, [authenticationStats, enrolledVotersCount, approvalStats])

    const pollsSection: StatSection = useMemo(() => {
        return {
            show: showVotersVoted,
            title: t("monitoringDashboardScreen.polls.title"),
            stats: [
                {
                    show: showVotersVoted,
                    title: t("monitoringDashboardScreen.polls.voterTurnout"),
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
    }, [votedCount, eligibleVotersCount])

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
