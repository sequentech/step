// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Box} from "@mui/system"
import React, {useContext, useEffect} from "react"
import {AuthContext} from "../providers/AuthContextProvider"
import {useNavigate} from "react-router-dom"
import {CircularProgress} from "@mui/material"
import {TenantEventContext} from ".."

export const LoginScreen: React.FC = () => {
    const authContext = useContext(AuthContext)
    const {tenantId, eventId} = useContext(TenantEventContext)
    const navigate = useNavigate()

    useEffect(() => {
        if (authContext.isAuthenticated) {
            console.log(`navigate to: /start`)
            navigate(`/tenant/${tenantId}/event/${eventId}/start`)
        }
    }, [authContext.isAuthenticated, navigate])

    return (
        <Box>
            <CircularProgress />
        </Box>
    )
}
