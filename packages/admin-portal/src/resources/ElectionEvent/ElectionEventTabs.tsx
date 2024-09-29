// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext, useEffect} from "react"
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
import {useTranslation} from "react-i18next"
import {useElectionEventTallyStore} from "@/providers/ElectionEventTallyProvider"
import {useLocation, useNavigate} from "react-router"
import {Publish} from "@/resources/Publish/Publish"
import {EPublishType} from "../Publish/EPublishType"
import {ElectoralLog} from "./ElectoralLog"
import EditElectionEventTextData from "./EditElectionEventTextData"
import {v4 as uuidv4} from "uuid"
import {EditElectionEventTasks} from "./EditElectionEventTasks"
import EditEvents from "./EditEvents"
import {EditElectionEvents} from "../Events/EditElectionEvents"

export const ElectionEventTabs: React.FC = () => {
    const record = useRecordContext<Sequent_Backend_Election_Event>()
    const authContext = useContext(AuthContext)
    const showVoters = authContext.isAuthorized(true, authContext.tenantId, IPermissions.VOTER_READ)
    const [showKeysList, setShowKeysList] = React.useState<string | null>(null)
    const [tabKey, setTabKey] = React.useState<string>(uuidv4())
    const location = useLocation()
    const navigate = useNavigate()

    const refreshRef = React.useRef<HTMLButtonElement>()

    const showDashboard = authContext.isAuthorized(
        true,
        authContext.tenantId,
        IPermissions.ADMIN_DASHBOARD_VIEW
    )
    const showData = authContext.isAuthorized(
        true,
        authContext.tenantId,
        IPermissions.ELECTION_EVENT_WRITE
    )
    const showTextData = authContext.isAuthorized(
        true,
        authContext.tenantId,
        IPermissions.ELECTION_EVENT_WRITE
    )
    const showAreas = authContext.isAuthorized(true, authContext.tenantId, IPermissions.AREA_READ)
    const showKeys = authContext.isAuthorized(true, authContext.tenantId, [
        IPermissions.ADMIN_CEREMONY,
        IPermissions.TRUSTEE_CEREMONY,
    ])
    const showTally = authContext.isAuthorized(true, authContext.tenantId, [
        IPermissions.TALLY_READ,
        IPermissions.TALLY_START,
    ])
    const showPublish = authContext.isAuthorized(
        true,
        authContext.tenantId,
        IPermissions.PUBLISH_READ
    )
    const showLogs = authContext.isAuthorized(true, authContext.tenantId, IPermissions.LOGS_READ)

    const showTasksExecution = authContext.isAuthorized(
        true,
        authContext.tenantId,
        IPermissions.TASKS_READ
    )
    const showEvents = authContext.isAuthorized(
        true,
        authContext.tenantId,
        IPermissions.EVENTS_READ
    )
    const {t} = useTranslation()
    const {setTallyId, setCreatingFlag, setSelectedTallySessionData} = useElectionEventTallyStore()

    useEffect(() => {
        const locArr = location.pathname.split("/").slice(0, 3).join("/")
        navigate(locArr)
    }, [])

    // code to refresh the dashboard when the user navigates to it
    // the ui has to wait for the children to be mounted before refreshing via ref click
    const [loadedChildren, setLoadedChildren] = React.useState<number>(0)
    const handleChildMount = () => {
        setLoadedChildren((prev) => (prev < 2 ? prev + 1 : prev))
    }
    useEffect(() => {
        if (loadedChildren === 1 || loadedChildren === 2) {
            refreshRef.current?.click()
        }
    }, [loadedChildren])
    // end of code to refresh the dashboard when the user navigates to it

    return (
        <>
            <ElectionHeader title={record?.name} subtitle="electionEventScreen.common.subtitle" />
            <TabbedShowLayout>
                {showDashboard ? (
                    <TabbedShowLayout.Tab
                        label={t("electionEventScreen.tabs.dashboard")}
                        className="election-event-dashboard-tab"
                        onClick={() => {
                            setLoadedChildren(0)
                        }}
                    >
                        <DashboardElectionEvent
                            refreshRef={refreshRef}
                            onMount={handleChildMount}
                        />
                    </TabbedShowLayout.Tab>
                ) : null}
                {showData ? (
                    <TabbedShowLayout.Tab
                        label={t("electionEventScreen.tabs.data")}
                        className="election-event-data-tab"
                    >
                        <EditElectionEventData />
                    </TabbedShowLayout.Tab>
                ) : null}
                {showTextData ? (
                    <TabbedShowLayout.Tab label={t("electionEventScreen.tabs.localization")}>
                        <EditElectionEventTextData />
                    </TabbedShowLayout.Tab>
                ) : null}
                {showVoters ? (
                    <TabbedShowLayout.Tab
                        label={t("electionEventScreen.tabs.voters")}
                        className="election-event-voter-tab"
                    >
                        <EditElectionEventUsers electionEventId={record?.id} />
                    </TabbedShowLayout.Tab>
                ) : null}
                {showAreas ? (
                    <TabbedShowLayout.Tab
                        label={t("electionEventScreen.tabs.areas")}
                        className="election-event-area-tab"
                    >
                        <EditElectionEventAreas />
                    </TabbedShowLayout.Tab>
                ) : null}
                {showKeys ? (
                    <TabbedShowLayout.Tab
                        label={t("electionEventScreen.tabs.keys")}
                        className="election-keys-tab"
                        onClick={() => {
                            setShowKeysList(Date.now().toString())
                        }}
                    >
                        <EditElectionEventKeys
                            isShowCeremony={showKeysList}
                            isShowTrusteeCeremony={showKeysList}
                        />
                    </TabbedShowLayout.Tab>
                ) : null}
                {showTally ? (
                    <TabbedShowLayout.Tab
                        label={t("electionEventScreen.tabs.tally")}
                        className="election-event-tally-tab"
                        onClick={() => {
                            setTallyId(null)
                            setCreatingFlag(false)
                            setSelectedTallySessionData(null)
                        }}
                    >
                        <EditElectionEventTally />
                    </TabbedShowLayout.Tab>
                ) : null}
                {showPublish ? (
                    <TabbedShowLayout.Tab
                        label={t("electionEventScreen.tabs.publish")}
                        onClick={() => setTabKey(uuidv4())}
                        className="election-event-publish-tab"
                    >
                        <Publish electionEventId={record?.id} type={EPublishType.Event} />
                    </TabbedShowLayout.Tab>
                ) : null}
                {showTasksExecution ? (
                    <TabbedShowLayout.Tab label={t("electionEventScreen.tabs.tasks")}>
                        <EditElectionEventTasks />
                    </TabbedShowLayout.Tab>
                ) : null}
                {showLogs ? (
                    <TabbedShowLayout.Tab
                        label={t("electionEventScreen.tabs.logs")}
                        className="election-event-logs-tab"
                    >
                        <ElectoralLog />
                    </TabbedShowLayout.Tab>
                ) : null}
                {showEvents ? (
                    <TabbedShowLayout.Tab label={"Events"}>
                        <EditElectionEvents electionEventId={record?.id} />
                    </TabbedShowLayout.Tab>
                ) : null}
            </TabbedShowLayout>
        </>
    )
}
function userRef() {
    throw new Error("Function not implemented.")
}
