import React from "react"
import {TabbedShowLayout, useRecordContext} from "react-admin"
import {Sequent_Backend_Election} from "../../gql/graphql"
import ElectionHeader from "../../components/ElectionHeader"
import {EditElectionData} from "./ElectionData"
import DashboardElection from "@/components/dashboard/election/Dashboard"

export const ElectionTabs: React.FC = () => {
    const record = useRecordContext<Sequent_Backend_Election>()

    return (
        <>
            <ElectionHeader title={record?.name} subtitle="electionEventScreen.common.subtitle" />
            <TabbedShowLayout>
                <TabbedShowLayout.Tab label="Dashboard">
                    <DashboardElection />
                </TabbedShowLayout.Tab>
                <TabbedShowLayout.Tab label="Data">
                    <EditElectionData />
                </TabbedShowLayout.Tab>
                <TabbedShowLayout.Tab label="Voters">a</TabbedShowLayout.Tab>
                <TabbedShowLayout.Tab label="Publish">a</TabbedShowLayout.Tab>
                <TabbedShowLayout.Tab label="Logs">a</TabbedShowLayout.Tab>
            </TabbedShowLayout>
        </>
    )
}
