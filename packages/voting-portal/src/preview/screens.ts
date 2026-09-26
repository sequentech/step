// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {matchPath} from "react-router-dom"
import type {ScenarioId} from "@sequentech/ui-test-kit/fixtures/scenarios"

/** Voter screens a preview opens directly; each value is also a Storybook story name. */
export enum PreviewScreen {
    CHOOSER = "chooser",
    START = "start",
    VOTE = "vote",
    REVIEW = "review",
    CONFIRMATION = "confirmation",
}

export const PREVIEW_SCREENS: readonly PreviewScreen[] = Object.values(PreviewScreen)

/** Production route of every screen below `/tenant/:tenantId/event/:eventId`. */
export const SCREEN_ROUTES: Record<PreviewScreen, string> = {
    [PreviewScreen.CHOOSER]: "election-chooser",
    [PreviewScreen.START]: "election/:electionId/start",
    [PreviewScreen.VOTE]: "election/:electionId/vote",
    [PreviewScreen.REVIEW]: "election/:electionId/review",
    [PreviewScreen.CONFIRMATION]: "election/:electionId/confirmation",
}

export const EVENT_ROUTE = "/tenant/:tenantId/event/:eventId"

export interface PreviewTarget {
    tenantId: string
    eventId: string
    /** The area's first election; an area without ballots only has the chooser. */
    electionId?: string
}

export const isPreviewScreen = (value: unknown): value is PreviewScreen =>
    PREVIEW_SCREENS.includes(value as PreviewScreen)

/** The production URL of a screen, or nothing when it needs an election the area lacks. */
export function previewScreenPath(target: PreviewTarget, screen: PreviewScreen) {
    const route = SCREEN_ROUTES[screen]
    if (route.includes(":electionId") && !target.electionId) return undefined
    return `/tenant/${target.tenantId}/event/${target.eventId}/${route.replace(
        ":electionId",
        target.electionId ?? ""
    )}`
}

/** The screen a production pathname shows, if it is one of the preview screens. */
export const previewScreenAt = (pathname: string) =>
    PREVIEW_SCREENS.find((screen) => matchPath(`${EVENT_ROUTE}/${SCREEN_ROUTES[screen]}`, pathname))

/** Storybook story of a scenario screen: title `Scenarios/<title>`, story named after the screen. */
export const previewStoryId = (scenarioId: ScenarioId, screen: PreviewScreen) =>
    `scenarios-${scenarioId}--${screen}`

/** Workbench hash route that loads a scenario and opens a screen. */
export const previewDeepLink = (scenarioId: ScenarioId, screen: PreviewScreen) =>
    `/scenario/${scenarioId}/${screen}`
