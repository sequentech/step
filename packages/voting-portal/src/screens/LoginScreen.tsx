// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Box} from "@mui/system"
import React, {useContext, useEffect} from "react"
import {AuthContext} from "../providers/AuthContextProvider"
import {useNavigate} from "react-router-dom"
import {CircularProgress} from "@mui/material"
import { TenantEventContext } from ".."

export const LoginScreen: React.FC = () => {
    const authContext = useContext(AuthContext)
    const {tenantId, eventId} = useContext(TenantEventContext)
    const navigate = useNavigate()

    useEffect(() => {
        if (!authContext.isAuthenticated && tenantId && eventId) {
            console.log(`login: not authenticated, calling login with tenantId=${tenantId}, eventId=${eventId}`)
            authContext.login(tenantId, eventId)
        }
    }, [authContext.isAuthenticated, tenantId, eventId])

    useEffect(() => {
        if (authContext.isAuthenticated) {
            console.log(`navigate to: /tenant/${tenantId}/event/${eventId}/election-chooser`)
            navigate(`/tenant/${tenantId}/event/${eventId}/election-chooser`)
        }
    }, [authContext.isAuthenticated, navigate])

    return (
        <Box>
            <CircularProgress />
        </Box>
    )
}
