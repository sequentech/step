import React, {useContext} from "react"
import {TabbedShowLayout, useRecordContext} from "react-admin"
import {Sequent_Backend_Election_Event} from "@/gql/graphql"
import ElectionHeader from "@/components/ElectionHeader"
import {EditElectionEventData} from "./EditElectionEventData"
import DashboardElectionEvent from "@/components/dashboard/election-event/Dashboard"
import {EditElectionEventAreas} from "./EditElectionEventAreas"
import {EditElectionEventUsers} from "./EditElectionEventUsers"
import {AuthContext} from "@/providers/AuthContextProvider"
import {IPermissions} from "@/types/keycloak"
import {EditElectionEventKeys} from "./EditElectionEventKeys"
import {EditElectionEventTally} from "./EditElectionEventTally"
import {EditElectionEventPublish} from "./EditElectionEventPublish"
import {useTranslation} from "react-i18next"

export const ElectionEventTabs: React.FC = () => {
    const record = useRecordContext<Sequent_Backend_Election_Event>()
    const authContext = useContext(AuthContext)
    const showVoters = authContext.isAuthorized(
        true, authContext.tenantId, IPermissions.VOTER_READ
    )
    const showDashboard = authContext.isAuthorized(
        true, authContext.tenantId, IPermissions.ADMIN_DASHBOARD_VIEW
    )
    const showData = authContext.isAuthorized(
        true, authContext.tenantId, IPermissions.ELECTION_EVENT_READ
    )
    const showAreas = authContext.isAuthorized(
        true, authContext.tenantId, IPermissions.AREA_READ
    )
    const showKeys = authContext.isAuthorized(
        true,
        authContext.tenantId,
        [IPermissions.ADMIN_CEREMONY, IPermissions.TRUSTEE_READ]
    )
    const showTally = authContext.isAuthorized(
        true,
        authContext.tenantId,
        [IPermissions.TALLY_READ, IPermissions.TALLY_START]
    )
    const showPublish = authContext.isAuthorized(
        true, authContext.tenantId, IPermissions.PUBLISH_READ
    )
    const showLogs = authContext.isAuthorized(
        true, authContext.tenantId, IPermissions.LOGS_READ
    )
    const {t} = useTranslation()

    return (
        <>
            <ElectionHeader title={record?.name} subtitle="electionEventScreen.common.subtitle" />
            <TabbedShowLayout>
                {showDashboard
                    ? <TabbedShowLayout.Tab label={t("electionEventScreen.tabs.dashboard")}>
                        <DashboardElectionEvent />
                    </TabbedShowLayout.Tab>
                    : null}
                {showData
                    ? <TabbedShowLayout.Tab label={t("electionEventScreen.tabs.data")}>
                        <EditElectionEventData />
                    </TabbedShowLayout.Tab>
                    : null}
                {showVoters
                    ? <TabbedShowLayout.Tab label={t("electionEventScreen.tabs.voters")}>
                        <EditElectionEventUsers />
                    </TabbedShowLayout.Tab>
                    : null}
                {showAreas
                    ? <TabbedShowLayout.Tab label={t("electionEventScreen.tabs.areas")}>
                        <EditElectionEventAreas />
                    </TabbedShowLayout.Tab>
                    : null}
                {showKeys
                    ? <TabbedShowLayout.Tab label={t("electionEventScreen.tabs.keys")}>
                        <EditElectionEventKeys />
                    </TabbedShowLayout.Tab>
                    : null}
                {showTally
                    ? <TabbedShowLayout.Tab label={t("electionEventScreen.tabs.tally")}>
                        <EditElectionEventTally />
                    </TabbedShowLayout.Tab>
                    : null}
                {showPublish
                    ? <TabbedShowLayout.Tab label={t("electionEventScreen.tabs.publish")}>
                        <EditElectionEventPublish />
                    </TabbedShowLayout.Tab>
                    : null}
                {showLogs
                    ? <TabbedShowLayout.Tab label={t("electionEventScreen.tabs.logs")}>
                        a
                    </TabbedShowLayout.Tab>
                    : null}
            </TabbedShowLayout>
        </>
    )
}
