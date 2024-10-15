// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext, useEffect, Suspense, lazy} from "react"
import {TabbedShowLayout, useRecordContext} from "react-admin"
import {Sequent_Backend_Election_Event} from "@/gql/graphql"
import ElectionHeader from "@/components/ElectionHeader"
import {AuthContext} from "@/providers/AuthContextProvider"
import {IPermissions} from "@/types/keycloak"
import {useTranslation} from "react-i18next"
import {useElectionEventTallyStore} from "@/providers/ElectionEventTallyProvider"
import {useLocation, useNavigate} from "react-router"
import {v4 as uuidv4} from "uuid"
import {Box, Tabs, Tab} from "@mui/material"
import {EPublishType} from "../Publish/EPublishType"
import {EElectionEventLockedDown} from "@sequentech/ui-core"

// Lazy load the tab components
const DashboardElectionEvent = lazy(() => import("@/components/dashboard/election-event/Dashboard"))
const EditElectionEventData = lazy(() =>
    import("./EditElectionEventData").then((module) => ({default: module.EditElectionEventData}))
)
const EditElectionEventTextData = lazy(() =>
    import("./EditElectionEventTextData").then((module) => ({default: module.default}))
)
const EditElectionEventUsers = lazy(() =>
    import("./EditElectionEventUsers").then((module) => ({default: module.EditElectionEventUsers}))
)
const EditElectionEventAreas = lazy(() =>
    import("./EditElectionEventAreas").then((module) => ({default: module.EditElectionEventAreas}))
)
const EditElectionEventKeys = lazy(() =>
    import("./EditElectionEventKeys").then((module) => ({default: module.EditElectionEventKeys}))
)
const EditElectionEventTally = lazy(() =>
    import("./EditElectionEventTally").then((module) => ({default: module.EditElectionEventTally}))
)
const Publish = lazy(() =>
    import("@/resources/Publish/Publish").then((module) => ({default: module.Publish}))
)
const ElectoralLog = lazy(() =>
    import("./ElectoralLog").then((module) => ({default: module.ElectoralLog}))
)
const EditElectionEventTasks = lazy(() =>
    import("./EditElectionEventTasks").then((module) => ({default: module.EditElectionEventTasks}))
)
const EditElectionEventEvents = lazy(() =>
    import("./EditElectionEventScheduledEvents").then((module) => ({
        default: module.EditElectionEventEvents,
    }))
)
const EditNotifications = lazy(() =>
    import("../Notifications/EditNotifications").then((module) => ({
        default: module.EditNotifications,
    }))
)

const Reports = lazy(() =>
    import("../Reports/EditReportsTab").then((module) => ({
        default: module.EditReportsTab,
    }))
)

