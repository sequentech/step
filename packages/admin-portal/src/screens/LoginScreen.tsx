// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Box} from "@mui/system"
import React, {useContext, useEffect, useState} from "react"
import {AuthContext} from "../providers/AuthContextProvider"
import {useNavigate, useParams} from "react-router"
import {CircularProgress} from "@mui/material"
import {SettingsContext} from "../providers/SettingsContextProvider"

export const LoginScreen: React.FC = () => {
    const authContext = useContext(AuthContext)
    const {globalSettings} = useContext(SettingsContext)
    const navigate = useNavigate()

    const params = useParams()

    function fetchGraphQL(
        operationsDoc: string,
        operationName: string,
        variables: Record<string, any>
    ) {
        return fetch(globalSettings.HASURA_URL, {
            method: "POST",
            body: JSON.stringify({
                query: operationsDoc,
                variables,
                operationName,
            }),
        }).then((result) => result.json())
    }

    const operation = `
        query GetTenant($tenant_name: String) {
          sequent_backend_tenant(where: {slug: {_eq: $tenant_name}}) {
            id
            slug
          }
        }
      `

    function fetchGetTenant() {
        return fetchGraphQL(operation, "GetTenant", {tenant_name: params.tenantId})
    }

    useEffect(() => {
        const getTenant = async () => {
            fetchGetTenant()
                .then(({data, errors}) => {
                    if (errors) {
                        console.error(errors)
                    }
                    let currentTenantId = localStorage.getItem("selected-tenant-id")
                    let result = data.sequent_backend_tenant

                    if (result.length == 1 && currentTenantId !== result[0].id) {
                        localStorage.setItem("selected-tenant-id", result[0].id)

                        if (authContext.isAuthenticated) {
                            authContext.logout()
                            navigate(`/`)
                        }
                    }
                })
                .catch((error) => {
                    console.error(error)
                })
        }

        if (!params.tenantId) {
            return
        }

        getTenant()

        navigate(`/`)
    }, [authContext.isAuthenticated, authContext.tenantId, params.tenantId, navigate])

    return (
        <Box>
            <CircularProgress />
        </Box>
    )
}
