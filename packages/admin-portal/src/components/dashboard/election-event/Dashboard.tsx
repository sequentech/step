// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useEffect, useState} from "react"
import {Box, Button} from "@mui/material"
import {BreadCrumbSteps, BreadCrumbStepsVariant} from "@sequentech/ui-essentials"
import styled from "@emotion/styled"
import Stats from "../Stats"
import VotesByDay from "../charts/VoteByDay"
import VotesByChannel from "../charts/VoteByChannels"
import {Link} from "react-router-dom"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {Sequent_Backend_Election_Event} from "@/gql/graphql"
import {useRecordContext} from "react-admin"
import {EVotingStatus, IElectionEventStatus} from "@/types/CoreTypes"

const Container = styled(Box)`
    display: flex;
    flex-wrap: wrap;
    justify-content: space-between;
`

export default function DashboardElectionEvent() {
    const [tenantId] = useTenantStore()
    const record = useRecordContext<Sequent_Backend_Election_Event>()
    const [selected, setSelected] = useState(0)
    const cardWidth = 470
    const cardHeight = 300

    useEffect(() => {
        if (!record?.status) {
            return
        }
        const status = record.status as IElectionEventStatus
        let data: Array<number> = [0]
        if (status.keys_ceremony_finished) {
            data.push(1)
        }
        if (status.is_published) {
            data.push(2)
        }
        if ([EVotingStatus.OPEN, EVotingStatus.PAUSED].includes(status.voting_status)) {
            data.push(3)
        }
        if (EVotingStatus.CLOSED === status.voting_status) {
            data.push(4)
        }
        if (status.tally_ceremony_finished) {
            data.push(5)
        }
        setSelected(Math.max(...data))
    }, [record?.status])

    return (
        <>
            <Box sx={{width: 1024, marginX: "auto"}}>
                <BreadCrumbSteps
                    labels={[
                        "electionEventBreadcrumbSteps.created", // 0
                        "electionEventBreadcrumbSteps.keys", // 1
                        "electionEventBreadcrumbSteps.publish", // 2
                        "electionEventBreadcrumbSteps.started", // 3
                        "electionEventBreadcrumbSteps.ended", // 4
                        "electionEventBreadcrumbSteps.results", // 5
                    ]}
                    selected={selected}
                    variant={BreadCrumbStepsVariant.Circle}
                    colorPreviousSteps={true}
                />

                <Box>
                    <Stats forElection={true} />

                    <Container>
                        <VotesByDay width={cardWidth} height={cardHeight} />
                        <VotesByChannel
                            electionEventId={record.id}
                            width={cardWidth}
                            height={cardHeight}
                        />
                    </Container>
                </Box>
            </Box>
        </>
    )
}
