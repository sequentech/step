// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext, useEffect, useState} from "react"

import {useTranslation} from "react-i18next"
import {TabbedShowLayout, useRecordContext} from "react-admin"
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
import {Typography} from "@mui/material"
import {EElectionEventLockedDown} from "@sequentech/ui-core"

export const ElectionTabs: React.FC = () => {
    const record = useRecordContext<Sequent_Backend_Election>()
    const {t} = useTranslation()
    const [tabKey, setTabKey] = React.useState<string>(uuidv4())
    const authContext = useContext(AuthContext)
    const usersPermissionLabels = authContext.permissionLabels
    const [hasPermissionToViewElection, setHasPermissionToViewElection] = useState<boolean>(true)

    const isElectionEventLocked =
        record?.presentation?.locked_down == EElectionEventLockedDown.LOCKED_DOWN

    const showDashboard = authContext.isAuthorized(
        true,
        authContext.tenantId,
        IPermissions.ADMIN_DASHBOARD_VIEW
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
        <>
            <ElectionHeader title={record?.name} subtitle="electionScreen.common.subtitle" />
            <TabbedShowLayout>
                {showDashboard && (
                    <TabbedShowLayout.Tab label={t("electionScreen.tabs.dashboard")}>
                        <DashboardElection />
                    </TabbedShowLayout.Tab>
                )}
                {showData && (
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
                        onClick={() => setTabKey(uuidv4())}
                    >
                        <Publish
                            key={tabKey}
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
