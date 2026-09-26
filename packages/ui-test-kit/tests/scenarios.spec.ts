// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {test, expect} from "@playwright/test"
import {
    isScenarioId,
    parseSnapshot,
    RANKED_IDS,
    SCENARIOS,
    ScenarioChannel,
    ScenarioId,
    scenarioSnapshot,
    serializeSnapshot,
    SnapshotError,
    validateSnapshot,
    type JsonObject,
} from "../fixtures/scenarios"
import {IDS} from "../fixtures"
import {loadCore} from "../wasm/node"

function issuesOf(input: unknown): string[] {
    try {
        validateSnapshot(input)
    } catch (error) {
        if (error instanceof SnapshotError) return error.issues
        throw error
    }
    throw new Error("The snapshot was accepted")
}

// Each rejection starts from this valid control and changes one property.
const control = () => JSON.parse(serializeSnapshot(scenarioSnapshot(ScenarioId.SIMPLE_PLURALITY)))

test("scenario IDs, titles and channels are the documented stable identifiers", () => {
    expect(SCENARIOS.map(({id, title, channel}) => [id, title, channel])).toEqual([
        ["simple-plurality", "Simple plurality", "online"],
        ["ranked-multi-contest", "Ranked multi-contest", "online"],
        ["kiosk-voter", "Kiosk voter", "kiosk"],
    ])
    // Storybook derives story IDs from titles; these titles must reduce to the IDs.
    for (const {id, title} of SCENARIOS)
        expect(title.toLowerCase().replace(/[^a-z0-9]+/g, "-")).toBe(id)
    expect(Object.values(ScenarioId).sort()).toEqual(SCENARIOS.map(({id}) => id).sort())
    expect(SCENARIOS.every(({id}) => isScenarioId(id))).toBe(true)
    for (const other of ["Simple-plurality", "simple plurality", "", undefined, 1])
        expect(isScenarioId(other)).toBe(false)
})

for (const {id} of SCENARIOS) {
    test(`${id} builds a valid, deterministic snapshot that survives a JSON round trip`, () => {
        const snapshot = scenarioSnapshot(id)
        expect(validateSnapshot(snapshot)).toBe(snapshot)
        expect(scenarioSnapshot(id)).toEqual(snapshot)
        expect(scenarioSnapshot(id)).not.toBe(snapshot)
        expect(parseSnapshot(serializeSnapshot(snapshot))).toEqual(snapshot)
        expect(snapshot).toMatchObject({
            version: 1,
            scenarioId: id,
            provenance: {origin: "bundled", createdAt: "2026-01-15T12:00:00.000Z", changes: []},
            tenantId: IDS.tenant,
            areaId: IDS.area,
        })
    })
}

test("scenario documents carry the literal ballots each scenario describes", () => {
    const contests = (id: ScenarioId) =>
        scenarioSnapshot(id).preview.ballot_styles[0].contests.map((contest) => ({
            name: contest.name,
            algorithm: contest.counting_algorithm,
            bounds: [contest.min_votes, contest.max_votes],
            candidates: contest.candidates.map((candidate) => candidate.name),
        }))
    expect(contests(ScenarioId.SIMPLE_PLURALITY)).toEqual([
        {
            name: "Council representative",
            algorithm: "plurality-at-large",
            bounds: [1, 1],
            candidates: ["Alice Example", "Bob Example"],
        },
    ])
    expect(contests(ScenarioId.RANKED_MULTI_CONTEST)).toEqual([
        {
            name: "Council representative",
            algorithm: "plurality-at-large",
            bounds: [1, 1],
            candidates: ["Alice Example", "Bob Example"],
        },
        {
            name: "Budget priorities",
            algorithm: "instant-runoff",
            bounds: [0, 3],
            candidates: ["Park renovation", "Library hours", "Cycle lanes", "Community garden"],
        },
    ])
    const kiosk = scenarioSnapshot(ScenarioId.KIOSK_VOTER)
    expect(kiosk.channel).toBe(ScenarioChannel.KIOSK)
    for (const record of [kiosk.preview.election_event, kiosk.preview.elections[0]])
        expect(record.status).toMatchObject({voting_status: "CLOSED", kiosk_voting_status: "OPEN"})
    // The builders copy the shared fixture instead of changing it.
    expect(
        scenarioSnapshot(ScenarioId.SIMPLE_PLURALITY).preview.election_event.status
    ).toMatchObject({voting_status: "OPEN"})
})

test("the actual sequent-core encrypts and decodes a choice in every scenario ballot", async () => {
    const core = await loadCore()
    for (const {id} of SCENARIOS) {
        const [style] = scenarioSnapshot(id).preview.ballot_styles
        const selection = style.contests.map((contest) => ({
            contest_id: contest.id,
            is_explicit_invalid: false,
            is_decline_to_vote: false,
            is_blank_ballot: false,
            invalid_errors: [],
            invalid_alerts: [],
            // A ranked contest records positions; the others record one mark.
            choices: contest.candidates.map((candidate, index) => ({
                id: candidate.id,
                selected: index < (contest.id === RANKED_IDS.contest ? 2 : 1) ? index : -1,
            })),
        }))
        const ballot = core.encrypt_decoded_contest_js(selection, style)
        expect(core.hash_auditable_ballot_js(ballot)).toMatch(/^[0-9a-f]{64}$/)
        const decoded = core.decode_auditable_ballot_js(ballot) as {
            contest_id: string
            choices: {id: string; selected: number}[]
        }[]
        const marks = decoded.map(({contest_id, choices}) => [
            contest_id,
            choices.filter(({selected}) => selected >= 0).map(({id, selected}) => [id, selected]),
        ])
        expect(marks, id).toEqual(
            id === ScenarioId.RANKED_MULTI_CONTEST
                ? [
                      [IDS.contest, [[IDS.alice, 0]]],
                      [
                          RANKED_IDS.contest,
                          [
                              [RANKED_IDS.options[0], 0],
                              [RANKED_IDS.options[1], 1],
                          ],
                      ],
                  ]
                : [[IDS.contest, [[IDS.alice, 0]]]]
        )
    }
})

