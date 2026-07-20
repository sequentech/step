// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {
    useContext,
    useEffect,
    useMemo,
    useRef,
    useState,
    Suspense,
    lazy,
    useCallback,
} from "react"
import {useRecordContext, useSidebarState, RecordContextProvider} from "react-admin"
import {Sequent_Backend_Election_Event} from "@/gql/graphql"
import ElectionHeader from "@/components/ElectionHeader"
import {AuthContext} from "@/providers/AuthContextProvider"
import {IPermissions} from "@/types/keycloak"
import {useTranslation} from "react-i18next"
import {useElectionEventTallyStore} from "@/providers/ElectionEventTallyProvider"
import {v4 as uuidv4} from "uuid"
import {EPublishType} from "../Publish/EPublishType"
import {
    EElectionEventLockedDown,
    EVoterCertificatePolicy,
    IVotingChannelsConfig,
} from "@sequentech/ui-core"
import {Box, CircularProgress} from "@mui/material"
import {Tabs} from "@/components/Tabs"
import {useNavigate, useLocation} from "react-router-dom"
import {useAliasRenderer} from "@/hooks/useAliasRenderer"

// ---------------------------------------------------------------------
// Lazy load all tab contents
// ---------------------------------------------------------------------
const DashboardElectionEvent = lazy(() => import("@/components/dashboard/election-event/Dashboard"))
const EditElectionEventData = lazy(() =>
    import("./EditElectionEventData").then((m) => ({default: m.EditElectionEventData}))
)
const EditElectionEventTextData = lazy(() =>
    import("./EditElectionEventTextData").then((m) => ({default: m.default}))
)
const EditElectionEventUsers = lazy(() =>
    import("./EditElectionEventUsers").then((m) => ({default: m.EditElectionEventUsers}))
)
const EditElectionEventAreas = lazy(() =>
    import("./EditElectionEventAreas").then((m) => ({default: m.EditElectionEventAreas}))
)
const EditElectionEventKeys = lazy(() =>
    import("./EditElectionEventKeys").then((m) => ({default: m.EditElectionEventKeys}))
)
const EditElectionEventCAs = lazy(() =>
    import("./EditElectionEventCAs").then((m) => ({default: m.EditElectionEventCAs}))
)
const EditElectionEventIvr = lazy(() =>
    import("./EditElectionEventIvr").then((m) => ({default: m.EditElectionEventIvr}))
)
const EditElectionEventTally = lazy(() =>
    import("./EditElectionEventTally").then((m) => ({default: m.EditElectionEventTally}))
)
const TallySheetImports = lazy(() =>
    import("../TallySheetImport/TallySheetImports").then((m) => ({
        default: m.TallySheetImports,
    }))
)
const Publish = lazy(() =>
    import("@/resources/Publish/Publish").then((m) => ({default: m.Publish}))
)
const ElectoralLog = lazy(() => import("./ElectoralLog").then((m) => ({default: m.ElectoralLog})))
const EditElectionEventTasks = lazy(() =>
    import("./EditElectionEventTasks").then((m) => ({default: m.EditElectionEventTasks}))
)
const EditElectionEventEvents = lazy(() =>
    import("./EditElectionEventScheduledEvents").then((m) => ({
        default: m.EditElectionEventEvents,
    }))
)
const EditElectionEventApprovals = lazy(() =>
    import("./EditElectionEventApprovals").then((m) => ({
        default: m.EditElectionEventApprovals,
    }))
)
const EditElectionEventReports = lazy(() =>
    import("../Reports/EditReportsTab").then((m) => ({default: m.EditReportsTab}))
)
const VoterListSync = lazy(() =>
    import("../VoterListSync/VoterListSync").then((m) => ({default: m.VoterListSync}))
)

// ---------------------------------------------------------------------
// Stable Tab Components (all use useRecordContext)
// ---------------------------------------------------------------------

interface ITabProps {
    refreshRef: React.RefObject<HTMLButtonElement | null>
    handleChildMount: () => void
}

const DashboardTab: React.FC<ITabProps> = ({refreshRef, handleChildMount}) => (
    <Suspense fallback={<div>Loading Dashboard...</div>}>
        <Box sx={{overflowX: "auto"}}>
            <DashboardElectionEvent refreshRef={refreshRef} onMount={handleChildMount} />
        </Box>
    </Suspense>
)

