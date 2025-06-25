// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext, useMemo} from "react"
import CheckCircleOutlineIcon from "@mui/icons-material/CheckCircleOutline"
import CancelOutlinedIcon from "@mui/icons-material/CancelOutlined"
import {
    ApprovalStats,
    AuthenticationStats,
    calcPrecentage,
    calcTotalApplications,
} from "./ElectionEventStats"
import {AuthContext} from "@/providers/AuthContextProvider"
import {IPermissions} from "@/types/keycloak"
import {StatSection} from "../../Stats"
import {useTranslation} from "react-i18next"

export interface VotersStatsProps {
    eligibleVotersCount: number | string
    enrolledVotersCount: number | string
    approvalStats: ApprovalStats
    authenticationStats: AuthenticationStats
    electionId?: string
}

const useVotersStats = (props: VotersStatsProps) => {
    const {eligibleVotersCount, enrolledVotersCount, approvalStats, authenticationStats} = props

    const {t} = useTranslation()
    const authContext = useContext(AuthContext)

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
                        {
                            icon: <CancelOutlinedIcon />,
                            info: t("monitoringDashboardScreen.voters.invalidUserErrors"),
                            count: authenticationStats.invalidUsersErrorsCount,
                            percentage: calcPrecentage(
                                authenticationStats.invalidUsersErrorsCount,
                                total_auth_errors
                            ),
                        },
                        {
                            icon: <CancelOutlinedIcon />,
                            info: t("monitoringDashboardScreen.voters.invalidPasswordErrors"),
                            count: authenticationStats.invalidPasswordErrorsCount,
                            percentage: calcPrecentage(
                                authenticationStats.invalidPasswordErrorsCount,
                                total_auth_errors
                            ),
                        },
                    ],
                },
            ],
        }
    }, [
        authenticationStats,
        enrolledVotersCount,
        showAuthenticatedVoters,
        approvalStats,
        eligibleVotersCount,
        showAutomaticApproveDisapproveVoters,
        showManuallyApproveDisapproveVoters,
        showAllApproveDisapproveVoters,
        showTotalEnrolledVoters,
    ])

    return {votersSection}
}

export default useVotersStats
