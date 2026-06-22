// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext} from "react"
import {Outlet, ScrollRestoration} from "react-router-dom"
import {Box, Stack} from "@mui/material"
import {styled} from "@mui/material/styles"
import {Footer, Header, HeaderErrorVariant, PageBanner} from "@sequentech/ui-essentials"
import {SettingsContext} from "@/providers/SettingsContextProvider"

const StyledApp = styled(Stack)`
    min-height: 100vh;
`

const StyledMain = styled(PageBanner)`
    flex: 1;
    align-items: stretch;
    background: #f7f8fa;
`

const AppHeader: React.FC = () => {
    const {globalSettings} = useContext(SettingsContext)

    return (
        <Header
            appVersion={{main: globalSettings.APP_VERSION}}
            appHash={{main: globalSettings.APP_HASH}}
            logoUrl="/Sequent_logo_small.png"
            logoLink="https://sequentech.io"
            languagesList={["en"]}
            errorVariant={HeaderErrorVariant.HIDE_PROFILE}
        />
    )
}

export const App: React.FC = () => (
    <StyledApp>
        <ScrollRestoration />
        <AppHeader />
        <StyledMain component="main" id="main-content" role="main">
            <Box sx={{width: "100%"}}>
                <Outlet />
            </Box>
        </StyledMain>
        <Footer />
    </StyledApp>
)
