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
    const {isAuthenticated, login} = useContext(AuthContext)

    useEffect(() => {
        if (!isAuthenticated && tenantId && eventId) {
            login(tenantId, eventId)
        } else if (authContext.isAuthenticated) {
            navigate(`/tenant/${tenantId}/event/${eventId}/election-chooser`)
        }
    }, [authContext.isAuthenticated, navigate, isAuthenticated, tenantId, eventId, login])

    return (
        <Box>
            <CircularProgress />
            {
                // TODO: Handle error no login
            }
        </Box>
    )
}
