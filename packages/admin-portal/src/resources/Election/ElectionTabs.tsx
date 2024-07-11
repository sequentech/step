// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext} from "react"

import {useTranslation} from "react-i18next"
import {TabbedShowLayout, useRecordContext} from "react-admin"

import {AuthContext} from "@/providers/AuthContextProvider"
import ElectionHeader from "@/components/ElectionHeader"
import DashboardElection from "@/components/dashboard/election/Dashboard"
import {Sequent_Backend_Election} from "@/gql/graphql"

import {Publish} from "../Publish/Publish"
import {EditElectionData} from "./ElectionData"
import {EPublishType} from "../Publish/EPublishType"
import {IPermissions} from "@/types/keycloak"
import {EditElectionEventUsers} from "../ElectionEvent/EditElectionEventUsers"
import {ViewMode, ViewModeContext} from "@/providers/ViewModeContextProvider"

export const ElectionTabs: React.FC = () => {
    const record = useRecordContext<Sequent_Backend_Election>()
    const {t} = useTranslation()
    const authContext = useContext(AuthContext)
    const showVoters = authContext.isAuthorized(true, authContext.tenantId, IPermissions.VOTER_READ)
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
    const showPublish = authContext.isAuthorized(
        true,
        authContext.tenantId,
        IPermissions.PUBLISH_READ
    )
    const {setViewMode} = useContext(ViewModeContext)

    return (
        <>
            <ElectionHeader title={record?.name} subtitle="electionScreen.common.subtitle" />
            <TabbedShowLayout>
                {showDashboard && (
                    <TabbedShowLayout.Tab label={t("electionScreen.tabs.dashboard")}>
                        <DashboardElection />
                    </TabbedShowLayout.Tab>
                )}
                {showDashboard && (
                    <TabbedShowLayout.Tab label={t("electionScreen.tabs.data")}>
                        <EditElectionData />
                    </TabbedShowLayout.Tab>
                )}
                {showVoters && (
                    <TabbedShowLayout.Tab label={t("electionEventScreen.tabs.voters")}>
                        <EditElectionEventUsers
                            electionEventId={record?.election_event_id}
                            electionId={record?.id}
                        />
                    </TabbedShowLayout.Tab>
                )}
                {showPublish && (
                    <TabbedShowLayout.Tab
                        label={t("electionScreen.tabs.publish")}
                        onClick={() => setViewMode(ViewMode.List)}
                    >
                        <Publish
                            electionEventId={record?.election_event_id}
                            electionId={record?.id}
                            type={EPublishType.Election}
                        />
                    </TabbedShowLayout.Tab>
                )}
            </TabbedShowLayout>
        </>
    )
}
