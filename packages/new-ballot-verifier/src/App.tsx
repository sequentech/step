// SPDX-FileCopyrightText: 2022 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState} from "react"
import {Routes, Route} from "react-router-dom"
import NewBallotVerifierLib, {set_hooks} from "new-ballot-verifier-lib"
import {styled} from "@mui/material/styles"
import {Footer, Header, PageBanner} from "ui-essentials"
import {HomeScreen} from "./screens/HomeScreen"
import {ConfirmationScreen} from "./screens/ConfirmationScreen"
import Stack from "@mui/material/Stack"
import {IConfirmationBallot, provideBallotService} from "./services/BallotService"

NewBallotVerifierLib().then(set_hooks)

const StyledApp = styled(Stack)`
    min-height: 100vh;
`

const App = () => {
    const [confirmationBallot, setConfirmationBallot] = useState<IConfirmationBallot | null>(null)
    const [ballotId, setBallotId] = useState<string>("")
    const [fileName, setFileName] = useState("")
    const ballotService = provideBallotService()

    return (
        <StyledApp>
            <Header />
            <PageBanner marginBottom="auto">
                <Routes>
                    <Route
                        path="/"
                        element={
                            <HomeScreen
                                confirmationBallot={confirmationBallot}
                                setConfirmationBallot={setConfirmationBallot}
                                ballotId={ballotId}
                                setBallotId={setBallotId}
                                fileName={fileName}
                                setFileName={setFileName}
                                ballotService={ballotService}
                            />
                        }
                    />
                    <Route
                        path="/confirmation"
                        element={
                            <ConfirmationScreen
                                confirmationBallot={confirmationBallot}
                                ballotId={ballotId}
                                ballotService={ballotService}
                            />
                        }
                    />
                </Routes>
            </PageBanner>
            <Footer />
        </StyledApp>
    )
}

export default App
