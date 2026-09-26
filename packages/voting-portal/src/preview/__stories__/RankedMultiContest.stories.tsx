// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import type {Meta} from "@storybook/react-vite"
import {expect, within} from "storybook/test"
import {ScenarioId} from "@sequentech/ui-test-kit/fixtures/scenarios"
import {PreviewScreen} from "../screens"
import {scenarioMeta, scenarioScreen} from "./scenarioStories"

const scenario = ScenarioId.RANKED_MULTI_CONTEST

export default {title: "Scenarios/Ranked multi-contest", ...scenarioMeta} satisfies Meta

export const Chooser = scenarioScreen(scenario, PreviewScreen.CHOOSER, async ({canvasElement}) => {
    const canvas = within(canvasElement)
    await expect(await canvas.findByRole("button", {name: /click to vote/i})).toBeEnabled()
})

export const Start = scenarioScreen(scenario, PreviewScreen.START, async ({canvasElement}) => {
    const canvas = within(canvasElement)
    await expect(
        await canvas.findByText(
            "Choose your council representative and rank the budget priorities."
        )
    ).toBeVisible()
})

export const Vote = scenarioScreen(scenario, PreviewScreen.VOTE, async ({canvasElement}) => {
    const canvas = within(canvasElement)
    const headings = await canvas.findAllByRole("heading", {level: 2})
    await expect(headings.map(({textContent}) => textContent)).toEqual(
        expect.arrayContaining(["Council representative", "Budget priorities"])
    )
    await expect(canvas.getByText("Community garden")).toBeVisible()
})

export const Review = scenarioScreen(scenario, PreviewScreen.REVIEW, async ({canvasElement}) => {
    const canvas = within(canvasElement)
    await expect(await canvas.findByRole("button", {name: "Cast ballot"})).toBeEnabled()
    // The sample ballot ranks the first option of the preferential contest.
    await expect(canvas.getByText("Alice Example", {exact: true})).toBeVisible()
    await expect(canvas.getByText("Park renovation", {exact: true})).toBeVisible()
})

export const Confirmation = scenarioScreen(
    scenario,
    PreviewScreen.CONFIRMATION,
    async ({canvasElement}) => {
        await expect(
            await within(canvasElement).findByRole("heading", {
                level: 1,
                name: "Your vote has been cast",
            })
        ).toBeVisible()
    }
)
