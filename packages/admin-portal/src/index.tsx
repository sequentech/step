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
import {initCore} from "@sequentech/ui-core"
import AuthContextProvider from "./providers/AuthContextProvider"
import {fullAdminTheme} from "./services/AdminTheme"
import {TenantContextProvider} from "./providers/TenantContextProvider"
import {ElectionEventContextProvider} from "./providers/ElectionEventContextProvider"
import {ElectionContextProvider} from "./providers/ElectionContextProvider"
import {ContestContextProvider} from "./providers/ContestContextProvider"
import {CandidateContextProvider} from "./providers/CandidateContextProvider"
import {ElectionEventTallyContextProvider} from "./providers/ElectionEventTallyProvider"
import NewResourceContextProvider from "./providers/NewResourceProvider"
import {PublishContextProvider} from "./providers/PublishContextProvider"
import {SettingsWrapper} from "./providers/SettingsContextProvider"
import {
    ApolloContextProvider,
    ApolloWrapper,
    defaultApolloContextValues,
} from "./providers/ApolloContextProvider"
import {BrowserRouter as Router} from "react-router-dom" // Import BrowserRouter

const root = ReactDOM.createRoot(document.getElementById("root") as HTMLElement)

initCore()

root.render(
    <React.StrictMode>
        <Router>
            <SettingsWrapper>
                <AuthContextProvider>
                    <TenantContextProvider>
                        <NewResourceContextProvider>
                            <ElectionEventContextProvider>
                                <ElectionContextProvider>
                                    <ContestContextProvider>
                                        <CandidateContextProvider>
                                            <ElectionEventTallyContextProvider>
                                                <PublishContextProvider>
                                                    <ThemeProvider theme={fullAdminTheme}>
                                                        <ApolloContextProvider
                                                            role={defaultApolloContextValues.role}
                                                        >
                                                            <ApolloWrapper>
                                                                <App />
                                                            </ApolloWrapper>
                                                        </ApolloContextProvider>
                                                    </ThemeProvider>
                                                </PublishContextProvider>
                                            </ElectionEventTallyContextProvider>
                                        </CandidateContextProvider>
                                    </ContestContextProvider>
                                </ElectionContextProvider>
                            </ElectionEventContextProvider>
                        </NewResourceContextProvider>
                    </TenantContextProvider>
                </AuthContextProvider>
            </SettingsWrapper>
        </Router>
    </React.StrictMode>
)

reportWebVitals()
