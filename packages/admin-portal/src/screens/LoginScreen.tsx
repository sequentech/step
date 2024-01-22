// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Box} from "@mui/system"
import React, {useContext, useEffect} from "react"
import {AuthContext} from "../providers/AuthContextProvider"
import {useNavigate} from "react-router"
import {CircularProgress} from "@mui/material"
import {useTranslation} from "react-i18next"

export const LoginScreen: React.FC = () => {
    const authContext = useContext(AuthContext)
    const navigate = useNavigate()
    const {i18n} = useTranslation()

    useEffect(() => {
        if (authContext.isAuthenticated) {
            navigate(`/test`)
        }
    }, [authContext.isAuthenticated, navigate])

    useEffect(() => {
        const dir = i18n.dir(i18n.language)
        document.documentElement.dir = dir
    }, [i18n, i18n.language])

    return (
        <Box>
            <CircularProgress />
        </Box>
    )
}
