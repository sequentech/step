// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext, useEffect, Suspense, lazy} from "react"
import {TabbedShowLayout, useRecordContext, useSidebarState} from "react-admin"
import {Sequent_Backend_Election_Event} from "@/gql/graphql"
import ElectionHeader from "@/components/ElectionHeader"
import {AuthContext} from "@/providers/AuthContextProvider"
import {IPermissions} from "@/types/keycloak"
import {useTranslation} from "react-i18next"
import {useElectionEventTallyStore} from "@/providers/ElectionEventTallyProvider"
import {useLocation, useNavigate} from "react-router"
import {v4 as uuidv4} from "uuid"
import {EPublishType} from "../Publish/EPublishType"
import {EElectionEventLockedDown, i18n, translateElection} from "@sequentech/ui-core"
import {Box, CircularProgress} from "@mui/material"
import {Tabs} from "@/components/Tabs"
import {Dialog} from "@sequentech/ui-essentials"
import TestSql from "@/components/TestSql"

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
            navigate(locArr)
        }
    }, [location.pathname, navigate, record])

    // Code to refresh the dashboard when the user navigates to it
    const handleChildMount = () => {
        setLoadedChildren((prev) => (prev < 2 ? prev + 1 : prev))
    }

    useEffect(() => {
        if (loadedChildren === 1 || loadedChildren === 2) {
            refreshRef.current?.click()
        }
    }, [loadedChildren])

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
                title={
                    translateElection(record, "alias", i18n.language) ||
                    translateElection(record, "name", i18n.language) ||
                    record.alias ||
                    record.name ||
                    "-"
                }
                subtitle="electionEventScreen.common.subtitle"
            />
            <Box
                sx={{
                    bgcolor: "background.paper",
                }}
            >
                <Tabs
                    elements={[
                        ...(showDashboard
                            ? [
                                  {
                                      label: t("electionEventScreen.tabs.dashboard"),
                                      component: () => (
                                          <Suspense fallback={<div>Loading Dashboard...</div>}>
                                              <Box sx={{overflowX: "auto"}}>
                                                  <DashboardElectionEvent
                                                      refreshRef={refreshRef}
                                                      onMount={handleChildMount}
                                                  />
                                              </Box>
                                          </Suspense>
                                      ),
                                  },
                              ]
                            : []),
                        ...(showMonitoringDashboard
                            ? [
                                  {
                                      label: t("electionEventScreen.tabs.monitoring"),
                                      component: () => (
                                          <Suspense fallback={<div>Loading Dashboard...</div>}>
                                              <OVOFDashboardElectionEvent
                                                  refreshRef={refreshRef}
                                                  onMount={handleChildMount}
                                              />
                                          </Suspense>
                                      ),
                                  },
                              ]
                            : []),
                        ...(showData
                            ? [
                                  {
                                      label: t("electionEventScreen.tabs.data"),
                                      component: () => (
                                          <Suspense fallback={<div>Loading Data...</div>}>
                                              <EditElectionEventData />
                                          </Suspense>
                                      ),
                                  },
                              ]
                            : []),
                        ...(showTextData
                            ? [
                                  {
                                      label: t("electionEventScreen.tabs.localization"),
                                      component: () => (
                                          <Suspense fallback={<div>Loading Text Data...</div>}>
                                              <EditElectionEventTextData />
                                          </Suspense>
                                      ),
                                  },
                              ]
                            : []),
                        ...(showVoters
                            ? [
                                  {
                                      label: t("electionEventScreen.tabs.voters"),
                                      component: () => (
                                          <Suspense fallback={<div>Loading Voters...</div>}>
                                              <EditElectionEventUsers
                                                  electionEventId={record?.id}
                                              />
                                          </Suspense>
                                      ),
                                  },
                              ]
                            : []),
                        ...(showAreas
                            ? [
                                  {
                                      label: t("electionEventScreen.tabs.areas"),
                                      component: () => (
                                          <Suspense fallback={<div>Loading Areas...</div>}>
                                              <EditElectionEventAreas />
                                          </Suspense>
                                      ),
                                  },
                              ]
                            : []),
                        ...(showKeys
                            ? [
                                  {
                                      label: t("electionEventScreen.tabs.keys"),
                                      component: () => (
                                          <Suspense fallback={<div>Loading Keys...</div>}>
                                              <EditElectionEventKeys
                                                  isShowCeremony={showKeysList}
                                                  isShowTrusteeCeremony={showKeysList}
                                              />
                                          </Suspense>
                                      ),
                                      action: () => {
                                          setShowKeysList(uuidv4())
                                      },
                                  },
                              ]
                            : []),
                        ...(showTally
                            ? [
                                  {
                                      label: t("electionEventScreen.tabs.tally"),
                                      component: () => (
                                          <Suspense fallback={<div>Loading Tally...</div>}>
                                              <EditElectionEventTally />
                                          </Suspense>
                                      ),
                                      action: () => setTallyId(null),
                                  },
                              ]
                            : []),
                        ...(showPublish
                            ? [
                                  {
                                      label: t("electionEventScreen.tabs.publish"),
                                      component: () => (
                                          <Suspense fallback={<div>Loading Publish...</div>}>
                                              <Publish
                                                  electionEventId={record?.id}
                                                  type={EPublishType.Event}
                                                  showList={showPublishList}
                                              />
                                          </Suspense>
                                      ),
                                      action: (index: number) => {
                                          localStorage.setItem(
                                              "electionEventPublishTabIndex",
                                              index.toString()
                                          )
                                          setShowPublishList(uuidv4())
                                      },
                                  },
                              ]
                            : []),
                        ...(showTasksExecution
                            ? [
                                  {
                                      label: t("electionEventScreen.tabs.tasks"),
                                      component: () => (
                                          <Suspense fallback={<div>Loading Tasks...</div>}>
                                              <EditElectionEventTasks showList={showTaskList} />
                                          </Suspense>
                                      ),
                                      action: () => {
                                          setShowTaskList(uuidv4())
                                      },
                                  },
                              ]
                            : []),
                        ...(showLogs
                            ? [
                                  {
                                      label: t("electionEventScreen.tabs.logs"),
                                      component: () => (
                                          <Suspense fallback={<div>Loading Logs...</div>}>
                                              <ElectoralLog />
                                          </Suspense>
                                      ),
                                  },
                              ]
                            : []),
                        ...(showEvents
                            ? [
                                  {
                                      label: t("electionEventScreen.tabs.events"),
                                      component: () => (
                                          <Suspense fallback={<div>Loading Events...</div>}>
                                              <EditElectionEventEvents
                                                  electionEventId={record?.id}
                                              />
                                          </Suspense>
                                      ),
                                  },
                              ]
                            : []),
                        ...(showReports
                            ? [
                                  {
                                      label: t("electionEventScreen.tabs.reports"),
                                      component: () => (
                                          <Suspense fallback={<div>Loading Reports...</div>}>
                                              <EditElectionEventReports
                                                  electionEventId={record?.id}
                                              />
                                          </Suspense>
                                      ),
                                  },
                              ]
                            : []),
                        ...(showApprovalsExecution
                            ? [
                                  {
                                      label: t("electionEventScreen.tabs.approvals"),
                                      component: () => (
                                          <Suspense fallback={<div>Loading Approvals...</div>}>
                                              <EditElectionEventApprovals
                                                  electionEventId={record?.id}
                                                  showList={showApprovalList}
                                              />
                                          </Suspense>
                                      ),
                                      action: () => {
                                          setShowApprovalList(uuidv4())
                                          localStorage.setItem("approvals_status_filter", "pending")
                                      },
                                  },
                              ]
                            : []),
                    ]}
                />
            </Box>
        </Box>
    )
}
