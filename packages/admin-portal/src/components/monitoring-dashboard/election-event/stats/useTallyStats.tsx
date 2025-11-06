// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext, useMemo} from "react"
import CheckCircleOutlineIcon from "@mui/icons-material/CheckCircleOutline"
import CancelOutlinedIcon from "@mui/icons-material/CancelOutlined"
import HourglassEmptyOutlinedIcon from "@mui/icons-material/HourglassEmptyOutlined"
import {TransmissionStats, calcPrecentage} from "./ElectionEventStats"
import {AuthContext} from "@/providers/AuthContextProvider"
import {IPermissions} from "@/types/keycloak"
import {useTranslation} from "react-i18next"

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

    const {t} = useTranslation()
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
            title: t("monitoringDashboardScreen.tally.title"),
            stats: [
                {
                    show: showPostsStartCounting,
                    title: t("monitoringDashboardScreen.tally.activeVotesCounting"),
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
                    title: t("monitoringDashboardScreen.tally.generatedERs"),
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
                    title: t("monitoringDashboardScreen.tally.transmittedResults"),
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
    }, [
        showPostsStartCounting,
        showPostsGeneratedER,
        showPostsTransmittedResults,
        electionsCount,
        transmissionStats,
        showPostsTransmittedResults,
        notGenereatedTallyCount,
        genereatedTallyCount,
        showPostsGeneratedER,
        startCountingVotesCount,
        notStartCountingVotesCount,
    ])

    return {tallySection}
}

export default useTallyStats
