// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext} from "react"
import {AuthContext} from "@/providers/AuthContextProvider"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {IPermissions} from "@/types/keycloak"
import Notifications from "./Notifications"

interface EditNotificationsProps {
    electionEventId: string
}
export const EditNotifications: React.FC<EditNotificationsProps> = ({electionEventId}) => {
    const authContext = useContext(AuthContext)
    const [tenantId] = useTenantStore()
    const showNotifications = authContext.isAuthorized(
        true,
        tenantId,
        IPermissions.NOTIFICATION_READ
    )

    if (!showNotifications) {
        return null
    }

    return <Notifications electionEventId={electionEventId} />
}
