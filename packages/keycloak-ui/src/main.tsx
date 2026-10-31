// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {StrictMode, lazy, Suspense} from "react"
import {createRoot} from "react-dom/client"
import {KcPage, type KcContext} from "./kc.gen"
import {useInitialize} from "keycloakify/login/Template.useInitialize"

const Preview = import.meta.env.DEV ? lazy(() => import("./preview")) : undefined

// Keycloak's session scripts belong to the live document, not synthetic stories.
function SessionPage({kcContext}: {kcContext: KcContext}) {
    const {isReadyToRender} = useInitialize({kcContext, doUseDefaultCss: false})
    return isReadyToRender ? <KcPage kcContext={kcContext} /> : null
}

const root = document.getElementById("root")
if (root === null) {
    throw new Error("index.html has no #root element")
}

createRoot(root).render(
    <StrictMode>
        {window.kcContext === undefined ? (
            Preview ? (
                <Suspense>
                    <Preview />
                </Suspense>
            ) : (
                <h1>No Keycloak context</h1>
            )
        ) : (
            <SessionPage kcContext={window.kcContext} />
        )}
    </StrictMode>
)
