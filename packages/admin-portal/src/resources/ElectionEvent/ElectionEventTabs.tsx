import React, {useContext} from "react"
import {TabbedShowLayout, useRecordContext} from "react-admin"
import {Sequent_Backend_Election_Event} from "../../gql/graphql"
import ElectionHeader from "../../components/ElectionHeader"
import {EditElectionEventData} from "./EditElectionEventData"
import DashboardElectionEvent from "@/components/dashboard/election-event/Dashboard"
import {EditElectionEventAreas} from "./EditElectionEventAreas"
import {EditElectionEventUsers} from "./EditElectionEventUsers"
import {AuthContext} from "../../providers/AuthContextProvider"
import {IPermissions} from "../../types/keycloak"
import {EditElectionEventKeys} from "./EditElectionEventKeys"
import {EditElectionEventTally} from "./EditElectionEventTally"
import {EditElectionEventPublish} from "./EditElectionEventPublish"
import {useTranslation} from "react-i18next"
import {
    ElectionEventTallyContextProvider,
    useElectionEventTallyStore,
} from "@/providers/ElectionEventTallyProvider"

export const ElectionEventTabs: React.FC = () => {
    const record = useRecordContext<Sequent_Backend_Election_Event>()
    const authContext = useContext(AuthContext)
    const showVoters = authContext.isAuthorized(true, authContext.tenantId, IPermissions.VOTER_READ)
    const {t} = useTranslation()
    const [_, setTallyId] = useElectionEventTallyStore()

    return (
        <>
            <ElectionHeader title={record?.name} subtitle="electionEventScreen.common.subtitle" />
            <TabbedShowLayout>
                <TabbedShowLayout.Tab label={t("electionEventScreen.tabs.dashboard")}>
                    <DashboardElectionEvent />
                </TabbedShowLayout.Tab>
                <TabbedShowLayout.Tab label={t("electionEventScreen.tabs.data")}>
                    <EditElectionEventData />
                </TabbedShowLayout.Tab>
                {showVoters ? (
                    <TabbedShowLayout.Tab label={t("electionEventScreen.tabs.voters")}>
                        <EditElectionEventUsers />
                    </TabbedShowLayout.Tab>
                ) : null}
                <TabbedShowLayout.Tab label={t("electionEventScreen.tabs.areas")}>
                    <EditElectionEventAreas />
                </TabbedShowLayout.Tab>
                <TabbedShowLayout.Tab label={t("electionEventScreen.tabs.keys")}>
                    <EditElectionEventKeys />
                </TabbedShowLayout.Tab>
                <TabbedShowLayout.Tab
                    label={t("electionEventScreen.tabs.tally")}
                    onClick={() => {
                        setTallyId(null)
                    }}
                >
                    <EditElectionEventTally />
                </TabbedShowLayout.Tab>
                <TabbedShowLayout.Tab label={t("electionEventScreen.tabs.publish")}>
                    <EditElectionEventPublish />
                </TabbedShowLayout.Tab>
                <TabbedShowLayout.Tab label={t("electionEventScreen.tabs.logs")}>
                    a
                </TabbedShowLayout.Tab>
            </TabbedShowLayout>
        </>
    )
}