const DataTab: React.FC = () => (
    <Suspense fallback={<div>Loading Data...</div>}>
        <EditElectionEventData />
    </Suspense>
)

const LocalizationTab: React.FC = () => (
    <Suspense fallback={<div>Loading Localization...</div>}>
        <EditElectionEventTextData />
    </Suspense>
)

const VotersTab: React.FC = () => {
    const record = useRecordContext<Sequent_Backend_Election_Event>()
    return (
        <Suspense fallback={<div>Loading Voters...</div>}>
            <EditElectionEventUsers />
        </Suspense>
    )
}

const AreasTab: React.FC = () => (
    <Suspense fallback={<div>Loading Areas...</div>}>
        <EditElectionEventAreas />
    </Suspense>
)

const KeysTab: React.FC<{showKeysList: string | null}> = ({showKeysList}) => (
    <Suspense fallback={<div>Loading Keys...</div>}>
        <EditElectionEventKeys isShowCeremony={showKeysList} isShowTrusteeCeremony={showKeysList} />
    </Suspense>
)

const CAsTab: React.FC = () => {
    const {t} = useTranslation()
    return (
        <Suspense fallback={<div>{t("common.label.loadingData")}</div>}>
            <EditElectionEventCAs />
        </Suspense>
    )
}

const IvrTab: React.FC = () => {
    const {t} = useTranslation()
    return (
        <Suspense fallback={<div>{t("common.label.loadingData")}</div>}>
            <EditElectionEventIvr />
        </Suspense>
    )
}

const TallyTab: React.FC = () => (
    <Suspense fallback={<div>Loading Tally...</div>}>
        <EditElectionEventTally />
    </Suspense>
)

const TallySheetImportsTab: React.FC<{
    openImportId?: string | null
    onOpenImportHandled?: () => void
}> = ({openImportId, onOpenImportHandled}) => (
    <Suspense fallback={<div>Loading Tally Sheet Imports...</div>}>
        <TallySheetImports openImportId={openImportId} onOpenImportHandled={onOpenImportHandled} />
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
            <EditElectionEventEvents electionEventId={record?.id} />
        </Suspense>
    )
}

const ReportsTab: React.FC = () => {
    const record = useRecordContext<Sequent_Backend_Election_Event>()
    return (
        <Suspense fallback={<div>Loading Reports...</div>}>
            <EditElectionEventReports electionEventId={record?.id} />
        </Suspense>
    )
}

const VoterListSyncTab: React.FC = () => (
    <Suspense fallback={<div>Loading Voterview Sync...</div>}>
        <VoterListSync />
    </Suspense>
)

const ApprovalsTab: React.FC<{showList: string | undefined}> = ({showList}) => {
    const record = useRecordContext<Sequent_Backend_Election_Event>()
    return (
        <Suspense fallback={<div>Loading Approvals...</div>}>
            <EditElectionEventApprovals electionEventId={record?.id} showList={showList} />
        </Suspense>
    )
}

