import React from "react"
import {Box} from "@mui/material"

import {
    BarChart,
    ChartsContainer,
    ElectionStats,
    PieChart,
} from "../../resources/ElectionEvent/EditElectionEventDashboard"
import {ReportDialog} from "../ReportDialog"
import {BreadCrumbSteps, BreadCrumbStepsVariant} from "@sequentech/ui-essentials"

export default function DashboardElectionEvent() {
    return (
        <>
            <Box sx={{padding: "16px"}}>
                <BreadCrumbSteps
                    labels={["created", "keys", "publish", "started", "ended", "results"]}
                    selected={1}
                    variant={BreadCrumbStepsVariant.Circle}
                    colorPreviousSteps={true}
                />

                <ElectionStats />

                <ChartsContainer>
                    <BarChart />
                    <PieChart />
                </ChartsContainer>

                <ReportDialog />
            </Box>
        </>
    )
}
