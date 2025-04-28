// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {Suspense, useContext, useEffect, useState} from "react"

import {useTranslation} from "react-i18next"
import {TabbedShowLayout, useRecordContext, useSidebarState} from "react-admin"
import {v4 as uuidv4} from "uuid"

import {AuthContext} from "@/providers/AuthContextProvider"
import ElectionHeader from "@/components/ElectionHeader"
import DashboardElection from "@/components/dashboard/election/Dashboard"
import MonitoringDashboardElection from "@/components/monitoring-dashboard/election/MonitoringDashboard"
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

export const ElectionTabs: React.FC = () => {
    const record = useRecordContext<Sequent_Backend_Election>()
    const {t} = useTranslation()
    const [tabKey, setTabKey] = React.useState<string>(uuidv4())
    const authContext = useContext(AuthContext)
    const usersPermissionLabels = authContext.permissionLabels
    const [hasPermissionToViewElection, setHasPermissionToViewElection] = useState<boolean>(true)
    const [open] = useSidebarState()

    const isElectionEventLocked =
        record?.presentation?.locked_down == EElectionEventLockedDown.LOCKED_DOWN

    const showDashboard = authContext.isAuthorized(
        true,
        authContext.tenantId,
        IPermissions.ELECTION_DASHBOARD_TAB
    )

    const showMonitoringDashboard = authContext.isAuthorized(
        true,
        authContext.tenantId,
        IPermissions.MONITORING_DASHBOARD_VIEW_ELECTION
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
    }, [record])

    if (!hasPermissionToViewElection) {
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
            sx={{maxWidth: `calc(100vw - ${open ? "352px" : "96px"})`, bgcolor: "background.paper"}}
            className="election-box"
        >
            <ElectionHeader
                title={
                    translateElection(record, "alias", i18n?.language) ||
                    translateElection(record, "name", i18n?.language) ||
                    record?.alias ||
                    record?.name ||
                    "-"
                }
                subtitle="electionScreen.common.subtitle"
            />
            <Tabs
                elements={[
                    ...(showDashboard
                        ? [
                              {
                                  label: t("electionScreen.tabs.dashboard"),
                                  component: () => (
                                      <Suspense fallback={<div>Loading Dashboard...</div>}>
                                          <DashboardElection />
                                      </Suspense>
                                  ),
                              },
                          ]
                        : []),
                    ...(showMonitoringDashboard
                        ? [
                              {
                                  label: t("electionScreen.tabs.monitoring"),
                                  component: () => (
                                      <Suspense fallback={<div>Loading Dashboard...</div>}>
                                          <MonitoringDashboardElection />
                                      </Suspense>
                                  ),
                              },
                          ]
                        : []),
                    ...(showData
                        ? [
                              {
                                  label: t("electionScreen.tabs.data"),
                                  component: () => (
                                      <Suspense fallback={<div>Loading Data...</div>}>
                                          <EditElectionData />
                                      </Suspense>
                                  ),
                              },
                          ]
                        : []),
                    ...(showVoters
                        ? [
                              {
                                  label: t("electionScreen.tabs.voters"),
                                  component: () => (
                                      <Suspense fallback={<div>Loading Voters...</div>}>
                                          <EditElectionEventUsers
                                              electionEventId={record?.election_event_id}
                                              electionId={record?.id}
                                          />
                                      </Suspense>
                                  ),
                              },
                          ]
                        : []),
                    ...(showPublish
                        ? [
                              {
                                  label: t("electionScreen.tabs.publish"),
                                  component: () => (
                                      <Suspense fallback={<div>Loading Publish...</div>}>
                                          <Publish
                                              key={tabKey}
                                              electionEventId={record?.election_event_id}
                                              electionId={record?.id}
                                              type={EPublishType.Election}
                                          />
                                      </Suspense>
                                  ),
                                  action: (index: number) => {
                                      localStorage.setItem(
                                          "electionPublishTabIndex",
                                          index.toString()
                                      )
                                  },
                              },
                          ]
                        : []),
                    ...(showApprovalsExecution
                        ? [
                              {
                                  label: t("electionScreen.tabs.approvals"),
                                  component: () => (
                                      <Suspense fallback={<div>Loading Approvals...</div>}>
                                          <EditElectionEventApprovals
                                              electionEventId={record?.election_event_id}
                                              electionId={record?.id}
                                          />
                                      </Suspense>
                                  ),
                              },
                          ]
                        : []),
                ]}
            />
        </Box>
    )
}
