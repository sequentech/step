import React, {useContext} from "react"
import {TabbedShowLayout, TextField, useRecordContext} from "react-admin"
import {Sequent_Backend_Election_Event} from "../../gql/graphql"
import ElectionHeader from "../../components/ElectionHeader"
import {Box} from "@mui/material"
import {EditElectionEventData} from "./EditElectionEventData"
import {BarChart, ChartsContainer, ElectionStats, PieChart} from "./EditElectionEventDashboard"
import {ReportDialog} from "../../components/ReportDialog"
import {EditElectionEventAreas} from "./EditElectionEventAreas"
import {EditElectionEventUsers} from "./EditElectionEventUsers"
import {AuthContext} from "../../providers/AuthContextProvider"

export const ElectionEventTabs: React.FC = () => {
    const record = useRecordContext<Sequent_Backend_Election_Event>()
    const authContext = useContext(AuthContext)
    const showVoters = authContext.hasPermissions(false, authContext.tenantId, "read-event-users")

    return (
        <>
            <ElectionHeader title={record?.name} subtitle="electionEventScreen.common.subtitle" />
            <TabbedShowLayout>
                <TabbedShowLayout.Tab label="Dashboard">
                    <Box sx={{padding: "16px"}}>
                        <TextField source="name" fontSize="24px" fontWeight="bold" />
                        <ElectionStats />
                        <ChartsContainer>
                            <BarChart />
                            <PieChart />
                        </ChartsContainer>
                        <ReportDialog />
                    </Box>
                </TabbedShowLayout.Tab>
                <TabbedShowLayout.Tab label="Data">
                    <EditElectionEventData />
                </TabbedShowLayout.Tab>
                {showVoters ? (
                    <TabbedShowLayout.Tab label="Voters">
                        <EditElectionEventUsers />
                    </TabbedShowLayout.Tab>
                ) : null}
                <TabbedShowLayout.Tab label="Areas">
                    <EditElectionEventAreas />
                </TabbedShowLayout.Tab>
                <TabbedShowLayout.Tab label="Keys">a</TabbedShowLayout.Tab>
                <TabbedShowLayout.Tab label="Tally">a</TabbedShowLayout.Tab>
                <TabbedShowLayout.Tab label="Publish">a</TabbedShowLayout.Tab>
                <TabbedShowLayout.Tab label="Logs">a</TabbedShowLayout.Tab>
            </TabbedShowLayout>
        </>
    )
}
