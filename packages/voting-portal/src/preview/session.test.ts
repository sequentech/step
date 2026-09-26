// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {
    RANKED_IDS,
    ScenarioId,
    scenarioSnapshot,
    type JsonObject,
    type ScenarioSnapshot,
} from "@sequentech/ui-test-kit/fixtures/scenarios"
import {IDS} from "@sequentech/ui-test-kit/fixtures"
import {clearVoterSession, store} from "../store/store"
import {setElection} from "../store/elections/electionsSlice"
import {addCastVotes, CastVoteStatus} from "../store/castVotes/castVotesSlice"
import {BallotStyleConfigurationError} from "../services/BallotStyles"
import type {IBallotStyle} from "../store/ballotStyles/ballotStylesSlice"
import {PreviewScreen, previewTarget} from "./screens"
import {
    loadPreviewSnapshot,
    preparePreviewScreen,
    sampleSelection,
    type EncryptForReview,
} from "./session"

jest.mock("@sequentech/ui-essentials", () => ({Loader: () => null}), {virtual: true})
// sequent-core decides this; the tests only need to tell the two contest kinds apart.
jest.mock("../services/BallotService", () => ({
    provideBallotService: () => ({
        isPreferential: (algorithm?: string) => algorithm === "instant-runoff",
    }),
}))

const load = (snapshot: ScenarioSnapshot) => {
    loadPreviewSnapshot(snapshot, store.dispatch)
    return store.getState()
}

const loadedStyle = (snapshot: ScenarioSnapshot): IBallotStyle =>
    load(snapshot).ballotStyles[IDS.election]!

const marks = (style: IBallotStyle) =>
    sampleSelection(style).map(({contest_id, choices}) => [
        contest_id,
        choices.filter(({selected}) => selected > -1).map(({id, selected}) => [id, selected]),
    ])

const firstContest = (snapshot: ScenarioSnapshot) =>
    snapshot.preview.ballot_styles[0].contests[0] as JsonObject

beforeEach(() => store.dispatch(clearVoterSession()))

test("a snapshot replaces the voter session through the production preview loader", () => {
    store.dispatch(
        setElection({
            id: "previous-election",
            tenant_id: "previous",
            election_event_id: "previous",
            image_document_id: "",
            contests: [],
        })
    )
    store.dispatch(
        addCastVotes([
            {
                id: "cast",
                tenant_id: "previous",
                election_id: "previous-election",
                election_event_id: "previous",
                status: CastVoteStatus.VALID,
            },
        ])
    )
    const state = load(scenarioSnapshot(ScenarioId.SIMPLE_PLURALITY))
    expect(Object.keys(state.elections)).toEqual([IDS.election])
    expect(state.castVotes).toEqual({})
    expect(Object.keys(state.electionEvent)).toEqual([IDS.event])
    expect(state.ballotStyles[IDS.election]).toMatchObject({
        id: IDS.election,
        tenant_id: IDS.tenant,
        area_id: IDS.area,
        ballot_eml: {contests: [{id: IDS.contest}]},
    })
    expect(state.ballotSelections[IDS.election]).toEqual([
        {
            contest_id: IDS.contest,
            is_explicit_invalid: false,
            is_decline_to_vote: false,
            is_blank_ballot: false,
            invalid_errors: [],
            invalid_alerts: [],
            choices: [
                {id: IDS.alice, selected: -1},
                {id: IDS.bob, selected: -1},
            ],
        },
    ])
})

test("the target is the area's first election, and nothing loads for an area without one", () => {
    const snapshot = scenarioSnapshot(ScenarioId.SIMPLE_PLURALITY)
    expect(previewTarget(snapshot)).toEqual({
        tenantId: IDS.tenant,
        eventId: IDS.event,
        electionId: IDS.election,
    })
    snapshot.areaId = "another-area"
    expect(previewTarget(snapshot)).toEqual({
        tenantId: IDS.tenant,
        eventId: IDS.event,
        electionId: undefined,
    })
    const state = load(snapshot)
    expect(state.ballotStyles).toEqual({})
    expect(state.elections).toEqual({})
    expect(Object.keys(state.electionEvent)).toEqual([IDS.event])
})

test("the production loader still rejects ballot configurations it cannot show", () => {
    const snapshot = scenarioSnapshot(ScenarioId.SIMPLE_PLURALITY)
    for (const candidate of firstContest(snapshot).candidates as JsonObject[])
        candidate.presentation = {is_explicit_invalid: true}
    expect(() => loadPreviewSnapshot(snapshot, store.dispatch)).toThrow(
        BallotStyleConfigurationError
    )
})

