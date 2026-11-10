// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Header, adminTheme} from "@sequentech/ui-essentials"
import React, {useContext, useEffect, useState} from "react"
import {AppBar, useGetOne} from "react-admin"
import {AuthContext} from "../providers/AuthContextProvider"
import {
    getValueFromCookie,
    ITenantSettings,
    setCookie,
    shortAppHash,
    USER_LANGUAGE_COOKIE_NAME,
} from "@sequentech/ui-core"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {TenantContext} from "@/providers/TenantContextProvider"
import {Sequent_Backend_Tenant} from "@/gql/graphql"
import SequentLogo from "@sequentech/ui-essentials/public/Sequent_logo.svg"
import BlankLogoImg from "@sequentech/ui-essentials/public/blank_logo.svg"
import {useMatch} from "react-router-dom"
import {WaitingForMySignature} from "@/resources/ElectionEvent/Signatures/WaitingForMySignature"
import {adminPortalLogoUrl} from "@/services/tenantLookAndFeel"

export const CustomAppBar: React.FC = () => {
    const authContext = useContext(AuthContext)
    const eventRoute = useMatch("/sequent_backend_election_event/:eventId/*")
    const eventId = eventRoute?.params.eventId
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

    // The tenant's logo, where its staff screens may show it (see the helper).
    const tenantLogoUrl = adminPortalLogoUrl(tenant)
    const [logoUrl, setLogoUrl] = useState<string | undefined>(tenantLogoUrl)

    const [logoImg, setLogoImg] = useState<string | undefined>(BlankLogoImg)

    useEffect(() => {
        setLogoImg(logoUrl ?? BlankLogoImg)
    }, [])

    /*  Without a tenant logo to show (none set, or the staff screens keep the
        built-in logo) the Sequent logo must be shown. But while data isn't
        fetched yet there is no tenant logo either, and a blank logo must be shown.
    */
    useEffect(() => {
        setLogoUrl(tenantLogoUrl)
        if (!isFetching) {
            setLogoImg(tenantLogoUrl ?? SequentLogo)
        }
    }, [tenantLogoUrl, logoUrl, isFetching])

    const onChangeLanguage = (lang: string) => {
        if (getValueFromCookie(USER_LANGUAGE_COOKIE_NAME) !== lang) {
            setCookie(USER_LANGUAGE_COOKIE_NAME, lang)
        }
    }

    const appHash = shortAppHash(globalSettings.APP_HASH)

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
                "& .header-class": {flex: 1, minWidth: 0},
                "& .header-logo-link": {minWidth: 0},
                "& .header-logo": {maxWidth: "100%"},
                "& .header-logo img": {objectFit: "contain !important"},
                "& .header-actions": {gap: {xs: 1, lg: "31px"}},
                "boxShadow": "unset",
            }}
        >
            <Header
                appVersion={{main: globalSettings.APP_VERSION}}
                appHash={appHash ? {main: appHash} : undefined}
                appHashDesktopOnly
                userProfile={{
                    firstName: authContext.firstName,
                    username: authContext.username,
                    email: authContext.email,
                }}
                logoutFn={authContext.isAuthenticated ? authContext.logout : undefined}
                languagesList={langList}
                logoUrl={logoImg}
                onChangeLanguage={onChangeLanguage}
                actions={
                    eventId && eventId !== "create" ? (
                        <WaitingForMySignature
                            key={eventId}
                            electionEventId={eventId}
                            presentation="header"
                        />
                    ) : undefined
                }
            />
        </AppBar>
    )
}
