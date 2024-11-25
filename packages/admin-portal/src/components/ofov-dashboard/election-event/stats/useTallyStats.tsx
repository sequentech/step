// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext, useMemo} from "react"
import CheckCircleOutlineIcon from "@mui/icons-material/CheckCircleOutline"
import CancelOutlinedIcon from "@mui/icons-material/CancelOutlined"
import HourglassEmptyOutlinedIcon from "@mui/icons-material/HourglassEmptyOutlined"
import {TransmissionStats, calcPrecentage} from "./Stats"
import {AuthContext} from "@/providers/AuthContextProvider"
import {IPermissions} from "@/types/keycloak"

export interface StatsProps {
    electionsCount: number | string
    startCountingVotesCount: number | string
    notStartCountingVotesCount: number | string
    genereatedTallyCount: number | string
    notGenereatedTallyCount: number | string
    transmissionStats: TransmissionStats
}

const useTallyStats = (props: StatsProps) => {
    const {
        electionsCount,
        startCountingVotesCount,
        notStartCountingVotesCount,
        genereatedTallyCount,
        notGenereatedTallyCount,
        transmissionStats,
    } = props

    const authContext = useContext(AuthContext)

    const showPostsStartCounting = authContext.isAuthorized(
        true,
        authContext.tenantId,
        IPermissions.MONITOR_POSTS_ALREADY_STARTED_COUNTING_VOTES
    )

    const showPostsGeneratedER = authContext.isAuthorized(
        true,
        authContext.tenantId,
        IPermissions.MONITOR_POSTS_ALREADY_GENERATED_ELECTION_RESULTS
    )

    const showPostsTransmittedResults = authContext.isAuthorized(
        true,
        authContext.tenantId,
        IPermissions.MONITOR_POSTS_TRANSMITTED_RESULTS
    )

    const tallySection = useMemo(() => {
        return {
            show: showPostsStartCounting || showPostsGeneratedER || showPostsTransmittedResults,
            title: "Tally",
            stats: [
                {
                    show: showPostsStartCounting,
                    title: "Total Posts which already started counting votes",
                    items: [
                        {
                            icon: <CheckCircleOutlineIcon />,
                            count: startCountingVotesCount,
                            percentage: calcPrecentage(startCountingVotesCount, electionsCount),
                        },
                        {
                            icon: <CancelOutlinedIcon />,
                            count: notStartCountingVotesCount,
                            percentage: calcPrecentage(notStartCountingVotesCount, electionsCount),
                        },
                    ],
                },
                {
                    show: showPostsGeneratedER,
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
                {
                    show: showPostsTransmittedResults,
                    title: "Total Posts which transmitted results",
                    items: [
                        {
                            icon: <CheckCircleOutlineIcon />,
                            count: transmissionStats.transmittedCount,
                            percentage: calcPrecentage(
                                transmissionStats.transmittedCount,
                                electionsCount
                            ),
                        },
                        {
                            icon: <HourglassEmptyOutlinedIcon />,
                            count: transmissionStats.halfTransmittedCount,
                            percentage: calcPrecentage(
                                transmissionStats.halfTransmittedCount,
                                electionsCount
                            ),
                        },
                        {
                            icon: <CancelOutlinedIcon />,
                            count: transmissionStats.notTransmittedCount,
                            percentage: calcPrecentage(
                                transmissionStats.notTransmittedCount,
                                electionsCount
                            ),
                        },
                    ],
                },
            ],
        }
    }, [])

    return {tallySection}
}

export default useTallyStats
