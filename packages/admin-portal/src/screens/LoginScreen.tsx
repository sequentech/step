// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Box} from "@mui/system"
import React, {useContext, useEffect} from "react"
import {AuthContext} from "../providers/AuthContextProvider"
import {useNavigate, useParams} from "react-router"
import {CircularProgress} from "@mui/material"
import {useGetList} from "react-admin"
import {Sequent_Backend_Tenant} from "@/gql/graphql"

export const LoginScreen: React.FC = () => {
    const authContext = useContext(AuthContext)
    const navigate = useNavigate()

    const params = useParams()

    // console.log("Login Screen")
    // console.log("Tenant ID: " + params.tenantId)

    // const {data} = useGetList("sequent_backend_tenant", {
    //     pagination: {page: 1, perPage: 10},
    //     sort: {field: "updated_at", order: "DESC"},
    //     filter: {is_active: true},
    // })

    useEffect(() => {
        if (!params.tenantId) {
            navigate(`/`)
            return
        }

        let currentTenantId = localStorage.getItem("selected-tenant-id")

        if (currentTenantId !== params.tenantId) {
            localStorage.setItem("selected-tenant-id", params.tenantId)

            if (authContext.isAuthenticated) {
                authContext.logout()
                navigate(`/`)
            }
        }

        navigate(`/`)
    }, [authContext.isAuthenticated, authContext.tenantId, params.tenantId, navigate])

    return (
        <Box>
            <CircularProgress />
        </Box>
    )
}
