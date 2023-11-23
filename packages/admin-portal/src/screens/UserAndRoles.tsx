// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Box} from "@mui/material"
import React, {useContext} from "react"
import {ListUsers} from "../resources/User/ListUsers"
import {AuthContext} from "../providers/AuthContextProvider"

export const UserAndRoles: React.FC = () => {
    const authContext = useContext(AuthContext)
    const showUsers = authContext.hasPermissions(false, authContext.tenantId, "read-users")

    return (
        <Box>
            <p>User and roles</p>
            {showUsers ? <ListUsers /> : null}
        </Box>
    )
}
