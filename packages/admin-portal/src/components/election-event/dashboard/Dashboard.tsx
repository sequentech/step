import React from "react"
import {Box} from "@mui/material"

import {
    BarChart,
    ChartsContainer,
    ElectionStats,
    PieChart,
} from "@/resources/ElectionEvent/EditElectionEventDashboard"
import {BreadCrumbSteps, BreadCrumbStepsVariant} from "@sequentech/ui-essentials"

export default function DashboardElectionEvent() {
    return (
        <>
            <Box sx={{padding: "16px"}}>
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

                <ElectionStats />

                <ChartsContainer>
                    <BarChart />
                    <PieChart />
                </ChartsContainer>
            </Box>
        </>
    )
}
