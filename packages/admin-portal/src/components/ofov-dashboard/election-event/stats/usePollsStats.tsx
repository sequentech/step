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

export interface PollsStatsProps {
    eligibleVotersCount: number | string
    enrolledVotersCount: number | string
    electionsCount: number | string
    openVotesCount: number | string
    notOpenedVotesCount: number | string
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
        openVotesCount,
        notOpenedVotesCount,
        closedVotesCount,
        notClosedVotesCount,
        initializeCount,
        notInitializeCount,
        votingStats,
    } = props

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
            title: "Polls",
            stats: [
                {
                    show: showTotalInitialized,
                    title: "Total Posts initialized the system",
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
                {
                    show: showPostsClosedVoting,
                    title: "Total Posts which already closed voting",
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
                    title: "Total Posts which started voting",
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
                {
                    show: showVotersVoted,
                    title: "Total Voters who voted",
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
    }, [])

    return {pollsSection}
}

export default usePollsStats
