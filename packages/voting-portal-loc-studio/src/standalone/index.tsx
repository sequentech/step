// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import ReactDOM from "react-dom/client"
import {ThemeProvider} from "@mui/material"
import {theme} from "@sequentech/ui-essentials"
import {WasmWrapper} from "@voting-portal/providers/WasmWrapper"
import {initializeLocStudioI18n} from "../i18n"
import {LocStudioProvider} from "../LocStudioContext"
import {LocStudioApp} from "../App"
import "@voting-portal/index.css"
import "../loc-studio.css"

initializeLocStudioI18n()

const root = ReactDOM.createRoot(document.getElementById("root") as HTMLElement)

root.render(
    <React.StrictMode>
        <WasmWrapper>
            <ThemeProvider theme={theme}>
                <LocStudioProvider>
                    <LocStudioApp />
                </LocStudioProvider>
            </ThemeProvider>
        </WasmWrapper>
    </React.StrictMode>
)
