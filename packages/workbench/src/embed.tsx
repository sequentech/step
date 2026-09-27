// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import "voting-portal/src/index.css"
import React, {StrictMode} from "react"
import {createRoot} from "react-dom/client"
import {WasmContextProvider} from "@sequentech/ui-core"
import {initializePreviewLanguages} from "voting-portal/src/preview/context"
import {EmbeddedPreview} from "voting-portal/src/preview/EmbeddedPreview"
import {installNetworkGuard} from "./networkGuard"

/**
 * `embed.html`: the voter preview another tool frames and sends documents to, such as the
 * Election Architect. The same screens and providers as the workbench, without its panels.
 */
installNetworkGuard(window, ({method, url}) =>
    console.warn(`The voter preview blocked ${method} ${url}`)
)
initializePreviewLanguages("en")

createRoot(document.getElementById("root") as HTMLElement).render(
    <StrictMode>
        <WasmContextProvider>
            <EmbeddedPreview />
        </WasmContextProvider>
    </StrictMode>
)
