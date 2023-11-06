// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import ReactDOM from "react-dom/client"
import {BrowserRouter} from "react-router-dom"
import "./index.css"
import App from "./App"
import {ThemeProvider} from "@mui/material"
import {theme} from "@sequentech/ui-essentials"
import SequentCoreLibInit, {set_hooks} from "sequent-core"

const root = ReactDOM.createRoot(document.getElementById("root") as HTMLElement)

SequentCoreLibInit().then(set_hooks)

root.render(
    <React.StrictMode>
                <BrowserRouter>
                    <ThemeProvider theme={theme}>
                            <App />
                    </ThemeProvider>
                </BrowserRouter>
    </React.StrictMode>
)

