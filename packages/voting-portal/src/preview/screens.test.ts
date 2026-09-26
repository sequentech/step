// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {ScenarioId} from "@sequentech/ui-test-kit/fixtures/scenarios"
import {
    isPreviewScreen,
    PREVIEW_SCREENS,
    PreviewScreen,
    previewDeepLink,
    previewScreenAt,
    previewScreenPath,
    previewStoryId,
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
