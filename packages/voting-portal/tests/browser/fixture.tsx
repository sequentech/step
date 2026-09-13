// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {createRoot} from "react-dom/client"
import {Provider} from "react-redux"
import {MemoryRouter} from "react-router-dom"
import {ThemeProvider} from "@mui/material/styles"
import i18next from "i18next"
import {I18nextProvider} from "react-i18next"
import {ECollapsibleLists, initCore, type BallotSelection} from "@sequentech/ui-core"
import theme from "../../../ui-essentials/src/services/theme"
import {Question} from "../../src/components/Question/Question"
import {ELECTION_WITH_INVALID} from "../../src/fixtures/election"
import {store} from "../../src/store/store"
import {resetBallotSelection} from "../../src/store/ballotSelections/ballotSelectionsSlice"
import type {IBallotStyle} from "../../src/store/ballotStyles/ballotStylesSlice"

declare global {
    interface Window {
        votingTest: {selection: () => BallotSelection | undefined}
    }
}

async function start() {
    await initCore()
    await i18next.init({
        lng: "en",
        fallbackLng: "en",
        interpolation: {escapeValue: false},
        resources: {
            en: {
                translation: {
                    candidatesList: {
                        expandAll: "Expand all",
                        collapseAll: "Collapse all",
                        collapseToggle: "Toggle {{listTitle}}",
                        showCandidates: "Show candidates",
                        hideCandidates: "Hide candidates",
                    },
                },
            },
        },
    })
    const ballot = structuredClone(ELECTION_WITH_INVALID)
    ballot.contests = [ballot.contests[0]]
    const question = ballot.contests[0]
    question.name = "Council"
    question.min_votes = 0
    question.max_votes = 1
    question.presentation = {
        collapsible_lists: ECollapsibleLists.ENABLED_COLLAPSED,
        types_presentation: {},
    }
    question.candidates = ["__proto__", "constructor", "toString", "Regular category"].map(
        (category, index) => ({
            ...question.candidates[0],
            id: `candidate-${index}`,
            name: `Candidate ${index}`,
            candidate_type: category,
            presentation: {},
        })
    )
    const style: IBallotStyle = {
        id: ballot.id,
        election_id: ballot.election_id,
        election_event_id: ballot.election_event_id,
        tenant_id: ballot.tenant_id,
        ballot_eml: ballot,
        created_at: "2026-01-01T00:00:00Z",
        last_updated_at: "2026-01-01T00:00:00Z",
    }
    store.dispatch(resetBallotSelection({ballotStyle: style, force: true}))
    window.votingTest = {selection: () => store.getState().ballotSelections[style.election_id]}
    const root = document.getElementById("root")
    if (!root) throw new Error("missing fixture root")
    createRoot(root).render(
        <Provider store={store}>
            <I18nextProvider i18n={i18next}>
                <MemoryRouter>
                    <ThemeProvider theme={theme}>
                        <main>
                            <h1>Voting Portal browser integration</h1>
                            <Question
                                ballotStyle={style}
                                question={question}
                                isReview={false}
                                setDecodedContests={() => {}}
                                errorSelectionState={[]}
                            />
                        </main>
                    </ThemeProvider>
                </MemoryRouter>
            </I18nextProvider>
        </Provider>
    )
}

// Surface bootstrap failures to the browser runner instead of leaving an empty
// fixture that only times out with no explanation.
void start().catch((error: unknown) => {
    document.body.textContent = `Fixture failed: ${String(error)}`
    throw error
})
