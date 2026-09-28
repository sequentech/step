// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import type {Meta} from "@storybook/react-vite"
import {expect, within} from "storybook/test"
import {ScenarioId, type JsonObject} from "@sequentech/ui-test-kit/fixtures/scenarios"
import {PreviewScreen} from "../screens"
import {scenarioMeta, screenStory} from "./scenarioStories"

/**
 * What `workbench/embed.html` shows a framing tool such as the Election Architect: the
 * production screen inside the portal chrome, with the event's own stylesheet. The
 * messages that choose the screen are covered by the embed's unit and browser tests.
 */
export default {title: "Embedded voter preview", ...scenarioMeta} satisfies Meta

const scenario = ScenarioId.SIMPLE_PLURALITY

export const Vote = screenStory(scenario, PreviewScreen.VOTE, {
    chrome: true,
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByRole("checkbox", {name: /Alice Example/})).toBeEnabled()
        await expect(canvas.getByRole("banner")).toBeVisible()
        await expect(canvas.getByRole("contentinfo")).toBeVisible()
    },
})

/** The event's stylesheet reaches the screens, as `App` applies it in production. */
export const EventStylesheet = screenStory(scenario, PreviewScreen.START, {
    chrome: true,
    snapshot: (snapshot) => {
        for (const style of snapshot.preview.ballot_styles)
            style.election_event_presentation = {
                ...(style.election_event_presentation as JsonObject),
                css: ".voting-portal h1 { color: rgb(170, 0, 85); }",
            }
        return snapshot
    },
    play: async ({canvasElement}) => {
        const heading = await within(canvasElement).findByRole("heading", {
            level: 1,
            name: "Community Council",
        })
        await expect(getComputedStyle(heading).color).toBe("rgb(170, 0, 85)")
    },
})
