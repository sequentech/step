// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {ReactElement, Suspense, useContext, useEffect, useState} from "react"

import {useTranslation} from "react-i18next"
import {useRecordContext} from "react-admin"
import {v4 as uuidv4} from "uuid"

import {AuthContext} from "@/providers/AuthContextProvider"
import ElectionHeader from "@/components/ElectionHeader"
import {IPermissions} from "@/types/keycloak"
import {EElectionEventLockedDown} from "@sequentech/ui-core"
import {ElectoralLogConversation} from "./ElectoralLogConversation"
import {ElectoralLogList} from "./ElectoralLogList"
import {Sequent_Backend_Election_Event} from "@/gql/graphql"
import {Tabs} from "@/components/Tabs"

export interface ElectoralLogTabsProps {
    aside?: ReactElement
    filterToShow?: ElectoralLogFilters
    filterValue?: string
    showActions?: boolean
}

export enum ElectoralLogFilters {
    ID = "id",
    STATEMENT_KIND = "statement_kind",
    USER_ID = "user_id",
}

export const ElectoralLogTabs: React.FC<ElectoralLogTabsProps> = ({
    aside,
    filterToShow,
    filterValue,
    showActions,
}) => {
    const record = useRecordContext<Sequent_Backend_Election_Event>()
    const {t} = useTranslation()
    // const [tabKey, setTabKey] = React.useState<string>(uuidv4())
    // const authContext = useContext(AuthContext)
    // const usersPermissionLabels = authContext.permissionLabels
    // const [hasPermissionToViewElection, setHasPermissionToViewElection] = useState<boolean>(true)

    // const isElectionEventLocked =
    //     record?.presentation?.locked_down == EElectionEventLockedDown.LOCKED_DOWN

    const showList = true
    const showConversation = true

    // const showDashboard = authContext.isAuthorized(
    //     true,
    //     authContext.tenantId,
    //     IPermissions.ADMIN_DASHBOARD_VIEW
    // )
    // const showData = authContext.isAuthorized(
    //     true,
    //     authContext.tenantId,
    //     IPermissions.ELECTION_DATA_TAB
    // )

    // useEffect(() => {
    //     if (
    //         usersPermissionLabels &&
    //         record?.permission_label &&
    //         !usersPermissionLabels.includes(record.permission_label)
    //     ) {
    //         setHasPermissionToViewElection(false)
    //     } else {
    //         setHasPermissionToViewElection(true)
    //     }
    // }, [record])

    // if (!hasPermissionToViewElection) {
    //     return (
    //         <ResourceListStyles.EmptyBox>
    //             <Typography variant="h4" paragraph>
    //                 {t("electionScreen.common.noPermission")}
    //             </Typography>
    //         </ResourceListStyles.EmptyBox>
    //     )
    // }

    return (
        <Tabs
            elements={[
                ...(showList
                    ? [
                          {
                              label: t("logsScreen.title"),
                              component: () => (
                                  <Suspense fallback={<div>Loading Dashboard...</div>}>
                                      <ElectoralLogList
                                          showActions={showActions}
                                          filterToShow={filterToShow}
                                          filterValue={filterValue}
                                      />
                                  </Suspense>
                              ),
                          },
                      ]
                    : []),

                ...(showConversation
                    ? [
                          {
                              label: t("logsScreen.conversation"),
                              component: () => (
                                  <Suspense fallback={<div>Loading Dashboard...</div>}>
                                      <ElectoralLogConversation
                                          showActions={showActions}
                                          filterToShow={filterToShow}
                                          filterValue={filterValue}
                                      />
                                  </Suspense>
                              ),
                          },
                      ]
                    : []),
            ]}
        />
    )
}
