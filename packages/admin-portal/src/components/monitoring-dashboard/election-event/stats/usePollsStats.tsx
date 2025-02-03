// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext, useMemo} from "react"
import CheckCircleOutlineIcon from "@mui/icons-material/CheckCircleOutline"
import CancelOutlinedIcon from "@mui/icons-material/CancelOutlined"
import MarkEmailReadOutlinedIcon from "@mui/icons-material/MarkEmailReadOutlined"
import {VotingStats, calcPrecentage} from "./ElectionEventStats"
import {AuthContext} from "@/providers/AuthContextProvider"
import {IPermissions} from "@/types/keycloak"
import {useTranslation} from "react-i18next"

export interface PollsStatsProps {
    eligibleVotersCount: number | string
    enrolledVotersCount: number | string
    electionsCount: number | string
    startedVoteCount: number | string
    notStartedVotesCount: number | string
    openVotesCount: number | string
    notOpenVotesCount: number | string
    closedVotesCount: number | string
    notClosedVotesCount: number | string
    initializeCount: number | string
    notInitializeCount: number | string
    votingStats: VotingStats
}

const usePollsStats = (props: PollsStatsProps) => {
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
        initializeCount,
        notInitializeCount,
        votingStats,
    } = props

    const {t} = useTranslation()
    const authContext = useContext(AuthContext)

    const showTotalInitialized = authContext.isAuthorized(
        true,
        authContext.tenantId,
        IPermissions.MONITOR_POSTS_INITIALIZED_THE_SYSTEM
    )
    const showPostsOpenedVoting = authContext.isAuthorized(
        true,
        authContext.tenantId,
        IPermissions.MONITOR_POSTS_ALREADY_OPENED_VOTING
    )
    const showPostsClosedVoting = authContext.isAuthorized(
        true,
        authContext.tenantId,
        IPermissions.MONITOR_POSTS_ALREADY_CLOSED_VOTING
    )

    const showPostsStartVoting = authContext.isAuthorized(
        true,
        authContext.tenantId,
        IPermissions.MONITOR_POSTS_STARTED_VOTING
    )

    const showVotersVoted = authContext.isAuthorized(
        true,
        authContext.tenantId,
        IPermissions.MONITOR_VOTERS_WHO_VOTED
    )

    const pollsSection = useMemo(() => {
        return {
            show:
                showTotalInitialized ||
                showPostsOpenedVoting ||
                showPostsClosedVoting ||
                showPostsStartVoting ||
                showVotersVoted,
            title: t("monitoringDashboardScreen.polls.title"),
            stats: [
                {
                    show: showTotalInitialized,
                    title: t("monitoringDashboardScreen.polls.initializedSystems"),
                    items: [
                        {
                            show: showTotalInitialized,
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
                {
                    show: showPostsOpenedVoting,
                    title: t("monitoringDashboardScreen.polls.votingOpened"),
                    items: [
                        {
                            icon: <CheckCircleOutlineIcon />,
                            count: openVotesCount,
                            percentage: calcPrecentage(openVotesCount, electionsCount),
                        },
                        {
                            icon: <CancelOutlinedIcon />,
                            count: notOpenVotesCount,
                            percentage: calcPrecentage(notOpenVotesCount, electionsCount),
                        },
                    ],
                },
                {
                    show: showPostsClosedVoting,
                    title: t("monitoringDashboardScreen.polls.votingClosed"),
                    items: [
                        {
                            icon: <CheckCircleOutlineIcon />,
                            count: closedVotesCount,
                            percentage: calcPrecentage(closedVotesCount, electionsCount),
                        },
                        {
                            icon: <CancelOutlinedIcon />,
                            count: notClosedVotesCount,
                            percentage: calcPrecentage(notClosedVotesCount, electionsCount),
                        },
                    ],
                },
                {
                    show: showPostsStartVoting,
                    title: t("monitoringDashboardScreen.polls.votingStarted"),
                    items: [
                        {
                            icon: <CheckCircleOutlineIcon />,
                            count: startedVoteCount,
                            percentage: calcPrecentage(startedVoteCount, electionsCount),
                        },
                        {
                            icon: <CancelOutlinedIcon />,
                            count: notStartedVotesCount,
                            percentage: calcPrecentage(notStartedVotesCount, electionsCount),
                        },
                    ],
                },
                {
                    show: showVotersVoted,
                    title: t("monitoringDashboardScreen.polls.voterTurnout"),
                    items: [
                        {
                            icon: <MarkEmailReadOutlinedIcon />,
                            count: votingStats.votedCount,
                            percentage: calcPrecentage(votingStats.votedCount, eligibleVotersCount),
                        },
                    ],
                },
            ],
        }
    }, [
        showTotalInitialized,
        showPostsOpenedVoting,
        showPostsClosedVoting,
        showPostsStartVoting,
        showVotersVoted,
        votingStats,
        eligibleVotersCount,
        electionsCount,
        startedVoteCount,
        notStartedVotesCount,
        openVotesCount,
        notOpenVotesCount,
        closedVotesCount,
        notClosedVotesCount,
        initializeCount,
        notInitializeCount,
    ])

    return {pollsSection}
}

export default usePollsStats
