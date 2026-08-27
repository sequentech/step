// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {renderToStaticMarkup} from "react-dom/server"
import CandidatesList from "./CandidatesList"

// CandidatesList calls `useTranslation()` for the hidden "select the whole list"
// label only; these tests assert on structure, not on translated copy. Without a
// configured i18next instance the hook logs a NO_I18NEXT_INSTANCE warning, so
// stub it to echo the key.
jest.mock("react-i18next", () => ({
    useTranslation: () => ({t: (key: string): string => key}),
}))

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

    it("keeps the selected-candidates live region mounted while expanded", () => {
        // A live region inserted at the same moment as its text is not reliably
        // announced, so the region has to already be there while it is empty.
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

        expect(markup).toContain('role="status"')
    })

    it("marks the children container as a list, which list-style: none removes", () => {
        const markup = renderToStaticMarkup(
            <CandidatesList title="Category A">
                <li>Candidate A</li>
            </CandidatesList>
        )

        expect(markup).toContain('role="list"')
    })
})
