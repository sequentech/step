// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Box} from "@mui/material"
import React from "react"
import { ListUsers } from "../resources/User/ListUsers"

export const UserAndRoles: React.FC = () => <Box>
    <p>User and roles</p>
    <ListUsers />
</Box>
