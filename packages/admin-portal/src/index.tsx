// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import ReactDOM from "react-dom/client"
import "./index.css"
import {AppWrapper} from "./App"
import "./services/i18n"
import reportWebVitals from "./reportWebVitals"
import {ThemeProvider} from "@mui/material"
import SequentCoreLibInit, {set_hooks} from "sequent-core"
import AuthContextProvider from "./providers/AuthContextProvider"
import {fullAdminTheme} from "./services/AdminTheme"
import {TenantContextProvider} from "./providers/TenantContextProvider"
import {ElectionEventContextProvider} from "./providers/ElectionEventContextProvider"
import {ElectionContextProvider} from "./providers/ElectionContextProvider"
import {ContestContextProvider} from "./providers/ContestContextProvider"
import {CandidateContextProvider} from "./providers/CandidateContextProvider"
import { ElectionEventTallyContextProvider } from './providers/ElectionEventTallyProvider'

const root = ReactDOM.createRoot(document.getElementById("root") as HTMLElement)

SequentCoreLibInit().then(set_hooks)

root.render(
    <React.StrictMode>
        <AuthContextProvider>
            <TenantContextProvider>
                <ElectionEventContextProvider>
                    <ElectionContextProvider>
                        <ContestContextProvider>
                            <CandidateContextProvider>
                                <ElectionEventTallyContextProvider>
                                    <ThemeProvider theme={fullAdminTheme}>
                                        <AppWrapper />
                                    </ThemeProvider>
                                </ElectionEventTallyContextProvider>
                            </CandidateContextProvider>
                        </ContestContextProvider>
                    </ElectionContextProvider>
                </ElectionEventContextProvider>
            </TenantContextProvider>
        </AuthContextProvider>
    </React.StrictMode>
)

// If you want to start measuring performance in your app, pass a function
// to log results (for example: reportWebVitals(console.log))
// or send to an analytics endpoint. Learn more: https://bit.ly/CRA-vitals
reportWebVitals()
