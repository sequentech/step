// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {renderToStaticMarkup} from "react-dom/server"
import CandidatesList from "./CandidatesList"

describe("CandidatesList", () => {
    it("shows the selected candidates label when the list is collapsed", () => {
        const markup = renderToStaticMarkup(
            <CandidatesList
                title="Category A"
                isCollapsible={true}
                defaultExpanded={false}
                selectedCandidatesLabel="2 candidates selected"
            >
                <li>Candidate A</li>
            </CandidatesList>
        )

        expect(markup).toContain("2 candidates selected")
    })

    it("hides the selected candidates label when the list is expanded", () => {
        const markup = renderToStaticMarkup(
            <CandidatesList
                title="Category A"
                isCollapsible={true}
                defaultExpanded={true}
                selectedCandidatesLabel="2 candidates selected"
            >
                <li>Candidate A</li>
            </CandidatesList>
        )

        expect(markup).not.toContain("2 candidates selected")
    })
})
