// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import ReactDOM from "react-dom/client"
import {Provider} from "react-redux"
import {store} from "./store/store"
import {BrowserRouter} from "react-router-dom"
import "./index.css"
import App from "./App"
import "./services/i18n"
import reportWebVitals from "./reportWebVitals"
import {ThemeProvider} from "@mui/material"
import {theme} from "@sequentech/ui-essentials"
import SequentCoreLibInit, {set_hooks} from "sequent-core"
//import AuthContextProvider from "./providers/AuthContextProvider"
import {ApolloProvider} from "@apollo/client"
import {apolloClient} from "./services/ApolloService"

const root = ReactDOM.createRoot(document.getElementById("root") as HTMLElement)

SequentCoreLibInit().then(set_hooks)

root.render(
    <React.StrictMode>
        {/*<AuthContextProvider>*/}
        <Provider store={store}>
            <BrowserRouter>
                <ThemeProvider theme={theme}>
                    <ApolloProvider client={apolloClient}>
                        <App />
                    </ApolloProvider>
                </ThemeProvider>
            </BrowserRouter>
        </Provider>
        {/*</AuthContextProvider>*/}
    </React.StrictMode>
)

// If you want to start measuring performance in your app, pass a function
// to log results (for example: reportWebVitals(console.log))
// or send to an analytics endpoint. Learn more: https://bit.ly/CRA-vitals
reportWebVitals()