test("the sample marks the first candidate of a plurality contest", () => {
    expect(marks(loadedStyle(scenarioSnapshot(ScenarioId.SIMPLE_PLURALITY)))).toEqual([
        [IDS.contest, [[IDS.alice, 0]]],
    ])
})

test("the sample ranks preferential contests and meets each minimum", () => {
    const snapshot = scenarioSnapshot(ScenarioId.RANKED_MULTI_CONTEST)
    snapshot.preview.ballot_styles[0].contests[1].min_votes = 2
    expect(marks(loadedStyle(snapshot))).toEqual([
        [IDS.contest, [[IDS.alice, 0]]],
        [
            RANKED_IDS.contest,
            [
                [RANKED_IDS.options[0], 0],
                [RANKED_IDS.options[1], 1],
            ],
        ],
    ])
})

test("the sample skips marker candidates, acclaimed contests and caps at the candidates", () => {
    const snapshot = scenarioSnapshot(ScenarioId.SIMPLE_PLURALITY)
    const contest = firstContest(snapshot)
    const [alice, bob] = contest.candidates as JsonObject[]
    contest.candidates = [
        {...alice, id: "invalid", presentation: {is_explicit_invalid: true}},
        {...alice, id: "blank", presentation: {is_explicit_blank: true}},
        {...alice, id: "write-in", presentation: {is_write_in: true}},
        bob,
        alice,
    ]
    Object.assign(contest, {min_votes: 3, max_votes: 3})
    expect(marks(loadedStyle(snapshot))).toEqual([
        [
            IDS.contest,
            [
                [IDS.bob, 0],
                [IDS.alice, 0],
            ],
        ],
    ])
    contest.is_acclaimed = true
    expect(marks(loadedStyle(snapshot))).toEqual([[IDS.contest, []]])
})

describe("preparing a screen", () => {
    const encrypt = jest.fn<ReturnType<EncryptForReview>, Parameters<EncryptForReview>>()
    beforeEach(() => encrypt.mockReset().mockReturnValue(true))

    const prepare = (snapshot: ScenarioSnapshot, screen: PreviewScreen) =>
        preparePreviewScreen({snapshot, screen}, load(snapshot), store.dispatch, encrypt)

    test("voting screens need no ballot", () => {
        for (const screen of [PreviewScreen.CHOOSER, PreviewScreen.START, PreviewScreen.VOTE])
            prepare(scenarioSnapshot(ScenarioId.SIMPLE_PLURALITY), screen)
        expect(encrypt).not.toHaveBeenCalled()
        expect(store.getState().extra.isVoted).toEqual({})
    })

    test("review and confirmation encrypt the sample ballot first", () => {
        for (const screen of [PreviewScreen.REVIEW, PreviewScreen.CONFIRMATION]) {
            encrypt.mockClear()
            prepare(scenarioSnapshot(ScenarioId.SIMPLE_PLURALITY), screen)
            const [[style, selection, multiContest]] = encrypt.mock.calls
            expect(style.election_id).toBe(IDS.election)
            expect(selection).toEqual(sampleSelection(style))
            expect(multiContest).toBe(false)
            expect(store.getState().extra.isVoted).toEqual({[IDS.election]: true})
        }
    })

    test("a multiple-contest ballot is encrypted as one", () => {
        const snapshot = scenarioSnapshot(ScenarioId.SIMPLE_PLURALITY)
        const style = snapshot.preview.ballot_styles[0]
        style.election_event_presentation = {
            ...(style.election_event_presentation as JsonObject),
            contest_encryption_policy: "multiple-contests",
        }
        prepare(snapshot, PreviewScreen.REVIEW)
        expect(encrypt.mock.calls[0][2]).toBe(true)
    })

    test("a failed encryption or a missing ballot stops the preparation", () => {
        encrypt.mockReturnValue(false)
        expect(() =>
            prepare(scenarioSnapshot(ScenarioId.SIMPLE_PLURALITY), PreviewScreen.REVIEW)
        ).toThrow("The sample ballot for the review screen could not be encrypted")
        expect(store.getState().extra.isVoted).toEqual({})
        const empty = scenarioSnapshot(ScenarioId.SIMPLE_PLURALITY)
        empty.areaId = "another-area"
        expect(() => prepare(empty, PreviewScreen.CONFIRMATION)).toThrow(
            "The snapshot has no ballot to prepare the confirmation screen"
        )
    })
})
