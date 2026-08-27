// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Box} from "@mui/system"
import React, {useContext, useEffect} from "react"
import {AuthContext} from "../providers/AuthContextProvider"
import {useLocation, useNavigate, useParams} from "react-router-dom"
import {CircularProgress} from "@mui/material"
import {useTranslation} from "react-i18next"
import {TenantEventType} from ".."

const LoginScreen: React.FC = () => {
    const {t} = useTranslation()
    const {tenantId, eventId} = useParams<TenantEventType>()
    const navigate = useNavigate()
    const location = useLocation()
    const {isAuthenticated} = useContext(AuthContext)

    useEffect(() => {
        if (isAuthenticated) {
            navigate(`/tenant/${tenantId}/event/${eventId}/election-chooser${location.search}`)
        }
    }, [navigate, isAuthenticated, tenantId, eventId, location.search])

    return (
        <Box>
            <CircularProgress aria-label={t("a11y.loading")} />
            {
                // TODO: Handle error no login
            }
        </Box>
    )
}

export default LoginScreen
