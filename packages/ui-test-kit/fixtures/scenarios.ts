// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {electionFixture, FIXED_TIME, IDS} from "./index"
import {
    ScenarioChannel,
    ScenarioId,
    SNAPSHOT_VERSION,
    SnapshotOrigin,
    type PreviewContest,
    type PreviewDocument,
    type ScenarioSnapshot,
} from "./snapshot"

export * from "./snapshot"

export interface ScenarioDefinition {
    id: ScenarioId
    /** Storybook title segment; it must sanitize to `id` so story IDs stay derivable. */
    title: string
    description: string
    channel: ScenarioChannel
}

export const SCENARIOS: readonly ScenarioDefinition[] = [
    {
        id: ScenarioId.SIMPLE_PLURALITY,
        title: "Simple plurality",
        description: "One plurality contest choosing one of two candidates, voting online.",
        channel: ScenarioChannel.ONLINE,
    },
    {
        id: ScenarioId.RANKED_MULTI_CONTEST,
        title: "Ranked multi-contest",
        description: "A plurality contest followed by an instant-runoff ranking of four options.",
        channel: ScenarioChannel.ONLINE,
    },
    {
        id: ScenarioId.KIOSK_VOTER,
        title: "Kiosk voter",
        description: "Online voting is closed and kiosk voting is open; the voter uses a kiosk.",
        channel: ScenarioChannel.KIOSK,
    },
]

export const RANKED_IDS = {
    contest: "60000000-0000-4000-8000-000000000002",
    options: [
        "71000000-0000-4000-8000-000000000001",
        "71000000-0000-4000-8000-000000000002",
        "71000000-0000-4000-8000-000000000003",
        "71000000-0000-4000-8000-000000000004",
    ],
} as const

type Fixture = ReturnType<typeof electionFixture>

function previewDocument(
    {ballot, event, election}: Fixture,
    {contests = ballot.contests}: {contests?: PreviewContest[]} = {}
): PreviewDocument {
    return {
        ballot_styles: [{...ballot, contests}],
        elections: [election],
        election_event: event,
        support_materials: [],
        documents: [],
    }
}

function rankedMultiContest(): PreviewDocument {
    const fixture = electionFixture()
    const [council] = fixture.ballot.contests
    const scope = {
        tenant_id: IDS.tenant,
        election_event_id: IDS.event,
        election_id: IDS.election,
    }
    const names = ["Park renovation", "Library hours", "Cycle lanes", "Community garden"]
    const budget: PreviewContest = {
        ...council,
        id: RANKED_IDS.contest,
        name: "Budget priorities",
        description: "Rank up to three projects in order of preference.",
        max_votes: 3,
        min_votes: 0,
        winning_candidates_num: 1,
        voting_type: "preferential",
        counting_algorithm: "instant-runoff",
        presentation: {
            candidates_order: "custom",
            invalid_vote_policy: "not-allowed",
            sort_order: 1,
        },
        candidates: RANKED_IDS.options.map((id, index) => ({
            ...scope,
            contest_id: RANKED_IDS.contest,
            id,
            name: names[index],
            presentation: {sort_order: index},
        })),
    }
    fixture.election.presentation.i18n.en.description =
        "Choose your council representative and rank the budget priorities."
    return previewDocument(fixture, {
        contests: [{...council, presentation: {...council.presentation, sort_order: 0}}, budget],
    })
}

function kioskVoter(): PreviewDocument {
    const fixture = electionFixture()
    const kioskOnly = {voting_status: "CLOSED", kiosk_voting_status: "OPEN"}
    Object.assign(fixture.event.status, kioskOnly)
    Object.assign(fixture.election.status, kioskOnly)
    fixture.election.voting_channels.online = false
    return previewDocument(fixture)
}

const builders: Record<ScenarioId, () => PreviewDocument> = {
    [ScenarioId.SIMPLE_PLURALITY]: () => previewDocument(electionFixture()),
    [ScenarioId.RANKED_MULTI_CONTEST]: rankedMultiContest,
    [ScenarioId.KIOSK_VOTER]: kioskVoter,
}

export function scenarioDefinition(id: ScenarioId): ScenarioDefinition {
    const definition = SCENARIOS.find((scenario) => scenario.id === id)
    if (!definition) throw new Error(`Unknown scenario ${id}`)
    return definition
}

/** A fresh, deterministic snapshot of a bundled scenario. */
export function scenarioSnapshot(id: ScenarioId): ScenarioSnapshot {
    return {
        version: SNAPSHOT_VERSION,
        scenarioId: id,
        provenance: {origin: SnapshotOrigin.BUNDLED, createdAt: FIXED_TIME, changes: []},
        tenantId: IDS.tenant,
        areaId: IDS.area,
        channel: scenarioDefinition(id).channel,
        preview: builders[id](),
    }
}
