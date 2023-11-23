import React from "react"
import {TabbedShowLayout, useRecordContext} from "react-admin"
import {Sequent_Backend_Election_Event} from "../../gql/graphql"
import ElectionHeader from "../../components/ElectionHeader"
import {EditElectionEventData} from "./EditElectionEventData"
import {EditElectionEventAreas} from "./EditElectionEventAreas"
import DashboardElectionEvent from "../../components/election-event/Dashboard"

export const ElectionEventTabs: React.FC = () => {
    const record = useRecordContext<Sequent_Backend_Election_Event>()

    return (
        <>
            <ElectionHeader title={record?.name} subtitle="electionEventScreen.common.subtitle" />
            <TabbedShowLayout>
                <TabbedShowLayout.Tab label="Dashboard">
                    <DashboardElectionEvent />
                </TabbedShowLayout.Tab>
                <TabbedShowLayout.Tab label="Data">
                    <EditElectionEventData />
                </TabbedShowLayout.Tab>
                <TabbedShowLayout.Tab label="Voters">a</TabbedShowLayout.Tab>
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
