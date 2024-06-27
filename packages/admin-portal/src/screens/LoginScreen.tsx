// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Box} from "@mui/system"
import React, {useContext, useEffect} from "react"
import {AuthContext} from "../providers/AuthContextProvider"
import {useNavigate, useParams} from "react-router"
import {CircularProgress} from "@mui/material"

export const LoginScreen: React.FC = () => {
    const authContext = useContext(AuthContext)
    const navigate = useNavigate()

    const params = useParams()

    console.log("Login Screen")
    console.log("Tenant ID: " + params.tenantId)

    useEffect(() => {
        if (!params.tenantId) {
            navigate(`/`)
            return
        }

        if (authContext.isAuthenticated) {
            // navigate(`/test`)
            if (authContext.tenantId !== params.tenantId) {
                localStorage.setItem("selected-tenant-id", params.tenantId)
                authContext.logout()
                navigate(`/`)
            }
        }
    }, [authContext.isAuthenticated, navigate])

    return (
        <Box>
            <CircularProgress />
        </Box>
    )
}
