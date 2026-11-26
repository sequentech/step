// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {captureGoldReturn} from "./lib/signing/operationalGoldReturn"

if (window.location.pathname === "/operational-signing/return") {
    // Remove callback credentials before importing the portal, telemetry or an API client.
    const captured = captureGoldReturn(window.location, (path) =>
        window.history.replaceState(null, "", path)
    )
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
    void import("./lib/signing/finishOperationalApproval")
        .then(async ({finishOperationalApproval}) => {
            if (!(await finishOperationalApproval(window, captured))) cannotResume()
        })
        .catch(cannotResume)
} else {
    void import("./PortalEntrypoint")
}
