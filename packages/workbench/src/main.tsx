// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import "voting-portal/src/index.css"
import React, {StrictMode} from "react"
import {createRoot} from "react-dom/client"
import {createHashRouter} from "react-router"
import {RouterProvider} from "react-router/dom"
import {ThemeProvider} from "@mui/material"
import {WasmContextProvider} from "@sequentech/ui-core"
import {initializePreviewLanguages} from "voting-portal/src/preview/context"
import {PreviewScreen, previewScreenAt} from "voting-portal/src/preview/screens"
import {BLOCKED_REQUEST_EVENT, installNetworkGuard} from "./networkGuard"
import {routes} from "./routes"
import {initialState, WorkbenchProvider} from "./state"
import {workbenchTheme} from "./theme"

const WORKBENCH_LANGUAGE = "en"

installNetworkGuard(window, ({method, url}) =>
    window.dispatchEvent(new CustomEvent(BLOCKED_REQUEST_EVENT, {detail: `${method} ${url}`}))
)
initializePreviewLanguages(WORKBENCH_LANGUAGE)

const router = createHashRouter(routes)
const initial = initialState(
    window.localStorage,
    previewScreenAt(router.state.location.pathname) ?? PreviewScreen.START
)

createRoot(document.getElementById("root") as HTMLElement).render(
    <StrictMode>
        <ThemeProvider theme={workbenchTheme}>
            <WasmContextProvider>
                <WorkbenchProvider initial={initial}>
                    <RouterProvider router={router} />
                </WorkbenchProvider>
            </WasmContextProvider>
        </ThemeProvider>
    </StrictMode>
)
