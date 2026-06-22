// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import ReactDOM from "react-dom/client"
import {createBrowserRouter, RouterProvider} from "react-router-dom"
import {ThemeProvider} from "@mui/material"
import {theme} from "@sequentech/ui-essentials"
import "./services/i18n"
import "./index.css"
import {SettingsWrapper} from "@/providers/SettingsContextProvider"
import {App} from "@/App"
import {ResultsRoute} from "@/routes/ResultsRoute"
import {StateMessage} from "@/components/StateMessage"

const router = createBrowserRouter(
    [
        {
            path: "/",
            element: <App />,
            errorElement: (
                <StateMessage
                    title="Unexpected error"
                    message="We could not load results right now. Please try again in a few minutes."
                />
            ),
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
                    element: (
                        <StateMessage
                            title="Results not published yet"
                            message="Results are not available at this time. Please check back later."
                        />
                    ),
                },
            ],
        },
    ],
    {basename: "/"}
)

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
    <React.StrictMode>
        <SettingsWrapper>
            <ThemeProvider theme={theme}>
                <RouterProvider router={router} />
            </ThemeProvider>
        </SettingsWrapper>
    </React.StrictMode>
)
