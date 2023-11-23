// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Box} from "@mui/material"
import React, {useContext} from "react"
import {ListUsers} from "../resources/User/ListUsers"
import {AuthContext} from "../providers/AuthContextProvider"
import { IPermissions } from "sequent-core"
import { useTenantStore } from "../providers/TenantContextProvider"

export const UserAndRoles: React.FC = () => {
    const authContext = useContext(AuthContext)
    const [tenantId] = useTenantStore()
    
    const showUsers = authContext.isAuthorized(true, tenantId, IPermissions.USER_READ)

    return (
        <Box>
            <p>User and roles</p>
            {showUsers ? <ListUsers /> : null}
        </Box>
    )
}
