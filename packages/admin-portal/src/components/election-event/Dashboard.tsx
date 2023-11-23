import React from "react"
import {Box} from "@mui/material"
import {TextField} from "react-admin"
import {
    BarChart,
    ChartsContainer,
    ElectionStats,
    PieChart,
} from "../../resources/ElectionEvent/EditElectionEventDashboard"
import {ReportDialog} from "../ReportDialog"
import {BreadCrumbSteps} from "@sequentech/ui-essentials"

export default function DashboardElectionEvent() {
    return (
        <>
            <Box sx={{padding: "16px"}}>
                <Box sx={{padding: "12px 0"}}>
                    <BreadCrumbSteps
                        labels={[
                            "breadcrumbSteps.import",
                            "breadcrumbSteps.verify",
                            "breadcrumbSteps.finish",
                        ]}
                        selected={1}
                    />
                </Box>

                <TextField source="name" fontSize="24px" fontWeight="bold" />

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
