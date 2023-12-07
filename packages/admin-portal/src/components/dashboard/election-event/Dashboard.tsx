// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
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

const Container = styled(Box)`
    display: flex;
    flex-wrap: wrap;
    justify-content: space-between;
`

export default function DashboardElectionEvent() {
    const [tenantId] = useTenantStore()
    const record = useRecordContext<Sequent_Backend_Election_Event>()
    const cardWidth = 470
    const cardHeight = 300

    return (
        <>
            <Box sx={{width: 1024, marginX: "auto"}}>
                <BreadCrumbSteps
                    labels={[
                        "electionEventBreadcrumbSteps.created",
                        "electionEventBreadcrumbSteps.keys",
                        "electionEventBreadcrumbSteps.publish",
                        "electionEventBreadcrumbSteps.started",
                        "electionEventBreadcrumbSteps.ended",
                        "electionEventBreadcrumbSteps.results",
                    ]}
                    selected={1}
                    variant={BreadCrumbStepsVariant.Circle}
                    colorPreviousSteps={true}
                />

                <Box>
                    <Stats forElection={true} />
                    <Link
                        to={`http://localhost:3000/tenant/${tenantId}/event/${record.id}/login`}
                        target="#"
                    >
                        <Button>Vote</Button>
                    </Link>

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