test("parse rejects malformed JSON and unsupported versions before reading fields", () => {
    expect(() => parseSnapshot("{")).toThrow(/snapshot: not valid JSON/)
    expect(issuesOf([])).toEqual(["snapshot: expected an object, found []"])
    expect(issuesOf({...control(), version: 2, channel: "fax"})).toEqual([
        "version: expected 1, found 2",
    ])
    expect(issuesOf({...control(), version: undefined})).toEqual([
        "version: expected 1, found nothing",
    ])
})

test("validation names every invalid envelope field", () => {
    const cases: [string, (snapshot: JsonObject) => void, string][] = [
        [
            "unknown scenario",
            (snapshot) => (snapshot.scenarioId = "other"),
            'scenarioId: expected one of simple-plurality, ranked-multi-contest, kiosk-voter, found "other"',
        ],
        [
            "unknown origin",
            (snapshot) => ((snapshot.provenance as JsonObject).origin = "copied"),
            'provenance.origin: expected one of bundled, exported, found "copied"',
        ],
        [
            "unparseable time",
            (snapshot) => ((snapshot.provenance as JsonObject).createdAt = "yesterday"),
            'provenance.createdAt: expected an ISO 8601 time, found "yesterday"',
        ],
        [
            "non-text change",
            (snapshot) => ((snapshot.provenance as JsonObject).changes = [1]),
            "provenance.changes: expected strings",
        ],
        [
            "empty area",
            (snapshot) => (snapshot.areaId = ""),
            'snapshot.areaId: expected a non-empty string, found ""',
        ],
        [
            "unknown channel",
            (snapshot) => (snapshot.channel = "telephone"),
            'channel: expected one of online, kiosk, found "telephone"',
        ],
    ]
    for (const [name, change, issue] of cases) {
        const snapshot = control()
        change(snapshot)
        expect(issuesOf(snapshot), name).toEqual([issue])
    }
})

test("validation rejects preview documents the portal loader cannot show", () => {
    const cases: [string, (preview: JsonObject) => void, string][] = [
        [
            "missing event",
            (preview) => delete preview.election_event,
            "preview.election_event: expected an object, found nothing",
        ],
        [
            "unlisted election",
            (preview) => (preview.elections = []),
            `preview.ballot_styles[0].election_id: election ${IDS.election} is not in preview.elections`,
        ],
        [
            "other event",
            (preview) => ((preview.election_event as JsonObject).id = "another-event"),
            `preview.ballot_styles[0].election_event_id: expected the preview event "another-event", found "${IDS.event}"`,
        ],
        [
            "negative bound",
            (preview) => (firstContest(preview).max_votes = -1),
            "preview.ballot_styles[0].contests[0].max_votes: expected a non-negative integer, found -1",
        ],
        [
            "candidate without id",
            (preview) => delete (firstContest(preview).candidates as JsonObject[])[1].id,
            "preview.ballot_styles[0].contests[0].candidates[1].id: expected a non-empty string, found nothing",
        ],
        [
            "documents not a list",
            (preview) => (preview.documents = {}),
            "preview.documents: expected an array, found {}",
        ],
    ]
    for (const [name, change, issue] of cases) {
        const snapshot = control()
        change(snapshot.preview)
        expect(issuesOf(snapshot), name).toEqual([issue])
    }
})

test("styles of other areas are not tied to the published elections", () => {
    const snapshot = control()
    snapshot.areaId = "another-area"
    snapshot.preview.elections = []
    // An area without ballots is the empty chooser the portal shows for it.
    expect(validateSnapshot(snapshot).areaId).toBe("another-area")
})

test("all issues are reported together and in the error message", () => {
    const snapshot = control()
    snapshot.channel = "telephone"
    snapshot.tenantId = 7
    let error: unknown
    try {
        parseSnapshot(JSON.stringify(snapshot))
    } catch (caught) {
        error = caught
    }
    expect(error).toBeInstanceOf(SnapshotError)
    expect((error as SnapshotError).issues).toEqual([
        "snapshot.tenantId: expected a non-empty string, found 7",
        'channel: expected one of online, kiosk, found "telephone"',
    ])
    expect((error as SnapshotError).message).toBe(
        'Invalid workbench snapshot:\n- snapshot.tenantId: expected a non-empty string, found 7\n- channel: expected one of online, kiosk, found "telephone"'
    )
})

function firstContest(preview: JsonObject): JsonObject {
    return ((preview.ballot_styles as JsonObject[])[0].contests as JsonObject[])[0]
}
