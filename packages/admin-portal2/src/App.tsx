// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useContext} from "react"
import {Routes, Route, useLocation} from "react-router-dom"
import {styled} from "@mui/material/styles"
import {Footer, Header, PageBanner} from "@sequentech/ui-essentials"
import Stack from "@mui/material/Stack"

const StyledApp = styled(Stack)`
    min-height: 100vh;
`

const App = () => {
    return (
        <StyledApp>
                <Routes>
                    <Route path="/" element={<div>Hello Felix</div>} />
                </Routes>
        </StyledApp>
    )
}

export default App
