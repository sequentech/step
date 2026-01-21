// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {Suspense, useContext, useEffect, useMemo, useState} from "react"
import {useTranslation} from "react-i18next"
import {useRecordContext, useSidebarState, RecordContextProvider} from "react-admin"
import {v4 as uuidv4} from "uuid"

import {AuthContext} from "@/providers/AuthContextProvider"
import ElectionHeader from "@/components/ElectionHeader"
import DashboardElection from "@/components/dashboard/election/Dashboard"
import {Sequent_Backend_Election} from "@/gql/graphql"
import {Publish} from "../Publish/Publish"
import {EditElectionData} from "./ElectionData"
import {EPublishType} from "../Publish/EPublishType"
import {IPermissions} from "@/types/keycloak"
import {EditElectionEventUsers} from "../ElectionEvent/EditElectionEventUsers"
import {ResourceListStyles} from "@/components/styles/ResourceListStyles"
import {Box, Typography} from "@mui/material"
import {EElectionEventLockedDown, i18n, translateElection} from "@sequentech/ui-core"
import {EditElectionEventApprovals} from "../ElectionEvent/EditElectionEventApprovals"
import {Tabs} from "@/components/Tabs"

// ---------------------------------------------------------------------
// Stable Tab Components
// ---------------------------------------------------------------------

const DashboardTab: React.FC = () => (
    <Suspense fallback={<div>Loading Dashboard...</div>}>
        <DashboardElection />
    </Suspense>
)

const DataTab: React.FC = () => (
    <Suspense fallback={<div>Loading Data...</div>}>
        <EditElectionData />
    </Suspense>
)

const VotersTab: React.FC = () => {
    return (
        <Suspense fallback={<div>Loading Voters...</div>}>
            <EditElectionEventUsers />
        </Suspense>
    )
}

const PublishTab: React.FC = () => {
    const record = useRecordContext<Sequent_Backend_Election>()
    const [key] = useState(() => uuidv4()) // Stable key per mount
    return (
        <Suspense fallback={<div>Loading Publish...</div>}>
            <Publish
                key={key}
                electionEventId={record?.election_event_id}
                electionId={record?.id}
                type={EPublishType.Election}
            />
        </Suspense>
    )
}

const ApprovalsTab: React.FC = () => {
    const record = useRecordContext<Sequent_Backend_Election>()
    return (
        <Suspense fallback={<div>Loading Approvals...</div>}>
            <EditElectionEventApprovals
                electionEventId={record?.election_event_id}
                electionId={record?.id}
            />
        </Suspense>
    )
}

// ---------------------------------------------------------------------
// Main Component
// ---------------------------------------------------------------------
export const ElectionTabs: React.FC = () => {
    const record = useRecordContext<Sequent_Backend_Election>()
    const {t} = useTranslation()
    const authContext = useContext(AuthContext)
    const usersPermissionLabels = authContext.permissionLabels
    const [hasPermissionToViewElection, setHasPermissionToViewElection] = useState<boolean>(true)
    const [open] = useSidebarState()

    const isElectionEventLocked =
        record?.presentation?.locked_down === EElectionEventLockedDown.LOCKED_DOWN

    // Permission checks
    const showDashboard = authContext.isAuthorized(
        true,
        authContext.tenantId,
        IPermissions.ELECTION_DASHBOARD_TAB
    )

    const showData = authContext.isAuthorized(
        true,
        authContext.tenantId,
        IPermissions.ELECTION_DATA_TAB
    )

    const showVoters = authContext.isAuthorized(
        true,
        authContext.tenantId,
        IPermissions.ELECTION_VOTERS_TAB
    )

    const showPublish = authContext.isAuthorized(
        true,
        authContext.tenantId,
        IPermissions.ELECTION_PUBLISH_TAB
    )

    const showApprovalsExecution =
        !isElectionEventLocked &&
        authContext.isAuthorized(true, authContext.tenantId, IPermissions.ELECTION_APPROVALS_TAB)

    // Permission label check
    useEffect(() => {
        if (
            usersPermissionLabels &&
            record?.permission_label &&
            !usersPermissionLabels.includes(record.permission_label)
        ) {
            setHasPermissionToViewElection(false)
        } else {
            setHasPermissionToViewElection(true)
        }
    }, [usersPermissionLabels, record?.permission_label])

    // Build tabs with stable references
    const tabs = useMemo(() => {
        const result: Array<{
            label: string
            component: React.FC
            action?: (index: number) => void
        }> = []

        if (showDashboard) {
            result.push({
                label: t("electionScreen.tabs.dashboard"),
                component: DashboardTab,
            })
        }

        if (showData) {
            result.push({
                label: t("electionScreen.tabs.data"),
                component: DataTab,
            })
        }

        if (showVoters) {
            result.push({
                label: t("electionScreen.tabs.voters"),
                component: VotersTab,
            })
        }

        if (showPublish) {
            result.push({
                label: t("electionScreen.tabs.publish"),
                component: PublishTab,
                action: (index: number) => {
                    localStorage.setItem("electionPublishTabIndex", index.toString())
                },
            })
        }

        if (showApprovalsExecution) {
            result.push({
                label: t("electionScreen.tabs.approvals"),
                component: ApprovalsTab,
            })
        }

        return result
    }, [showDashboard, showData, showVoters, showPublish, showApprovalsExecution, t])

    if (!record || !hasPermissionToViewElection) {
        return (
            <ResourceListStyles.EmptyBox>
                <Typography variant="h4" paragraph>
                    {t("electionScreen.common.noPermission")}
                </Typography>
            </ResourceListStyles.EmptyBox>
        )
    }

    return (
        <Box
            sx={{
                maxWidth: `calc(100vw - ${open ? "352px" : "96px"})`,
                bgcolor: "background.paper",
            }}
            className="election-box"
        >
            <ElectionHeader
                title={
                    translateElection(record, "alias", i18n.language) ||
                    translateElection(record, "name", i18n.language) ||
                    record.alias ||
                    record.name ||
                    "-"
                }
                subtitle="electionScreen.common.subtitle"
            />
            <Box sx={{bgcolor: "background.paper"}}>
                <RecordContextProvider value={record}>
                    <Tabs elements={tabs} />
                </RecordContextProvider>
            </Box>
        </Box>
    )
}
