// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {Suspense, useContext, useEffect, useState} from "react"

import {useTranslation} from "react-i18next"
import {TabbedShowLayout, useRecordContext, useSidebarState, Identifier} from "react-admin"
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
import {TallySheetWizard, WizardSteps} from "../TallySheet/TallySheetWizard"
import {Sequent_Backend_Contest} from "../../gql/graphql"
import {ListTallySheet} from "../TallySheet/ListTallySheet"

export const ElectionTabs: React.FC = () => {
    const electionRecord = useRecordContext<Sequent_Backend_Election>()
    const {t} = useTranslation()
    const [tabKey, setTabKey] = React.useState<string>(uuidv4())
    const authContext = useContext(AuthContext)
    const usersPermissionLabels = authContext.permissionLabels
    const [hasPermissionToViewElection, setHasPermissionToViewElection] = useState<boolean>(true)
    const [open] = useSidebarState()
    const [action, setAction] = useState<number>(WizardSteps.List)
    const [refresh, setRefresh] = useState<string | null>(null)
    const [tallySheetId, setTallySheetId] = useState<Identifier | undefined>()
    // const contestRecord = useRecordContext<Sequent_Backend_Contest>()
    
    const handleAction = (action: number, id?: Identifier) => {
        setAction(action)
        setRefresh(new Date().getTime().toString())
        if (id) {
            setTallySheetId(id)
        }
    }
    const isElectionEventLocked =
        electionRecord?.presentation?.locked_down == EElectionEventLockedDown.LOCKED_DOWN

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
            electionRecord?.permission_label &&
            !usersPermissionLabels.includes(electionRecord.permission_label)
        ) {
            setHasPermissionToViewElection(false)
        } else {
            setHasPermissionToViewElection(true)
        }
    }, [electionRecord])

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
                    translateElection(electionRecord, "alias", i18n?.language) ||
                    translateElection(electionRecord, "name", i18n?.language) ||
                    electionRecord?.alias ||
                    electionRecord?.name ||
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
                                              electionEventId={electionRecord?.election_event_id}
                                              electionId={electionRecord?.id}
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
                                              electionEventId={electionRecord?.election_event_id}
                                              electionId={electionRecord?.id}
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
                                              electionEventId={electionRecord?.election_event_id}
                                              electionId={electionRecord?.id}
                                          />
                                      </Suspense>
                                  ),
                              },
                          ]
                        : []),
                    ...(
                        [
                            {
                                label: t("electionScreen.tabs.tallySheets"),
                                component: () => (
                                    <Suspense fallback={<div>Loading {t("electionScreen.tabs.tallySheets")} ...</div>}>
                                        <ElectionHeader title={t("tasksScreen.title")} subtitle="tasksScreen.subtitle" />
                                        {action === WizardSteps.List ? (
                                            <ListTallySheet election={electionRecord} doAction={handleAction} reload={refresh} />
                                        ) : action === WizardSteps.Start ? (
                                            <TallySheetWizard
                                                election={electionRecord}
                                                action={action}
                                                doAction={handleAction}
                                            />
                                        ) : action === WizardSteps.Edit ? (
                                            <TallySheetWizard
                                                tallySheetId={tallySheetId}
                                                election={electionRecord}
                                                action={action}
                                                doAction={handleAction}
                                            />
                                        ) : action === WizardSteps.Confirm ? (
                                            <TallySheetWizard
                                                tallySheetId={tallySheetId}
                                                election={electionRecord}
                                                action={action}
                                                doAction={handleAction}
                                            />
                                        ) : action === WizardSteps.View ? (
                                            <TallySheetWizard
                                                tallySheetId={tallySheetId}
                                                election={electionRecord}
                                                action={action}
                                                doAction={handleAction}
                                            />
                                        ) : null}
                                    </Suspense>
                                ),
                            },
                        ]
                        
                    ),
                ]}
            />
        </Box>
    )
}
