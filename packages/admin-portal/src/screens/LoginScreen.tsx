// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Box} from "@mui/system"
import React, {useContext, useEffect} from "react"
import {AuthContext} from "../providers/AuthContextProvider"
import {useNavigate} from "react-router"
import {CircularProgress} from "@mui/material"

export const LoginScreen: React.FC = () => {
    const authContext = useContext(AuthContext)
    const navigate = useNavigate()

    useEffect(() => {
        if (authContext.isAuthenticated) {
            console.log("aa LOGIN?")

            // navigate(`/test`)
        }
    }, [authContext.isAuthenticated, navigate])

    return (
        <Box>
            <CircularProgress />
        </Box>
    )
}
