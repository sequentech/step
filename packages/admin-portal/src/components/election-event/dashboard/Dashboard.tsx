import React from "react"
import {Box} from "@mui/material"

import {BreadCrumbSteps, BreadCrumbStepsVariant} from "@sequentech/ui-essentials"
import Stats from "./Stats"
import {VotesByDay, VotesByChannel} from "./Charts"
import styled from "@emotion/styled"

const Container = styled(Box)`
    display: flex;
    flex-wrap: wrap;
    gap: 24px;
`

export default function DashboardElectionEvent() {
    return (
        <>
            {" "}
            <Box sx={{maxWidth: 1258}}>
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

                <Box sx={{paddingX: "48px"}}>
                    <Stats />

                    <Container>
                        <VotesByDay />
                        <VotesByChannel />
                    </Container>
                </Box>
            </Box>
        </>
    )
}
