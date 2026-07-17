// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {describe, expect, it} from "@jest/globals"
import {renderToStaticMarkup} from "react-dom/server"
import {manifestCustomCss} from "@/services/customCss"
import type {ResultsManifest} from "@/types/results"
import {CustomCssStylesheet} from "./CustomCssStylesheet"

const manifest: ResultsManifest = {
    schema_version: 1,
    tenant_id: "tenant-id",
    election_event_id: "event-id",
    election_ids: ["election-id"],
    route_scope: "event",
    publication_id: "publication-id",
    results_event_id: "results-event-id",
    version: 1,
    access: "public",
    visibility_scope: "full_event",
    custom_css: {
        election_event: ".seq-results-page { color: red; }",
        elections: {
            "election-id": ".seq-results-contest { color: blue; }",
        },
    },
    contests: [],
    artifacts: {},
}

describe("CustomCssStylesheet", () => {
    it("loads event and election CSS globally in deterministic order", () => {
        const css = manifestCustomCss(manifest, "election-id")
        const markup = renderToStaticMarkup(<CustomCssStylesheet css={css} />)

        expect(css).toBe(".seq-results-page { color: red; }\n.seq-results-contest { color: blue; }")
        expect(markup).toContain('class="seq-results-portal-custom-css"')
        expect(markup).toContain('data-seq-results-custom-css="active"')
        expect(markup).toContain(".seq-results-page { color: red; }")
        expect(markup).toContain(".seq-results-contest { color: blue; }")
    })
})
