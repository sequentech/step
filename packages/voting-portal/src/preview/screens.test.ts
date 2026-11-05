// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {ScenarioId, scenarioSnapshot} from "@sequentech/ui-test-kit/fixtures/scenarios"
import {
    isPreviewScreen,
    PREVIEW_SCREENS,
    PreviewScreen,
    previewDeepLink,
    previewScreenAt,
    previewScreenPath,
    previewSessionPath,
    previewStoryId,
    previewTarget,
} from "./screens"

const target = {tenantId: "tenant", eventId: "event", electionId: "election"}

test("every screen opens at its production route", () => {
    expect(PREVIEW_SCREENS.map((screen) => previewScreenPath(target, screen))).toEqual([
        "/tenant/tenant/event/event/election-chooser",
        "/tenant/tenant/event/event/election/election/start",
        "/tenant/tenant/event/event/election/election/vote",
        "/tenant/tenant/event/event/election/election/review",
        "/tenant/tenant/event/event/election/election/confirmation",
    ])
})

test("an area without an election only has the chooser", () => {
    const withoutElection = {tenantId: "tenant", eventId: "event"}
    expect(previewScreenPath(withoutElection, PreviewScreen.CHOOSER)).toBe(
        "/tenant/tenant/event/event/election-chooser"
    )
    for (const screen of PREVIEW_SCREENS.filter((screen) => screen !== PreviewScreen.CHOOSER))
        expect(previewScreenPath(withoutElection, screen)).toBeUndefined()
})

test("production pathnames map back to their screen", () => {
    for (const screen of PREVIEW_SCREENS)
        expect(previewScreenAt(previewScreenPath(target, screen)!)).toBe(screen)
    expect(previewScreenAt("/tenant/t/event/e/election/x/vote/")).toBe(PreviewScreen.VOTE)
    for (const other of [
        "/tenant/t/event/e",
        "/tenant/t/event/e/materials",
        "/tenant/t/event/e/election/x/audit",
        "/tenant/t/event/e/election/x/ballot-locator/id",
        "/scenario/simple-plurality/vote",
    ])
        expect(previewScreenAt(other)).toBeUndefined()
})

test("scenario screens have the documented story IDs and workbench links", () => {
    expect(previewStoryId(ScenarioId.KIOSK_VOTER, PreviewScreen.CHOOSER)).toBe(
        "scenarios-kiosk-voter--chooser"
    )
    expect(previewDeepLink(ScenarioId.RANKED_MULTI_CONTEST, PreviewScreen.REVIEW)).toBe(
        "/scenario/ranked-multi-contest/review"
    )
})

test("only screen values are screens", () => {
    expect(PREVIEW_SCREENS.every(isPreviewScreen)).toBe(true)
    for (const value of ["audit", "Vote", "", undefined, 1])
        expect(isPreviewScreen(value)).toBe(false)
})

test("a source opens its area's first election unless it names one of that area's", () => {
    const snapshot = scenarioSnapshot(ScenarioId.RANKED_MULTI_CONTEST)
    const [first] = snapshot.preview.ballot_styles
    const other = {...first, id: "other-style", election_id: "other-election"}
    const source = {...snapshot, preview: {...snapshot.preview, ballot_styles: [first, other]}}

    expect(previewTarget(source).electionId).toBe(first.election_id)
    expect(previewTarget({...source, electionId: "other-election"}).electionId).toBe(
        "other-election"
    )
    // An election the area has no ballot for falls back to the area's first.
    expect(previewTarget({...source, electionId: "elsewhere"}).electionId).toBe(first.election_id)
})

test("a session opens its screen, or the chooser when its area has no election", () => {
    const snapshot = scenarioSnapshot(ScenarioId.SIMPLE_PLURALITY)
    const event = `/tenant/${snapshot.tenantId}/event/${snapshot.preview.election_event.id}`
    const election = snapshot.preview.ballot_styles[0].election_id
    expect(previewSessionPath(snapshot, PreviewScreen.VOTE)).toBe(
        `${event}/election/${election}/vote`
    )
    expect(previewSessionPath({...snapshot, areaId: "other"}, PreviewScreen.VOTE)).toBe(
        `${event}/election-chooser`
    )
})
