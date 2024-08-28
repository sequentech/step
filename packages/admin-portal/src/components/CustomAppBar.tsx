// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Header, adminTheme} from "@sequentech/ui-essentials"
import React, {useContext, useEffect} from "react"
import {AppBar, useGetOne} from "react-admin"
import {AuthContext} from "../providers/AuthContextProvider"
import {ITenantSettings} from "@sequentech/ui-core"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {TenantContext} from "@/providers/TenantContextProvider"
import {Sequent_Backend_Tenant} from "@/gql/graphql"
import LogoImg from "@sequentech/ui-essentials/public/Sequent_logo.svg"

export const CustomAppBar: React.FC = () => {
    const authContext = useContext(AuthContext)
    const {globalSettings} = useContext(SettingsContext)
    const {tenantId, tenant, setTenant} = useContext(TenantContext)
    const {data: tenantData} = useGetOne<Sequent_Backend_Tenant>("sequent_backend_tenant", {
        id: tenantId,
    })

    useEffect(() => {
        if (tenantData) {
            setTenant(tenantData)
        }
    }, [tenantData])

    const langList = (tenant?.settings as ITenantSettings | undefined)?.language_conf
        ?.enabled_language_codes ?? ["en"]
    return (
        <AppBar
            toolbar={<></>}
            position="static"
            sx={{
                "backgroundColor": adminTheme.palette.lightBackground,
                "color": "black",
                "& .MuiContainer-root.MuiContainer-maxWidthLg": {
                    maxWidth: "unset",
                },
                "boxShadow": "unset",
            }}
        >
            <Header
                appVersion={{main: globalSettings.APP_VERSION}}
                userProfile={{
                    username: authContext.username,
                    email: authContext.email,
                    openLink: authContext.openProfileLink,
                }}
                logoutFn={authContext.isAuthenticated ? authContext.logout : undefined}
                languagesList={langList}
                logoUrl={LogoImg}
            />
        </AppBar>
    )
}
