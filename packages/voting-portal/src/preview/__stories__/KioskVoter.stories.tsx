// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, within} from "storybook/test"
import {ScenarioChannel, ScenarioId} from "@sequentech/ui-test-kit/fixtures/scenarios"
import {PreviewScreen} from "../screens"
import {scenarioMeta, scenarioScreen, screenStory} from "./scenarioStories"

const scenario = ScenarioId.KIOSK_VOTER

export default {title: "Scenarios/Kiosk voter", ...scenarioMeta} satisfies Meta

/** Online voting is closed, yet the kiosk voter can vote. */
export const Chooser = scenarioScreen(scenario, PreviewScreen.CHOOSER, async ({canvasElement}) => {
    const canvas = within(canvasElement)
    await expect(await canvas.findByRole("button", {name: /click to vote/i})).toBeEnabled()
})

export const Start = scenarioScreen(scenario, PreviewScreen.START, async ({canvasElement}) => {
    await expect(
        await within(canvasElement).findByRole("button", {name: "Start Voting"})
    ).toBeEnabled()
})

export const Vote = scenarioScreen(scenario, PreviewScreen.VOTE, async ({canvasElement}) => {
    await expect(
        await within(canvasElement).findByRole("checkbox", {name: /Bob Example/})
    ).toBeEnabled()
})

export const Review = scenarioScreen(scenario, PreviewScreen.REVIEW, async ({canvasElement}) => {
    await expect(
        await within(canvasElement).findByRole("button", {name: "Cast ballot"})
    ).toBeEnabled()
})

export const Confirmation = scenarioScreen(
    scenario,
    PreviewScreen.CONFIRMATION,
    async ({canvasElement}) => {
        await expect(
            await within(canvasElement).findByRole("button", {name: "Finish"})
        ).toBeEnabled()
    }
)

/** The same event reached online: its closed online channel offers no vote. */
export const ChooserOnlineChannel: StoryObj = {
    ...screenStory(scenario, PreviewScreen.CHOOSER, {
        play: async ({canvasElement}) => {
            const canvas = within(canvasElement)
            await expect(await canvas.findByRole("heading", {name: "Ballot list"})).toBeVisible()
            await expect(canvas.queryByRole("button", {name: /click to vote/i})).toBeNull()
        },
    }),
    globals: {voterChannel: ScenarioChannel.ONLINE},
}
