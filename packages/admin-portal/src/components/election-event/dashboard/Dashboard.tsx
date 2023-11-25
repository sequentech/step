import React from "react"
import {Box} from "@mui/material"

import {
    BarChart,
    ChartsContainer,
    PieChart,
} from "@/resources/ElectionEvent/EditElectionEventDashboard"
import {BreadCrumbSteps, BreadCrumbStepsVariant} from "@sequentech/ui-essentials"

import ElectionStats from "./ElectionStats"

export default function DashboardElectionEvent() {
    return (
        <>
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
                    <ElectionStats />

                    <ChartsContainer>
                        <BarChart />
                        <PieChart />
                    </ChartsContainer>
                </Box>
            </Box>
        </>
    )
}
