// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import ReactDOM from "react-dom/client"
import {createBrowserRouter, RouterProvider} from "react-router-dom"
import {ThemeProvider} from "@mui/material"
import {theme} from "@sequentech/ui-essentials"
import {useTranslation} from "react-i18next"
import "./services/i18n"
import "./index.css"
import {SettingsWrapper} from "@/providers/SettingsContextProvider"
import {CustomCssContextProvider} from "@/providers/CustomCssContextProvider"
import {ResultsManifestContextProvider} from "@/providers/ResultsManifestContextProvider"
import {ResultsAuthContextProvider} from "@/providers/ResultsAuthContextProvider"
import {App} from "@/App"
import {ResultsRoute} from "@/routes/ResultsRoute"
import {StateMessage} from "@/components/StateMessage"

const UnexpectedResultsError: React.FC = () => {
    const {t} = useTranslation()

    return (
        <StateMessage
            title={t("resultsPortal.state.unexpectedErrorTitle")}
            message={t("resultsPortal.state.loadErrorMessage")}
        />
    )
}

const ResultsNotPublished: React.FC = () => {
    const {t} = useTranslation()

    return (
        <StateMessage
            title={t("resultsPortal.state.notPublishedTitle")}
            message={t("resultsPortal.state.notPublishedMessage")}
        />
    )
}

const router = createBrowserRouter(
    [
        {
            path: "/",
            element: <App />,
            errorElement: <UnexpectedResultsError />,
            children: [
                {
                    path: ":eeId",
                    element: <ResultsRoute />,
                },
                {
                    path: ":eeId/elections/:electionId",
                    element: <ResultsRoute />,
                },
                {
                    path: "*",
                    element: <ResultsNotPublished />,
                },
            ],
        },
    ],
    {basename: "/"}
)

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
    <React.StrictMode>
        <SettingsWrapper>
            <CustomCssContextProvider>
                <ResultsManifestContextProvider>
                    <ResultsAuthContextProvider>
                        <ThemeProvider theme={theme}>
                            <RouterProvider router={router} />
                        </ThemeProvider>
                    </ResultsAuthContextProvider>
                </ResultsManifestContextProvider>
            </CustomCssContextProvider>
        </SettingsWrapper>
    </React.StrictMode>
)
