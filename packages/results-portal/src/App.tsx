// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext, useEffect, useMemo} from "react"
import {Outlet, ScrollRestoration} from "react-router-dom"
import {Box, Stack} from "@mui/material"
import {styled} from "@mui/material/styles"
import {Footer, Header, HeaderErrorVariant, PageBanner} from "@sequentech/ui-essentials"
import {getValueFromCookie, setCookie, USER_LANGUAGE_COOKIE_NAME} from "@sequentech/ui-core"
import {useTranslation} from "react-i18next"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {useCustomCss} from "@/providers/CustomCssContextProvider"
import {useResultsManifest} from "@/providers/ResultsManifestContextProvider"

const StyledApp = styled(Stack)`
    min-height: 100vh;
`

const StyledAppWrapper = styled(Stack, {
    shouldForwardProp: (prop) => prop !== "customCss",
})<{customCss: string}>`
    ${({customCss}) => customCss}
`

const StyledMain = styled(PageBanner)`
    flex: 1;
    align-items: stretch;
    background: #f7f8fa;
`

const AppHeader: React.FC = () => {
    const {globalSettings} = useContext(SettingsContext)
    const {i18n} = useTranslation()
    const {availableLanguages, defaultLocale} = useResultsManifest()
    const languagesList = useMemo(
        () => (availableLanguages.length > 0 ? availableLanguages : ["en"]),
        [availableLanguages]
    )
    const languagesKey = languagesList.join("|")

    useEffect(() => {
        const currentLanguage = i18n.resolvedLanguage ?? i18n.language
        if (languagesList.includes(currentLanguage)) {
            return
        }

        const fallbackLanguage = languagesList.includes(defaultLocale)
            ? defaultLocale
            : (languagesList[0] ?? "en")
        void i18n.changeLanguage(fallbackLanguage)
        if (getValueFromCookie(USER_LANGUAGE_COOKIE_NAME) !== fallbackLanguage) {
            setCookie(USER_LANGUAGE_COOKIE_NAME, fallbackLanguage)
        }
    }, [defaultLocale, i18n, languagesKey, languagesList])

    const onChangeLanguage = (lang: string) => {
        if (getValueFromCookie(USER_LANGUAGE_COOKIE_NAME) !== lang) {
            setCookie(USER_LANGUAGE_COOKIE_NAME, lang)
        }
    }

    return (
        <Box className="seq-results-portal-header">
            <Header
                appVersion={{main: globalSettings.APP_VERSION}}
                appHash={{main: globalSettings.APP_HASH}}
                logoUrl="/Sequent_logo_small.png"
                logoLink="https://sequentech.io"
                languagesList={languagesList}
                errorVariant={HeaderErrorVariant.HIDE_PROFILE}
                onChangeLanguage={onChangeLanguage}
            />
        </Box>
    )
}

export const App: React.FC = () => {
    const {customCss} = useCustomCss()

    return (
        <StyledAppWrapper className="seq-results-portal-css-wrapper" customCss={customCss}>
            <StyledApp className="seq-results-portal-app app-root">
                <ScrollRestoration />
                <AppHeader />
                <StyledMain
                    className="seq-results-portal-main"
                    component="main"
                    id="main-content"
                    role="main"
                >
                    <Box className="seq-results-portal-main__content" sx={{width: "100%"}}>
                        <Outlet />
                    </Box>
                </StyledMain>
                <Box className="seq-results-portal-footer">
                    <Footer />
                </Box>
            </StyledApp>
        </StyledAppWrapper>
    )
}
