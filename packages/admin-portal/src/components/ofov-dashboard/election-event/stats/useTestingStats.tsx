// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext, useMemo} from "react"
import CheckCircleOutlineIcon from "@mui/icons-material/CheckCircleOutline"
import {VotingStats, calcPrecentage} from "./Stats"
import {AuthContext} from "@/providers/AuthContextProvider"
import {IPermissions} from "@/types/keycloak"

export interface StatsProps {
    enrolledVotersCount: number | string
    votingStats: VotingStats
}

const useTestingStats = (props: StatsProps) => {
    const {enrolledVotersCount, votingStats} = props

    const authContext = useContext(AuthContext)

    const showVoterVotedTestElection = authContext.isAuthorized(
        true,
        authContext.tenantId,
        IPermissions.MONITOR_VOTERS_VOTED_TEST_ELECTION
    )

    const testSection = useMemo(() => {
        return {
            show: showVoterVotedTestElection,
            title: "Testing",
            stats: [
                {
                    show: showVoterVotedTestElection,
                    title: "Total Voters who voted in the Test Election",
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
    }, [])

    return {testSection}
}

export default useTestingStats
