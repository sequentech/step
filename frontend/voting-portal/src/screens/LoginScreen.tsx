// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import { Box } from "@mui/system"
import React, { useContext, useEffect } from "react"
import { AuthContext } from "../providers/AuthContextProvider"
import { useNavigate } from "react-router"

export const LoginScreen: React.FC = () => {
    const authContext = useContext(AuthContext)
    const navigate = useNavigate()

    useEffect(() => {
        if (authContext.isAuthenticated) {
            navigate(`/election-chooser`)
        }
    }, [authContext.isAuthenticated])

    return <Box>
        {
        !authContext.isAuthenticated
          ? <p>Not Authenticated</p>
          : <p>User Authenticated</p>
        }       
    </Box>
}