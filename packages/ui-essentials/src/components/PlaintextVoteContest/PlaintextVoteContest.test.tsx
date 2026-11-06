// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {renderToStaticMarkup} from "react-dom/server"
import {uniq} from "lodash"
import {ThemeProvider} from "@mui/material/styles"
import type {
    ICandidate,
    IContest,
    IDecodedVoteChoice,
    IDecodedVoteContest,
} from "@sequentech/ui-core"
import theme from "../../services/theme"
import {PlaintextVoteContest, type PlaintextVoteContestProps} from "./PlaintextVoteContest"

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
        getImageUrl: (candidate: ICandidate) =>
            candidate.presentation?.urls?.find((url) => url.is_image)?.url,
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
} as unknown as IDecodedVoteContest

const renderContest = (
    question: IContest,
    plaintext: IDecodedVoteContest = questionPlaintext,
    props: Partial<PlaintextVoteContestProps> = {}
) =>
    renderToStaticMarkup(
        <ThemeProvider theme={theme}>
            <PlaintextVoteContest
                question={question}
                questionPlaintext={plaintext}
                publicBucketUrl=""
                contestNotFoundLabel="Contest not found"
                markedInvalidLabel="Marked invalid"
                pointsLabel={(points) => `${points} points`}
                isDeclineToVotePolicyEnabled={false}
                acclamationDescription="Default acclamation description"
                defaultLanguageCode="en"
                {...props}
            />
        </ThemeProvider>
    )

const BUCKET_URL = "https://bucket.example/"
const IMAGE_CANDIDATE_IDS = ["candidate-a", "candidate-b", "candidate-c"]
const ALL_CANDIDATE_IMAGES = IMAGE_CANDIDATE_IDS.map((id) => `${BUCKET_URL}${id}.png`)

const contestWithImages = {
    id: "contest",
    name: "Contest with images",
    candidates: IMAGE_CANDIDATE_IDS.map((id) => ({
        id,
        name: `${id} name`,
        presentation: {urls: [{url: `${id}.png`, kind: "image", is_image: true}]},
    })),
} as unknown as IContest

const withChoices = (
    selectedIds: string[],
    overrides: Partial<IDecodedVoteContest> = {}
): IDecodedVoteContest =>
    ({
        ...questionPlaintext,
        choices: IMAGE_CANDIDATE_IDS.map((id) => ({
            id,
            selected: selectedIds.indexOf(id),
        })),
        ...overrides,
    }) as IDecodedVoteContest

const requestedImages = (markup: string): string[] =>
    uniq(Array.from(markup.matchAll(/<img[^>]*\ssrc="([^"]*)"/g), (match) => match[1]))

const renderImages = (
    plaintext: IDecodedVoteContest,
    props: Partial<PlaintextVoteContestProps> = {}
) =>
    requestedImages(
        renderContest(contestWithImages, plaintext, {publicBucketUrl: BUCKET_URL, ...props})
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

    it("requests every candidate image of the contest in configuration order", () => {
        const markup = renderContest(contestWithImages, withChoices(["candidate-b"]), {
            publicBucketUrl: BUCKET_URL,
        })

        expect(markup).toContain("candidate-b name")
        expect(markup).not.toContain("candidate-a name")
        expect(markup).not.toContain("candidate-c name")
        expect(requestedImages(markup)).toEqual(ALL_CANDIDATE_IMAGES)
    })

    it("requests the same candidate images whatever the decoded choices", () => {
        const requestedPerBallot = [
            renderImages(withChoices([])),
            renderImages(withChoices(["candidate-a"])),
            renderImages(withChoices(["candidate-c"])),
            renderImages(withChoices(["candidate-c", "candidate-a"])),
            renderImages(withChoices([], {is_explicit_invalid: true})),
            renderImages(withChoices([], {is_decline_to_vote: true}), {
                isDeclineToVotePolicyEnabled: true,
            }),
            renderImages(withChoices([], {is_blank_ballot: true}), {
                isBlankBallotsPolicyEnabled: true,
            }),
        ]

        requestedPerBallot.forEach((requested) => {
            expect(requested).toEqual(ALL_CANDIDATE_IMAGES)
        })
    })
})
