// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext, useEffect, Suspense, lazy, useCallback, useMemo} from "react"
import {useRecordContext, useSidebarState} from "react-admin"
import {Sequent_Backend_Election_Event} from "@/gql/graphql"
import ElectionHeader from "@/components/ElectionHeader"
import {AuthContext} from "@/providers/AuthContextProvider"
import {IPermissions} from "@/types/keycloak"
import {useTranslation} from "react-i18next"
import {useElectionEventTallyStore} from "@/providers/ElectionEventTallyProvider"
import {useLocation, useNavigate} from "react-router-dom"
import {v4 as uuidv4} from "uuid"
import {EPublishType} from "../Publish/EPublishType"
import {EElectionEventLockedDown, i18n} from "@sequentech/ui-core"
import {Box, CircularProgress} from "@mui/material"
import {Tabs} from "@/components/Tabs"
import {Dialog} from "@sequentech/ui-essentials"
import {useAliasRenderer} from "@/hooks/useAliasRenderer"

// Lazy load the tab components
const DashboardElectionEvent = lazy(() => import("@/components/dashboard/election-event/Dashboard"))
const OVOFDashboardElectionEvent = lazy(
    () => import("@/components/monitoring-dashboard/election-event/MonitoringDashboard")
)
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
const EditElectionEventApprovals = lazy(() =>
    import("./EditElectionEventApprovals").then((module) => ({
        default: module.EditElectionEventApprovals,
    }))
)

const EditElectionEventReports = lazy(() =>
    import("../Reports/EditReportsTab").then((module) => ({
        default: module.EditReportsTab,
    }))
)

// ---------------------------------------------------------------------------
// Stable module-level tab components — never recreated during re-renders.
// Using module-level definitions prevents React from seeing a "new" component
// type each render, which would cause unnecessary unmount/remount cycles and
// the infinite fetch loop triggered by EditBase re-initialising on each mount.
// ---------------------------------------------------------------------------

interface DashboardTabProps {
    refreshRef: React.MutableRefObject<HTMLButtonElement | undefined>
    handleChildMount: () => void
}

const DashboardTab: React.FC<DashboardTabProps> = ({refreshRef, handleChildMount}) => (
    <Suspense fallback={<div>Loading Dashboard...</div>}>
        <Box sx={{overflowX: "auto"}}>
            <DashboardElectionEvent refreshRef={refreshRef} onMount={handleChildMount} />
        </Box>
    </Suspense>
)

const OVOFDashboardTab: React.FC<DashboardTabProps> = ({refreshRef, handleChildMount}) => (
    <Suspense fallback={<div>Loading Dashboard...</div>}>
        <Box sx={{overflowX: "auto"}}>
            <OVOFDashboardElectionEvent refreshRef={refreshRef} onMount={handleChildMount} />
        </Box>
    </Suspense>
)

const DataTab: React.FC = () => (
    <Suspense fallback={<div>Loading Data...</div>}>
        <EditElectionEventData />
    </Suspense>
)

const LocalizationTab: React.FC = () => (
    <Suspense fallback={<div>Loading Text Data...</div>}>
        <EditElectionEventTextData />
    </Suspense>
)

const VotersTab: React.FC = () => {
    const record = useRecordContext<Sequent_Backend_Election_Event>()
    return (
        <Suspense fallback={<div>Loading Voters...</div>}>
            <EditElectionEventUsers electionEventId={record?.id as string | undefined} />
        </Suspense>
    )
}

const AreasTab: React.FC = () => (
    <Suspense fallback={<div>Loading Areas...</div>}>
        <EditElectionEventAreas />
    </Suspense>
)

const KeysTab: React.FC<{isShowCeremony: string | null; isShowTrusteeCeremony: string | null}> = ({
    isShowCeremony,
    isShowTrusteeCeremony,
}) => (
    <Suspense fallback={<div>Loading Keys...</div>}>
        <EditElectionEventKeys
            isShowCeremony={isShowCeremony}
            isShowTrusteeCeremony={isShowTrusteeCeremony}
        />
    </Suspense>
)

const TallyTab: React.FC = () => (
    <Suspense fallback={<div>Loading Tally...</div>}>
        <EditElectionEventTally />
    </Suspense>
)

const PublishTab: React.FC<{showList: string | undefined}> = ({showList}) => {
    const record = useRecordContext<Sequent_Backend_Election_Event>()
    return (
        <Suspense fallback={<div>Loading Publish...</div>}>
            <Publish electionEventId={record?.id} type={EPublishType.Event} showList={showList} />
        </Suspense>
    )
}

const TasksTab: React.FC<{showList: string | undefined}> = ({showList}) => (
    <Suspense fallback={<div>Loading Tasks...</div>}>
        <EditElectionEventTasks showList={showList} />
    </Suspense>
)

const LogsTab: React.FC = () => (
    <Suspense fallback={<div>Loading Logs...</div>}>
        <ElectoralLog />
    </Suspense>
)

