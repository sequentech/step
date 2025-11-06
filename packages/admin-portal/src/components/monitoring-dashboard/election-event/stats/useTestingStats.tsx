// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext, useMemo} from "react"
import CheckCircleOutlineIcon from "@mui/icons-material/CheckCircleOutline"
import {VotingStats, calcPrecentage} from "./ElectionEventStats"
import {AuthContext} from "@/providers/AuthContextProvider"
import {IPermissions} from "@/types/keycloak"
import {useTranslation} from "react-i18next"

export interface StatsProps {
    enrolledVotersCount: number | string
    votingStats: VotingStats
}

const useTestingStats = (props: StatsProps) => {
    const {enrolledVotersCount, votingStats} = props

    const {t} = useTranslation()
    const authContext = useContext(AuthContext)

    const showVoterVotedTestElection = authContext.isAuthorized(
        true,
        authContext.tenantId,
        IPermissions.MONITOR_VOTERS_VOTED_TEST_ELECTION
    )

    const testSection = useMemo(() => {
        return {
            show: showVoterVotedTestElection,
            title: t("monitoringDashboardScreen.testing.title"),
            stats: [
                {
                    show: showVoterVotedTestElection,
                    title: t("monitoringDashboardScreen.testing.testElectionVoterCount"),
                    items: [
                        {
                            icon: <CheckCircleOutlineIcon />,
                            count: votingStats.votedTestsElectionsCount,
                            percentage: calcPrecentage(
                                votingStats.votedTestsElectionsCount,
                                enrolledVotersCount
                            ),
                        },
                    ],
                },
            ],
        }
    }, [showVoterVotedTestElection, votingStats, enrolledVotersCount])

    return {testSection}
}

export default useTestingStats
