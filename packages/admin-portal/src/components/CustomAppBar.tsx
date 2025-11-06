// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Header, adminTheme} from "@sequentech/ui-essentials"
import React, {useContext, useEffect, useState} from "react"
import {AppBar, useGetOne} from "react-admin"
import {AuthContext} from "../providers/AuthContextProvider"
import {ITenantSettings, ITenantTheme} from "@sequentech/ui-core"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {TenantContext} from "@/providers/TenantContextProvider"
import {Sequent_Backend_Tenant} from "@/gql/graphql"
import SequentLogo from "@sequentech/ui-essentials/public/Sequent_logo.svg"
import BlankLogoImg from "@sequentech/ui-essentials/public/blank_logo.svg"

export const CustomAppBar: React.FC = () => {
    const authContext = useContext(AuthContext)
    const {globalSettings} = useContext(SettingsContext)
    const {tenantId, tenant, setTenant} = useContext(TenantContext)
    const {data: tenantData} = useGetOne<Sequent_Backend_Tenant>("sequent_backend_tenant", {
        id: tenantId,
    })

    const [isFetching, setIsFetching] = useState(true)

    useEffect(() => {
        if (tenantData) {
            setTenant(tenantData)
            setIsFetching(false)
        }
    }, [tenantData])

    const langList = (tenant?.settings as ITenantSettings | undefined)?.language_conf
        ?.enabled_language_codes ?? ["en"]

    const [logoUrl, setLogoUrl] = useState<string | undefined | null>(
        (tenant?.annotations as ITenantTheme | undefined)?.logo_url
    )

    const [logoImg, setLogoImg] = useState<string | undefined>(BlankLogoImg)

    useEffect(() => {
        setLogoImg(logoUrl ?? BlankLogoImg)
    }, [])

    /*  When tenant annotations column is empty annotations.logo_url will be 
        undefined and in that case Sequent logo must be shown.
        But while data isn't fetched yet annotations.logo_url is also undefined 
        and a blank logo must be shown.
    */
    useEffect(() => {
        const newLogoState = (tenant?.annotations as ITenantTheme | undefined)?.logo_url
        setLogoUrl(newLogoState)
        if (!isFetching) {
            setLogoImg(newLogoState ?? SequentLogo)
        }
    }, [(tenant?.annotations as ITenantTheme | undefined)?.logo_url, logoUrl, isFetching])

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
                appHash={{main: globalSettings.APP_HASH}}
                userProfile={{
                    firstName: authContext.firstName,
                    username: authContext.username,
                    email: authContext.email,
                    openLink: authContext.openProfileLink,
                }}
                logoutFn={authContext.isAuthenticated ? authContext.logout : undefined}
                languagesList={langList}
                logoUrl={logoImg}
            />
        </AppBar>
    )
}
