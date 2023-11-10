// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import ReactDOM from "react-dom/client"
import "./index.css"
import App from "./App"
import "./services/i18n"
import reportWebVitals from "./reportWebVitals"
import {ThemeProvider} from "@mui/material"
import SequentCoreLibInit, {set_hooks} from "sequent-core"
//import AuthContextProvider from "./providers/AuthContextProvider"
import {ApolloProvider} from "@apollo/client"
import {apolloClient} from "./services/ApolloService"
import AuthContextProvider from "./providers/AuthContextProvider"
import {fullAdminTheme} from "./services/AdminTheme"

const root = ReactDOM.createRoot(document.getElementById("root") as HTMLElement)

SequentCoreLibInit().then(set_hooks)

root.render(
    <React.StrictMode>
        <AuthContextProvider>
            <ThemeProvider theme={fullAdminTheme}>
                <ApolloProvider client={apolloClient}>
                    <App />
                </ApolloProvider>
            </ThemeProvider>
        </AuthContextProvider>
    </React.StrictMode>
)

// If you want to start measuring performance in your app, pass a function
// to log results (for example: reportWebVitals(console.log))
// or send to an analytics endpoint. Learn more: https://bit.ly/CRA-vitals
reportWebVitals()
