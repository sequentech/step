// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {createRoot} from "react-dom/client"
import * as core from "../../src/index"

// Expose the actual package exports to Playwright. The WASM glue and binary are
// bundled/served from the package's pinned local sequent-core dependency.
declare global {
    interface Window {
        uiCore: typeof core
    }
}
window.uiCore = core

const container = document.getElementById("root")
if (!container) throw new Error("missing browser fixture root")
createRoot(container).render(
    <main>
        <h1>UI Core browser integration</h1>
        <section aria-label="Election instructions">
            {core.stringToHtml(
                '<p lang="fr" dir="ltr" onclick="window.injected=true">Conseil</p>' +
                    "<script>window.injected=true</script>" +
                    '<a href="javascript:window.injected=true">Unsafe link</a>'
            )}
        </section>
    </main>
)
