// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext, useMemo} from "react"
import CheckCircleOutlineIcon from "@mui/icons-material/CheckCircleOutline"
import CancelOutlinedIcon from "@mui/icons-material/CancelOutlined"
import {ApprovalStats, AuthenticationStats, calcPrecentage} from "./Stats"
import {AuthContext} from "@/providers/AuthContextProvider"
import {IPermissions} from "@/types/keycloak"

export interface VotersStatsProps {
    eligibleVotersCount: number | string
    enrolledVotersCount: number | string
    approvalStats: ApprovalStats
    authenticationStats: AuthenticationStats
}

const useVotersStats = (props: VotersStatsProps) => {
    const {eligibleVotersCount, enrolledVotersCount, approvalStats, authenticationStats} = props

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

    const votersSection = useMemo(() => {
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
                        {
                            icon: <CancelOutlinedIcon />,
                            info: "Invalid User Errors:",
                            count: authenticationStats.invalidUsersErrorsCount,
                            percentage: calcPrecentage(
                                authenticationStats.invalidUsersErrorsCount,
                                total_auth_errors
                            ),
                        },
                        {
                            icon: <CancelOutlinedIcon />,
                            info: "Invalid Password Errors:",
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
    }, [])

    return {votersSection}
}

export default useVotersStats
