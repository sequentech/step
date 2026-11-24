// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {captureGoldReturn, type ReturnSecret} from "./lib/signing/operationalGoldReturn"
import {captureProductGoldReturn, PRODUCT_RETURN_PATH} from "./lib/signing/productCallbackCapture"

const productReturn = window.location.pathname === PRODUCT_RETURN_PATH
if (productReturn || window.location.pathname === "/operational-signing/return") {
    // Remove callback credentials before importing the portal, telemetry or an API client.
    const capture = productReturn ? captureProductGoldReturn : captureGoldReturn
    const captured = capture(window.location, (path) => window.history.replaceState(null, "", path))
    const lang = window.navigator.language.split("-")[0]
    const messages =
        lang === "es"
            ? [
                  "Consultando el estado de la aprobación…",
                  "No se pudo confirmar la aprobación. ",
                  "Volver al portal de administración",
              ]
            : lang === "tl" || lang === "fil"
              ? [
                    "Tinitingnan ang pag-apruba…",
                    "Hindi makumpirma ang pag-apruba. ",
                    "Bumalik sa Admin Portal",
                ]
              : [
                    "Checking your approval…",
                    "Could not confirm the approval. ",
                    "Return to the Admin Portal",
                ]
    const root = document.getElementById("root")
    if (root) {
        root.setAttribute("role", "status")
        root.textContent = messages[0]
    }
    const cannotResume = () => {
        if (!root) return
        root.textContent = messages[1]
        const link = document.createElement("a")
        link.href = "/"
        link.textContent = messages[2]
        root.append(link)
    }
    const finish: Promise<(browser: Window, secret: ReturnSecret | undefined) => Promise<boolean>> =
        productReturn
            ? import("./lib/signing/finishProductApproval").then(
                  (entry) => entry.finishProductApproval
              )
            : import("./lib/signing/finishOperationalApproval").then(
                  (entry) => entry.finishOperationalApproval
              )
    void finish
        .then(async (run) => {
            if (!(await run(window, captured))) cannotResume()
        })
        .catch(cannotResume)
} else {
    void import("./PortalEntrypoint")
}
