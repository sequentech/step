// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import type {Meta, StoryContext, StoryObj} from "@storybook/react-vite"
import {expect, within} from "storybook/test"
import {initCore} from "@sequentech/ui-core"
import {
    scenarioSnapshot,
    type ScenarioId,
    type ScenarioSnapshot,
} from "@sequentech/ui-test-kit/fixtures/scenarios"
import type {VoterPreviewParameters} from "../../../.storybook/withVoterPreview"
import {electionRoutes, tenantEventRoutes} from "../../appRoutes"
import {ErrorPage} from "../../routes/ErrorPage"
import {
    EVENT_ROUTE,
    PreviewScreen,
    previewScreenPath,
    previewStoryId,
    previewTarget,
    SCREEN_ROUTES,
} from "../screens"

type Play = (context: StoryContext) => Promise<void>

interface ScreenOptions {
    /** The screen prepared in the store; by default the rendered one. */
    prepared?: PreviewScreen
    snapshot?: VoterPreviewParameters["snapshot"]
    play?: Play
}

// Screens share the production store, so a docs page rendering them together would mix them.
export const scenarioMeta = {tags: ["!autodocs"]} satisfies Partial<Meta>

/** The production route of a screen, split for the story router's parent and leaf routes. */
function productionRoute(screen: PreviewScreen) {
    const route = SCREEN_ROUTES[screen]
    const separator = route.lastIndexOf("/")
    const leaf = route.slice(separator + 1)
    const routes = screen === PreviewScreen.CHOOSER ? tenantEventRoutes : electionRoutes
    const production = routes.find(({path}) => path === leaf)
    if (!production) throw new Error(`The portal has no ${leaf} route`)
    return {
        parentPath: separator < 0 ? EVENT_ROUTE : `${EVENT_ROUTE}/${route.slice(0, separator)}`,
        path: leaf,
        action: production.action,
        element: production.element,
    }
}

/** A production screen rendered in the story router, with the scenario loaded for it. */
export function screenStory(
    scenario: ScenarioId,
    screen: PreviewScreen,
    {prepared = screen, snapshot, play}: ScreenOptions = {}
): StoryObj {
    const {element, ...route} = productionRoute(screen)
    const base = scenarioSnapshot(scenario)
    const path = previewScreenPath(previewTarget(snapshot?.(base) ?? base), screen)
    if (!path) throw new Error(`The ${scenario} snapshot has no election for ${screen}`)
    return {
        parameters: {
            voterPreview: {scenario, screen: prepared, snapshot} satisfies VoterPreviewParameters,
            router: {
                ...route,
                initialEntries: [path],
                errorElement: <ErrorPage />,
            },
        },
        loaders: [() => initCore()],
        render: () => <main className="preview-screen-story">{element}</main>,
        play,
    }
}

/** A screen story whose ID is the documented equivalent of the workbench link. */
export function scenarioScreen(scenario: ScenarioId, screen: PreviewScreen, play: Play) {
    return screenStory(scenario, screen, {
        play: async (context) => {
            await expect(context.id).toBe(previewStoryId(scenario, screen))
            await play(context)
        },
    })
}

export const currentLocation = (canvasElement: HTMLElement) =>
    within(canvasElement.ownerDocument.body).getByRole("status", {name: "Current location"})

/** Two explicit invalid candidates in one contest, which the preview loader refuses. */
export const withTwoInvalidCandidates = (snapshot: ScenarioSnapshot): ScenarioSnapshot => {
    const [contest] = snapshot.preview.ballot_styles[0].contests
    contest.candidates = contest.candidates.map((candidate) => ({
        ...candidate,
        presentation: {is_explicit_invalid: true},
    }))
    return snapshot
}