// ---------------------------------------------------------------------
// Main Component
// ---------------------------------------------------------------------
export const ElectionEventTabs: React.FC = () => {
    const record = useRecordContext<Sequent_Backend_Election_Event>()
    const authContext = useContext(AuthContext)
    const {t} = useTranslation()
    const location = useLocation()
    const navigate = useNavigate()
    const refreshRef = useRef<HTMLButtonElement | null>(null)
    const {setTallyId} = useElectionEventTallyStore()
    const [open] = useSidebarState()
    const aliasRenderer = useAliasRenderer()

    // State for tab-specific triggers
    const [showKeysList, setShowKeysList] = useState<string | null>(null)
    const [showTaskList, setShowTaskList] = useState<string | undefined>()
    const [showPublishList, setShowPublishList] = useState<string | undefined>()
    const [showApprovalList, setShowApprovalList] = useState<string | undefined>()
    const [pendingTallySheetImportId, setPendingTallySheetImportId] = useState<string | null>(
        () => {
            const baseUrl = new URL(window.location.href)
            return baseUrl.searchParams.get("tallySheetImportId")
        }
    )
    const [selectedTab, setSelectedTab] = useState(() => {
        const baseUrl = new URL(window.location.href)
        return Number.parseInt(baseUrl.searchParams.get("tabIndex") ?? "0")
    })
    const clearPendingTallySheetImportId = useCallback(() => {
        setPendingTallySheetImportId(null)
        const searchParams = new URLSearchParams(location.search)
        if (searchParams.has("tallySheetImportId")) {
            searchParams.delete("tallySheetImportId")
            const search = searchParams.toString()
            navigate(
                {pathname: location.pathname, search: search ? `?${search}` : ""},
                {replace: true}
            )
        }
    }, [location.pathname, location.search, navigate])

    // Dashboard refresh logic// Dashboard refresh logic
    const [loadedChildren, setLoadedChildren] = useState(0)

    // MEMOIZE THIS!
    const handleChildMount = useCallback(() => {
        setLoadedChildren((prev) => (prev < 2 ? prev + 1 : prev))
    }, [])
    useEffect(() => {
        if (loadedChildren === 1 || loadedChildren === 2) {
            refreshRef.current?.click()
        }
    }, [loadedChildren])

    // Clean URL on mount
    useEffect(() => {
        if (record) {
            const basePath = location.pathname.split("/").slice(0, 3).join("/")
            navigate(basePath)
        }
    }, [location.pathname, navigate, record])

    // Permission checks
    const isElectionEventLocked =
        record?.presentation?.locked_down === EElectionEventLockedDown.LOCKED_DOWN

    const showDashboard = authContext.isAuthorized(
        true,
        authContext.tenantId,
        IPermissions.ADMIN_DASHBOARD_VIEW
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
    const showTallySheetImports =
        !isElectionEventLocked &&
        authContext.isAuthorized(true, authContext.tenantId, IPermissions.TALLY_SHEET_IMPORT_VIEW)
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
    const showCAs =
        authContext.isAuthorized(true, authContext.tenantId, IPermissions.ELECTION_EVENT_CAS_TAB) &&
        record?.presentation?.voter_certificate_policy === EVoterCertificatePolicy.ENABLED
    // MOCK: hardcoded to true for the UI prototype - see DatafixPossibleImplementation.md
    // "Permissions". Final implementation should read:
    // authContext.isAuthorized(true, authContext.tenantId, IPermissions.ELECTION_EVENT_VOTER_LIST_SYNC_TAB)
    const showVoterListSync = true
    const isTelephoneChannelEnabled =
        (record?.voting_channels as IVotingChannelsConfig | undefined)?.telephone ?? false
    const showIvr =
        isTelephoneChannelEnabled &&
        authContext.isAuthorized(true, authContext.tenantId, IPermissions.ELECTION_EVENT_IVR_TAB)

    // -----------------------------------------------------------------
    // Build tabs with 100% stable references
    // -----------------------------------------------------------------
    const tabs = useMemo(() => {
        const result: Array<{
            id?: string
            label: string
            component: React.FC<any>
            props?: any
            action?: (index?: number) => void
        }> = []

        // Dashboard
        if (showDashboard) {
            result.push({
                label: t("electionEventScreen.tabs.dashboard"),
                component: DashboardTab,
                props: {refreshRef, handleChildMount},
            })
        }

        // Data
        if (showData) {
            result.push({label: t("electionEventScreen.tabs.data"), component: DataTab})
        }

        // IVR
        if (showIvr) {
            result.push({label: t("electionEventScreen.tabs.ivr"), component: IvrTab})
        }

        // Localization
        if (showTextData) {
            result.push({
                label: t("electionEventScreen.tabs.localization"),
                component: LocalizationTab,
            })
        }

        // Voters
        if (showVoters) {
            result.push({label: t("electionEventScreen.tabs.voters"), component: VotersTab})
        }

        // Areas
        if (showAreas) {
            result.push({label: t("electionEventScreen.tabs.areas"), component: AreasTab})
        }

        // Keys
        if (showKeys) {
            result.push({
                label: t("electionEventScreen.tabs.keys"),
                component: KeysTab,
                props: {showKeysList},
                action: () => setShowKeysList(uuidv4()),
            })
        }

        // CAs
        if (showCAs) {
            result.push({label: t("electionEventScreen.tabs.cas"), component: CAsTab})
        }

        // Tally
        if (showTally) {
            result.push({
                id: "tally",
                label: t("electionEventScreen.tabs.tally"),
                component: TallyTab,
                action: () => setTallyId(null),
            })
        }

        if (showTallySheetImports) {
            result.push({
                id: "tally-sheet-imports",
                label: t("electionEventScreen.tabs.tallySheetImports"),
                component: TallySheetImportsTab,
                props: {
                    openImportId: pendingTallySheetImportId,
                    onOpenImportHandled: clearPendingTallySheetImportId,
                },
            })
        }

        // Voter List Sync (Datafix reconciliation wizards)
        if (showVoterListSync) {
            result.push({
                // MOCK: hardcoded label, no i18n key added for this prototype.
                label: "VOTERVIEW SYNC",
                component: VoterListSyncTab,
            })
        }

        // Publish
        if (showPublish) {
            result.push({
                label: t("electionEventScreen.tabs.publish"),
                component: PublishTab,
                props: {showList: showPublishList},
                action: (index?: number) => {
                    if (!index) {
                        return
                    }
                    localStorage.setItem("electionEventPublishTabIndex", index.toString())
                    setShowPublishList(uuidv4())
                },
            })
        }

        // Tasks
        if (showTasksExecution) {
            result.push({
                label: t("electionEventScreen.tabs.tasks"),
                component: TasksTab,
                props: {showList: showTaskList},
                action: () => setShowTaskList(uuidv4()),
            })
        }

        // Logs
        if (showLogs) {
            result.push({label: t("electionEventScreen.tabs.logs"), component: LogsTab})
        }

        // Events
        if (showEvents) {
            result.push({label: t("electionEventScreen.tabs.events"), component: EventsTab})
        }

        // Reports
        if (showReports) {
            result.push({label: t("electionEventScreen.tabs.reports"), component: ReportsTab})
        }

        // Approvals
        if (showApprovalsExecution) {
            result.push({
                label: t("electionEventScreen.tabs.approvals"),
                component: ApprovalsTab,
                props: {showList: showApprovalList},
                action: () => {
                    setShowApprovalList(uuidv4())
                    localStorage.setItem("approvals_status_filter", "pending")
                },
            })
        }

        return result
    }, [
        showDashboard,
        showData,
        showTextData,
        showVoters,
        showAreas,
        showKeys,
        showTally,
        showTallySheetImports,
        showVoterListSync,
        showPublish,
        showLogs,
        showTasksExecution,
        showEvents,
        showReports,
        showApprovalsExecution,
        showCAs,
        showIvr,
        t,
        showKeysList,
        showPublishList,
        showTaskList,
        showApprovalList,
        refreshRef,
        handleChildMount,
        setTallyId,
        pendingTallySheetImportId,
        clearPendingTallySheetImportId,
    ])

    const tallySheetImportsTabIndex = tabs.findIndex((tab) => tab.id === "tally-sheet-imports")

    useEffect(() => {
        const searchParams = new URLSearchParams(location.search)
        const tabId = searchParams.get("tabId")
        const importId = searchParams.get("tallySheetImportId")
        if (tallySheetImportsTabIndex < 0) {
            return
        }
        if (tabId === "tally-sheet-imports" || importId) {
            setSelectedTab(tallySheetImportsTabIndex)
        }
        if (importId) {
            setPendingTallySheetImportId(importId)
        }
    }, [location.search, tallySheetImportsTabIndex])

    if (!record) {
        return (
            <Box>
                <CircularProgress />
            </Box>
        )
    }

    return (
        <Box
            sx={{
                maxWidth: `calc(100vw - ${open ? "352px" : "96px"})`,
                bgcolor: "background.paper",
            }}
            className="events-box"
        >
            <ElectionHeader
                title={aliasRenderer(record)}
                subtitle="electionEventScreen.common.subtitle"
            />
            <Box sx={{bgcolor: "background.paper"}}>
                <RecordContextProvider value={record}>
                    <Tabs
                        elements={tabs}
                        selectedTab={selectedTab}
                        onSelectedTabChange={setSelectedTab}
                    />
                </RecordContextProvider>
            </Box>
        </Box>
    )
}