export const ElectionEventTabs: React.FC = () => {
    const record = useRecordContext<Sequent_Backend_Election_Event>()
    const authContext = useContext(AuthContext)
    const showVoters = authContext.isAuthorized(true, authContext.tenantId, IPermissions.VOTER_READ)
    const [showKeysList, setShowKeysList] = React.useState<string | null>(null)
    const [tabKey, setTabKey] = React.useState<string>(uuidv4())
    const location = useLocation()
    const navigate = useNavigate()
    const refreshRef = React.useRef<HTMLButtonElement>()
    const {t} = useTranslation()
    const {setTallyId, setCreatingFlag, setSelectedTallySessionData} = useElectionEventTallyStore()
    const isElectionEventLocked =
        record?.presentation?.locked_down == EElectionEventLockedDown.LOCKED_DOWN

    const showDashboard = authContext.isAuthorized(
        true,
        authContext.tenantId,
        IPermissions.ADMIN_DASHBOARD_VIEW
    )
    const showData =
        !isElectionEventLocked &&
        authContext.isAuthorized(true, authContext.tenantId, IPermissions.ELECTION_EVENT_WRITE)
    const showTextData =
        !isElectionEventLocked &&
        authContext.isAuthorized(true, authContext.tenantId, IPermissions.ELECTION_EVENT_WRITE)
    const showAreas =
        !isElectionEventLocked &&
        authContext.isAuthorized(true, authContext.tenantId, IPermissions.AREA_READ)
    const showKeys =
        !isElectionEventLocked &&
        authContext.isAuthorized(true, authContext.tenantId, [
            IPermissions.ADMIN_CEREMONY,
            IPermissions.TRUSTEE_CEREMONY,
        ])
    const showTally =
        !isElectionEventLocked &&
        authContext.isAuthorized(true, authContext.tenantId, [
            IPermissions.TALLY_READ,
            IPermissions.TALLY_START,
        ])
    const showPublish =
        !isElectionEventLocked &&
        authContext.isAuthorized(true, authContext.tenantId, IPermissions.PUBLISH_READ)
    const showLogs = authContext.isAuthorized(true, authContext.tenantId, IPermissions.LOGS_READ)
    const showTasksExecution =
        !isElectionEventLocked &&
        authContext.isAuthorized(true, authContext.tenantId, IPermissions.TASKS_READ)
    const showEvents =
        !isElectionEventLocked &&
        authContext.isAuthorized(true, authContext.tenantId, IPermissions.SCHEDULED_EVENT_WRITE)
    const showNotifications = authContext.isAuthorized(
        true,
        authContext.tenantId,
        IPermissions.NOTIFICATION_READ
    )

    const showReports = authContext.isAuthorized(
        true,
        authContext.tenantId,
        IPermissions.REPORT_READ
    )

    const [loadedChildren, setLoadedChildren] = React.useState<number>(0)
    const [value, setValue] = React.useState(0)

    const handleChange = (event: React.SyntheticEvent, newValue: number) => {
        setValue(newValue)
    }

    useEffect(() => {
        const locArr = location.pathname.split("/").slice(0, 3).join("/")
        navigate(locArr)
    }, [location.pathname, navigate])

    // Code to refresh the dashboard when the user navigates to it
    const handleChildMount = () => {
        setLoadedChildren((prev) => (prev < 2 ? prev + 1 : prev))
    }

    useEffect(() => {
        if (loadedChildren === 1 || loadedChildren === 2) {
            refreshRef.current?.click()
        }
    }, [loadedChildren])

    // This useEffect handles the 'tabIndex' search parameter from the URL.
    // It reads the parameter, parses it, and sets the active tab based on the index.
    // If the 'tabIndex' parameter is present and valid, the corresponding tab will be selected.
    useEffect(() => {
        const params = new URLSearchParams(location.search)
        const tabIndexParam = params.get("tabIndex")

        if (tabIndexParam) {
            const tabIndex = parseInt(tabIndexParam, 10)
            if (!isNaN(tabIndex)) {
                setValue(tabIndex)
            }
        }
    }, [location.search])

    const renderTabContent = () => {
        switch (value) {
            case 0:
                return showDashboard ? (
                    <Suspense fallback={<div>Loading Dashboard...</div>}>
                        <DashboardElectionEvent
                            refreshRef={refreshRef}
                            onMount={handleChildMount}
                        />
                    </Suspense>
                ) : null
            case 1:
                return showData ? (
                    <Suspense fallback={<div>Loading Data...</div>}>
                        <EditElectionEventData />
                    </Suspense>
                ) : null
            case 2:
                return showTextData ? (
                    <Suspense fallback={<div>Loading Text Data...</div>}>
                        <EditElectionEventTextData />
                    </Suspense>
                ) : null
            case 3:
                return showVoters ? (
                    <Suspense fallback={<div>Loading Voters...</div>}>
                        <EditElectionEventUsers electionEventId={record?.id} />
                    </Suspense>
                ) : null
            case 4:
                return showAreas ? (
                    <Suspense fallback={<div>Loading Areas...</div>}>
                        <EditElectionEventAreas />
                    </Suspense>
                ) : null
            case 5:
                return showKeys ? (
                    <Suspense fallback={<div>Loading Keys...</div>}>
                        <EditElectionEventKeys
                            isShowCeremony={showKeysList}
                            isShowTrusteeCeremony={showKeysList}
                        />
                    </Suspense>
                ) : null
            case 6:
                return showTally ? (
                    <Suspense fallback={<div>Loading Tally...</div>}>
                        <EditElectionEventTally />
                    </Suspense>
                ) : null
            case 7:
                return showPublish && record?.id ? (
                    <Suspense fallback={<div>Loading Publish...</div>}>
                        <Publish electionEventId={record?.id} type={EPublishType.Event} />
                    </Suspense>
                ) : null
            case 8:
                return showTasksExecution ? (
                    <Suspense fallback={<div>Loading Tasks...</div>}>
                        <EditElectionEventTasks />
                    </Suspense>
                ) : null
            case 9:
                return showLogs ? (
                    <Suspense fallback={<div>Loading Logs...</div>}>
                        <ElectoralLog />
                    </Suspense>
                ) : null
            case 10:
                return showEvents ? (
                    <Suspense fallback={<div>Loading Events...</div>}>
                        <EditElectionEventEvents electionEventId={record?.id} />
                    </Suspense>
                ) : null
            case 11:
                return showNotifications ? (
                    <Suspense fallback={<div>Loading Notifications...</div>}>
                        <EditNotifications electionEventId={record?.id} />
                    </Suspense>
                ) : null
            case 12:
                return showReports ? (
                    <Suspense fallback={<div>Loading Reports...</div>}>
                        <Reports electionEventId={record?.id} />
                    </Suspense>
                ) : null
            default:
                return null
        }
    }

    return (
        <>
            <ElectionHeader title={record?.name} subtitle="electionEventScreen.common.subtitle" />
            <Box sx={{maxWidth: {xs: 360, sm: 420, m: 680, lg: 1100}, bgcolor: "background.paper"}}>
                <Tabs
                    value={value}
                    onChange={handleChange}
                    variant="scrollable"
                    allowScrollButtonsMobile
                    scrollButtons="auto"
                    aria-label="scrollable auto tabs example"
                >
                    {showDashboard ? (
                        <Tab label={t("electionEventScreen.tabs.dashboard")} value={0} />
                    ) : null}
                    {showData ? <Tab label={t("electionEventScreen.tabs.data")} value={1} /> : null}
                    {showTextData ? (
                        <Tab label={t("electionEventScreen.tabs.localization")} value={2} />
                    ) : null}
                    {showVoters ? (
                        <Tab label={t("electionEventScreen.tabs.voters")} value={3} />
                    ) : null}
                    {showAreas ? (
                        <Tab label={t("electionEventScreen.tabs.areas")} value={4} />
                    ) : null}
                    {showKeys ? <Tab label={t("electionEventScreen.tabs.keys")} value={5} /> : null}
                    {showTally ? (
                        <Tab label={t("electionEventScreen.tabs.tally")} value={6} />
                    ) : null}
                    {showPublish ? (
                        <Tab label={t("electionEventScreen.tabs.publish")} value={7} />
                    ) : null}
                    {showTasksExecution ? (
                        <Tab label={t("electionEventScreen.tabs.tasks")} value={8} />
                    ) : null}
                    {showLogs ? <Tab label={t("electionEventScreen.tabs.logs")} value={9} /> : null}
                    {showEvents ? (
                        <Tab label={t("electionEventScreen.tabs.events")} value={10} />
                    ) : null}
                    {showReports ? (
                        <Tab label={t("electionEventScreen.tabs.reports")} value={12} />
                    ) : null}
                    {/*showNotifications ? (
                        <Tab label={t("electionEventScreen.tabs.notifications")} value={11} />
                    ) : null*/}
                </Tabs>
            </Box>

            <Box sx={{padding: 2}}>{renderTabContent()}</Box>
        </>
    )
}