const EventsTab: React.FC = () => {
    const record = useRecordContext<Sequent_Backend_Election_Event>()
    return (
        <Suspense fallback={<div>Loading Events...</div>}>
            <EditElectionEventEvents electionEventId={record?.id as string} />
        </Suspense>
    )
}

const ReportsTab: React.FC = () => {
    const record = useRecordContext<Sequent_Backend_Election_Event>()
    return (
        <Suspense fallback={<div>Loading Reports...</div>}>
            <EditElectionEventReports electionEventId={record?.id as string} />
        </Suspense>
    )
}

const ApprovalsTab: React.FC<{showList: string | undefined}> = ({showList}) => {
    const record = useRecordContext<Sequent_Backend_Election_Event>()
    return (
        <Suspense fallback={<div>Loading Approvals...</div>}>
            <EditElectionEventApprovals
                electionEventId={record?.id as string}
                showList={showList}
            />
        </Suspense>
    )
}

export const ElectionEventTabs: React.FC = () => {
    const record = useRecordContext<Sequent_Backend_Election_Event>()
    const authContext = useContext(AuthContext)
    const [showKeysList, setShowKeysList] = React.useState<string | null>(null)
    const [showTaskList, setShowTaskList] = React.useState<string | undefined>()
    const [showPublishList, setShowPublishList] = React.useState<string | undefined>()
    const [showApprovalList, setShowApprovalList] = React.useState<string | undefined>()
    const location = useLocation()
    const navigate = useNavigate()
    const refreshRef = React.useRef<HTMLButtonElement>()
    const {t} = useTranslation()
    const isElectionEventLocked =
        record?.presentation?.locked_down == EElectionEventLockedDown.LOCKED_DOWN
    const {setTallyId} = useElectionEventTallyStore()
    const [open] = useSidebarState()
    const aliasRenderer = useAliasRenderer()

    const showDashboard = authContext.isAuthorized(
        true,
        authContext.tenantId,
        IPermissions.ADMIN_DASHBOARD_VIEW
    )

    const showMonitoringDashboard = authContext.isAuthorized(
        true,
        authContext.tenantId,
        IPermissions.MONITORING_DASHBOARD_VIEW_ELECTION_EVENT
    )
    const showData =
        !isElectionEventLocked &&
        authContext.isAuthorized(true, authContext.tenantId, IPermissions.ELECTION_EVENT_DATA_TAB)
    const showTextData =
        !isElectionEventLocked &&
        authContext.isAuthorized(true, authContext.tenantId, IPermissions.ELECTION_EVENT_DATA_TAB)
    const showVoters = authContext.isAuthorized(
        true,
        authContext.tenantId,
        IPermissions.ELECTION_EVENT_VOTERS_TAB
    )
    const showAreas =
        !isElectionEventLocked &&
        authContext.isAuthorized(true, authContext.tenantId, IPermissions.ELECTION_EVENT_AREAS_TAB)
    const showKeys =
        !isElectionEventLocked &&
        authContext.isAuthorized(true, authContext.tenantId, [
            IPermissions.ADMIN_CEREMONY,
            IPermissions.TRUSTEE_CEREMONY,
        ]) &&
        authContext.isAuthorized(true, authContext.tenantId, IPermissions.ELECTION_EVENT_KEYS_TAB)
    const showTally =
        !isElectionEventLocked &&
        authContext.isAuthorized(true, authContext.tenantId, [
            IPermissions.TALLY_READ,
            IPermissions.TALLY_START,
        ]) &&
        authContext.isAuthorized(true, authContext.tenantId, IPermissions.ELECTION_EVENT_TALLY_TAB)
    const showPublish =
        !isElectionEventLocked &&
        authContext.isAuthorized(
            true,
            authContext.tenantId,
            IPermissions.ELECTION_EVENT_PUBLISH_TAB
        )
    const showLogs = authContext.isAuthorized(
        true,
        authContext.tenantId,
        IPermissions.ELECTION_EVENT_LOGS_TAB
    )
    const showTasksExecution =
        !isElectionEventLocked &&
        authContext.isAuthorized(true, authContext.tenantId, IPermissions.ELECTION_EVENT_TASKS_TAB)
    const showEvents =
        !isElectionEventLocked &&
        authContext.isAuthorized(
            true,
            authContext.tenantId,
            IPermissions.ELECTION_EVENT_SCHEDULED_TAB
        )
    const showNotifications = authContext.isAuthorized(
        true,
        authContext.tenantId,
        IPermissions.ELECTION_EVENT_LOGS_TAB
    )

    const showReports = authContext.isAuthorized(
        true,
        authContext.tenantId,
        IPermissions.ELECTION_EVENT_REPORTS_TAB
    )
    const showApprovalsExecution =
        !isElectionEventLocked &&
        authContext.isAuthorized(
            true,
            authContext.tenantId,
            IPermissions.ELECTION_EVENT_APPROVALS_TAB
        )

    const [loadedChildren, setLoadedChildren] = React.useState<number>(0)
    const [value, setValue] = React.useState(0)

    const handleChange = (event: React.SyntheticEvent, newValue: number) => {
        setValue(newValue)
    }

    useEffect(() => {
        if (record) {
            const locArr = location.pathname.split("/").slice(0, 3).join("/")
            console.log("[ElectionEventTabs] navigate effect:", {
                pathname: location.pathname,
                locArr,
                willNavigate: location.pathname !== locArr,
            })
            if (location.pathname !== locArr) {
                navigate(locArr)
            }
        }
    }, [location.pathname, navigate, record])

    const handleChildMount = useCallback(() => {
        setLoadedChildren((prev) => (prev < 2 ? prev + 1 : prev))
    }, [])

    useEffect(() => {
        if (loadedChildren === 1 || loadedChildren === 2) {
            refreshRef.current?.click()
        }
    }, [loadedChildren])

    const tabs = useMemo(
        () => [
            ...(showDashboard
                ? [
                      {
                          label: t("electionEventScreen.tabs.dashboard"),
                          component: DashboardTab,
                          props: {refreshRef, handleChildMount},
                      },
                  ]
                : []),
            ...(showMonitoringDashboard
                ? [
                      {
                          label: t("electionEventScreen.tabs.monitoring"),
                          component: OVOFDashboardTab,
                          props: {refreshRef, handleChildMount},
                      },
                  ]
                : []),
            ...(showData
                ? [
                      {
                          label: t("electionEventScreen.tabs.data"),
                          component: DataTab,
                      },
                  ]
                : []),
            ...(showTextData
                ? [
                      {
                          label: t("electionEventScreen.tabs.localization"),
                          component: LocalizationTab,
                      },
                  ]
                : []),
            ...(showVoters
                ? [
                      {
                          label: t("electionEventScreen.tabs.voters"),
                          component: VotersTab,
                      },
                  ]
                : []),
            ...(showAreas
                ? [
                      {
                          label: t("electionEventScreen.tabs.areas"),
                          component: AreasTab,
                      },
                  ]
                : []),
            ...(showKeys
                ? [
                      {
                          label: t("electionEventScreen.tabs.keys"),
                          component: KeysTab,
                          props: {
                              isShowCeremony: showKeysList,
                              isShowTrusteeCeremony: showKeysList,
                          },
                          action: () => setShowKeysList(uuidv4()),
                      },
                  ]
                : []),
            ...(showTally
                ? [
                      {
                          label: t("electionEventScreen.tabs.tally"),
                          component: TallyTab,
                          action: () => setTallyId(null),
                      },
                  ]
                : []),
            ...(showPublish
                ? [
                      {
                          label: t("electionEventScreen.tabs.publish"),
                          component: PublishTab,
                          props: {showList: showPublishList},
                          action: (index: number) => {
                              localStorage.setItem("electionEventPublishTabIndex", index.toString())
                              setShowPublishList(uuidv4())
                          },
                      },
                  ]
                : []),
            ...(showTasksExecution
                ? [
                      {
                          label: t("electionEventScreen.tabs.tasks"),
                          component: TasksTab,
                          props: {showList: showTaskList},
                          action: () => setShowTaskList(uuidv4()),
                      },
                  ]
                : []),
            ...(showLogs
                ? [
                      {
                          label: t("electionEventScreen.tabs.logs"),
                          component: LogsTab,
                      },
                  ]
                : []),
            ...(showEvents
                ? [
                      {
                          label: t("electionEventScreen.tabs.events"),
                          component: EventsTab,
                      },
                  ]
                : []),
            ...(showReports
                ? [
                      {
                          label: t("electionEventScreen.tabs.reports"),
                          component: ReportsTab,
                      },
                  ]
                : []),
            ...(showApprovalsExecution
                ? [
                      {
                          label: t("electionEventScreen.tabs.approvals"),
                          component: ApprovalsTab,
                          props: {showList: showApprovalList},
                          action: () => {
                              setShowApprovalList(uuidv4())
                              localStorage.setItem("approvals_status_filter", "pending")
                          },
                      },
                  ]
                : []),
        ],
        [
            showDashboard,
            showMonitoringDashboard,
            showData,
            showTextData,
            showVoters,
            showAreas,
            showKeys,
            showKeysList,
            showTally,
            showPublish,
            showPublishList,
            showTasksExecution,
            showTaskList,
            showLogs,
            showEvents,
            showReports,
            showApprovalsExecution,
            showApprovalList,
            t,
            handleChildMount,
            setTallyId,
        ]
    )

    if (!record) {
        return (
            <Box>
                <CircularProgress />
            </Box>
        )
    }

    return (
        <Box
            sx={{maxWidth: `calc(100vw - ${open ? "352px" : "96px"})`, bgcolor: "background.paper"}}
            className="events-box"
        >
            <ElectionHeader
                title={aliasRenderer(record)}
                subtitle="electionEventScreen.common.subtitle"
            />
            <Box
                sx={{
                    bgcolor: "background.paper",
                }}
            >
                <Tabs elements={tabs} />
            </Box>
        </Box>
    )
}
