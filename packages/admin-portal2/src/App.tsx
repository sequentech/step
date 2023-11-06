// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {Routes, Route} from "react-router-dom"
import {styled} from "@mui/material/styles"
import Stack from "@mui/material/Stack"
import Screen from "./Screen"

const StyledApp = styled(Stack)`
    min-height: 100vh;
`

const App: React.FC = () => {
    return (
        <StyledApp>
                <Routes>
                    <Route path="/" element={<Screen />} />
                </Routes>
        </StyledApp>
    )
}

export default App
