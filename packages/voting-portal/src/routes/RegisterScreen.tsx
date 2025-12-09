// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Box} from "@mui/system"
import React, {useContext, useEffect} from "react"
import {AuthContext} from "../providers/AuthContextProvider"
import {useLocation, useNavigate, useParams} from "react-router-dom"
import {CircularProgress} from "@mui/material"
import {TenantEventType} from ".."

const RegisterScreen: React.FC = () => {
    const authContext = useContext(AuthContext)
    const {tenantId, eventId} = useParams<TenantEventType>()
    const navigate = useNavigate()
    const location = useLocation()
    const {isAuthenticated, setTenantEvent} = useContext(AuthContext)

    useEffect(() => {
        if (!isAuthenticated && tenantId && eventId) {
            setTenantEvent(tenantId, eventId, "register")
        } else if (authContext.isAuthenticated) {
            navigate(`/tenant/${tenantId}/event/${eventId}/election-chooser${location.search}`)
        }
    }, [authContext.isAuthenticated, navigate, isAuthenticated, tenantId, eventId, setTenantEvent])

    return (
        <Box>
            <CircularProgress />
            {
                // TODO: Handle error no login
            }
        </Box>
    )
}

export default RegisterScreen
