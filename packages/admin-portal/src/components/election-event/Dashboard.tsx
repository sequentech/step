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

export default function DashboardElectionEvent() {
    return (
        <>
            <Box sx={{padding: "16px"}}>
                <TextField source="name" fontSize="24px" fontWeight="bold" />

                <ElectionStats />

                <ChartsContainer>
                    <BarChart />
                    <PieChart />
                </ChartsContainer>

                <ReportDialog />
            </Box>

            <div>hello</div>
        </>
    )
}
