import React, {useContext} from "react"
import {TabbedShowLayout, useRecordContext} from "react-admin"
import {Sequent_Backend_Election_Event} from "../../gql/graphql"
import ElectionHeader from "../../components/ElectionHeader"
import {EditElectionEventData} from "./EditElectionEventData"
import DashboardElectionEvent from "../../components/election-event/Dashboard"
import {EditElectionEventAreas} from "./EditElectionEventAreas"
import {EditElectionEventUsers} from "./EditElectionEventUsers"
import {AuthContext} from "../../providers/AuthContextProvider"
import {IPermissions} from "../../types/keycloak"

export const ElectionEventTabs: React.FC = () => {
    const record = useRecordContext<Sequent_Backend_Election_Event>()
    const authContext = useContext(AuthContext)
    const showVoters = authContext.isAuthorized(true, authContext.tenantId, IPermissions.VOTER_READ)

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
