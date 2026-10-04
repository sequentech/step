// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
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
import {DatabaseProvider} from "./providers/DatabaseProvider"
import {WidgetsContextProvider} from "./providers/WidgetsContextProvider"
import {SigningProvider} from "./components/signing/SigningProvider"
import {SigningAction} from "./lib/signing/types"
import type {ISigningPanelData} from "./lib/signing/api"
import {ClosedVotingCard} from "./resources/Publish/ClosedVotingCard"
import {
    ReportCompletionActions,
    TransmissionCompletionActions,
} from "./resources/Reports/ReportSigning"
import {BrowserRouter as Router} from "react-router-dom"

/** What a completed request of each protected action shows in its panel. */
const signingCompletionActions = {
    [SigningAction.TransmitResults]: (data: ISigningPanelData) => (
        <TransmissionCompletionActions data={data} />
    ),
    [SigningAction.CloseVoting]: (data: ISigningPanelData) => <ClosedVotingCard data={data} />,
    [SigningAction.GenerateElectionReturns]: (data: ISigningPanelData) => (
        <ReportCompletionActions data={data} />
    ),
    [SigningAction.GenerateReports]: (data: ISigningPanelData) => (
        <ReportCompletionActions data={data} />
    ),
}

const root = ReactDOM.createRoot(document.getElementById("root") as HTMLElement)

initCore()

root.render(
    <React.StrictMode>
        <Router>
            <SettingsWrapper>
                <ThemeProvider theme={fullAdminTheme}>
                    <AuthContextProvider>
                        <TenantContextProvider>
                            <NewResourceContextProvider>
                                <ElectionEventContextProvider>
                                    <ElectionContextProvider>
                                        <ContestContextProvider>
                                            <CandidateContextProvider>
                                                <ElectionEventTallyContextProvider>
                                                    <PublishContextProvider>
                                                        <ApolloContextProvider
                                                            role={defaultApolloContextValues.role}
                                                        >
                                                            <ApolloWrapper>
                                                                <DatabaseProvider>
                                                                    <WidgetsContextProvider>
                                                                        <SigningProvider
                                                                            completionActions={
                                                                                signingCompletionActions
                                                                            }
                                                                        >
                                                                            <App />
                                                                        </SigningProvider>
                                                                    </WidgetsContextProvider>
                                                                </DatabaseProvider>
                                                            </ApolloWrapper>
                                                        </ApolloContextProvider>
                                                    </PublishContextProvider>
                                                </ElectionEventTallyContextProvider>
                                            </CandidateContextProvider>
                                        </ContestContextProvider>
                                    </ElectionContextProvider>
                                </ElectionEventContextProvider>
                            </NewResourceContextProvider>
                        </TenantContextProvider>
                    </AuthContextProvider>
                </ThemeProvider>
            </SettingsWrapper>
        </Router>
    </React.StrictMode>
)

// If you want to start measuring performance in your app, pass a function
// to log results (for example: reportWebVitals(console.log))
// or send to an analytics endpoint. Learn more: https://bit.ly/CRA-vitals
reportWebVitals()
