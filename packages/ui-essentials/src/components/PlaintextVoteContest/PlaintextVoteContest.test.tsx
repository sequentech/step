// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {renderToStaticMarkup} from "react-dom/server"
import {ThemeProvider} from "@mui/material/styles"
import type {IContest, IDecodedVoteChoice} from "@sequentech/ui-core"
import theme from "../../services/theme"
import {PlaintextVoteContest} from "./PlaintextVoteContest"

jest.mock("react-i18next", () => ({
    useTranslation: () => ({
        t: (key: string) => key,
        i18n: {language: "en"},
    }),
}))

jest.mock(
    "@sequentech/ui-core",
    () => ({
        EInvalidVotePolicy: {NOT_ALLOWED: "NOT_ALLOWED"},
        translate: (value: Record<string, unknown>, key: string) => value[key],
        isPreferential: () => false,
        getLayoutProperties: () => ({ordered: false}),
        checkIsBlank: () => false,
        checkIsInvalidVote: () => false,
        checkIsWriteIn: () => false,
        getImageUrl: () => undefined,
        sortCandidatesInContest: <T,>(candidates: T[]) => candidates,
        categorizeCandidates: (contest: IContest) => ({
            invalidOrBlankCandidates: [],
            noCategoryCandidates: contest.candidates,
            categoriesMap: {},
        }),
        sortCategoryEntries: () => [],
        showCategoryOnReview: () => false,
        isChoiceSelected: (choices: Record<string, IDecodedVoteChoice>, candidateId: string) =>
            (choices[candidateId]?.selected ?? -1) > -1,
        isCategoryListSelected: () => false,
        shouldShowCategoryCandidateOnReview: () => false,
        isAcclaimedContest: (contest?: IContest | null) => Boolean(contest?.is_acclaimed),
        isEligibleAcclaimedCandidate: (candidate: any) =>
            !candidate.presentation?.is_explicit_blank &&
            !candidate.presentation?.is_explicit_invalid &&
            !candidate.presentation?.is_disabled &&
            !candidate.presentation?.is_write_in,
        translateFromPresentation: (contest: IContest, key: string, language: string) =>
            contest.presentation?.i18n?.[language]?.[key],
        stringToHtml: (value: string) => value,
    }),
    {virtual: true}
)

const questionPlaintext = {
    contest_id: "contest",
    is_explicit_invalid: false,
    is_decline_to_vote: false,
    is_blank_ballot: false,
    invalid_errors: [],
    invalid_alerts: [],
    choices: [],
}

const renderContest = (question: IContest) =>
    renderToStaticMarkup(
        <ThemeProvider theme={theme}>
            <PlaintextVoteContest
                question={question}
                questionPlaintext={questionPlaintext}
                publicBucketUrl=""
                contestNotFoundLabel="Contest not found"
                markedInvalidLabel="Marked invalid"
                pointsLabel={(points) => `${points} points`}
                isDeclineToVotePolicyEnabled={false}
                acclamationDescription="Default acclamation description"
                defaultLanguageCode="en"
            />
        </ThemeProvider>
    )

describe("PlaintextVoteContest", () => {
    it("shows every candidate and the configured description for an acclaimed contest", () => {
        const markup = renderContest({
            id: "contest",
            name: "Acclaimed contest",
            is_acclaimed: true,
            candidates: [
                {id: "candidate-a", name: "Candidate A"},
                {id: "candidate-b", name: "Candidate B"},
            ],
            presentation: {
                i18n: {
                    en: {acclamation_description: "Custom acclamation description"},
                },
            },
        } as unknown as IContest)

        expect(markup).toContain("Acclaimed contest")
        expect(markup).toContain("Custom acclamation description")
        expect(markup).toContain('role="alert"')
        expect(markup).toContain("Candidate A")
        expect(markup).toContain("Candidate B")
    })

    it("uses canonical acclaimed eligibility when displaying candidates", () => {
        const markup = renderContest({
            id: "contest",
            name: "Acclaimed contest",
            is_acclaimed: true,
            candidates: [
                {id: "eligible", name: "Eligible candidate"},
                {
                    id: "blank",
                    name: "Explicit blank marker",
                    presentation: {is_explicit_blank: true},
                },
                {
                    id: "invalid",
                    name: "Explicit invalid marker",
                    presentation: {is_explicit_invalid: true},
                },
                {
                    id: "disabled",
                    name: "Disabled candidate",
                    presentation: {is_disabled: true},
                },
                {
                    id: "write-in",
                    name: "Write-in slot",
                    presentation: {is_write_in: true},
                },
            ],
        } as unknown as IContest)

        expect(markup).toContain("Eligible candidate")
        expect(markup).not.toContain("Explicit blank marker")
        expect(markup).not.toContain("Explicit invalid marker")
        expect(markup).not.toContain("Disabled candidate")
        expect(markup).not.toContain("Write-in slot")
    })

    it("continues to hide unselected candidates for a normal contest", () => {
        const markup = renderContest({
            id: "contest",
            name: "Normal contest",
            candidates: [{id: "candidate-a", name: "Candidate A"}],
        } as unknown as IContest)

        expect(markup).toContain("Normal contest")
        expect(markup).not.toContain("Candidate A")
        expect(markup).not.toContain("Default acclamation description")
    })
})
