// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Box} from "@mui/system"
import React, {useContext, useEffect} from "react"
import {AuthContext} from "../providers/AuthContextProvider"
import {useNavigate, useParams} from "react-router-dom"
import {CircularProgress} from "@mui/material"
import {TenantEvent} from ".."

export const LoginScreen: React.FC = () => {
    const authContext = useContext(AuthContext)
    const {tenantId, eventId} = useParams<TenantEvent>()
    const navigate = useNavigate()

    useEffect(() => {
        if (authContext.isAuthenticated) {
            console.log(`navigate to: /tenant/${tenantId}/event/${eventId}/election-chooser`)
            navigate(`/tenant/${tenantId}/event/${eventId}/election-chooser`)
        }
    }, [authContext.isAuthenticated, navigate, tenantId, eventId])

    return (
        <Box>
            <CircularProgress />
        </Box>
    )
}
