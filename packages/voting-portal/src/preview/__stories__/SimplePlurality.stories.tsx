// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import type {Meta} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {ScenarioId, type JsonObject} from "@sequentech/ui-test-kit/fixtures/scenarios"
import {IDS} from "@sequentech/ui-test-kit/fixtures"
import {store} from "../../store/store"
import {PreviewScreen} from "../screens"
import {
    currentLocation,
    scenarioMeta,
    scenarioScreen,
    screenStory,
    withTwoInvalidCandidates,
} from "./scenarioStories"

const scenario = ScenarioId.SIMPLE_PLURALITY
const electionPath = `/tenant/${IDS.tenant}/event/${IDS.event}/election/${IDS.election}`

export default {title: "Scenarios/Simple plurality", ...scenarioMeta} satisfies Meta

export const Chooser = scenarioScreen(scenario, PreviewScreen.CHOOSER, async ({canvasElement}) => {
    const canvas = within(canvasElement)
    await expect(await canvas.findByRole("button", {name: /click to vote/i})).toBeEnabled()
})

export const Start = scenarioScreen(scenario, PreviewScreen.START, async ({canvasElement}) => {
    const canvas = within(canvasElement)
    await expect(
        await canvas.findByRole("heading", {level: 1, name: "Community Council"})
    ).toBeVisible()
    await expect(canvas.getByRole("button", {name: "Start Voting"})).toBeEnabled()
})

export const Vote = scenarioScreen(scenario, PreviewScreen.VOTE, async ({canvasElement}) => {
    const canvas = within(canvasElement)
    await userEvent.click(await canvas.findByRole("checkbox", {name: /Alice Example/}))
    await userEvent.click(canvas.getByRole("button", {name: "Next"}))
    // The production route action redirects to review after the real encryption.
    await waitFor(() =>
        expect(currentLocation(canvasElement)).toHaveTextContent(`${electionPath}/review`)
    )
    await expect(store.getState().auditableBallots[IDS.election]).toBeDefined()
})

export const Review = scenarioScreen(scenario, PreviewScreen.REVIEW, async ({canvasElement}) => {
    const canvas = within(canvasElement)
    await expect(
        await canvas.findByRole("heading", {level: 1, name: "Review your ballot"})
    ).toBeVisible()
    await expect(canvas.getByText("Alice Example", {exact: true})).toBeVisible()
    await userEvent.click(canvas.getByRole("button", {name: "Cast ballot"}))
    await waitFor(() =>
        expect(currentLocation(canvasElement)).toHaveTextContent(`${electionPath}/confirmation`)
    )
})

export const Confirmation = scenarioScreen(
    scenario,
    PreviewScreen.CONFIRMATION,
    async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            await canvas.findByRole("heading", {level: 1, name: "Your vote has been cast"})
        ).toBeVisible()
        await expect(canvas.getByRole("button", {name: "Finish"})).toBeEnabled()
    }
)

/** Review without an encrypted ballot keeps its production loading state. */
export const ReviewLoading = screenStory(scenario, PreviewScreen.REVIEW, {
    prepared: PreviewScreen.VOTE,
    play: async ({canvasElement}) => {
        await expect(
            await within(canvasElement).findByRole("progressbar", {name: "Loading"})
        ).toBeVisible()
    },
})

/** An area without ballots: the production chooser has nothing to list. */
export const EmptyArea = screenStory(scenario, PreviewScreen.CHOOSER, {
    snapshot: (snapshot) => ({...snapshot, areaId: "40000000-0000-4000-8000-0000000000ff"}),
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByRole("heading", {name: "Ballot list"})).toBeVisible()
        await expect(canvas.queryByRole("button", {name: /click to vote/i})).toBeNull()
    },
})

/** The production preview loader rejects this ballot configuration. */
export const InvalidSnapshot = screenStory(scenario, PreviewScreen.VOTE, {
    snapshot: withTwoInvalidCandidates,
    play: async ({canvasElement}) => {
        await expect(await within(canvasElement).findByRole("alert")).toHaveTextContent(
            "Invalid ballot configuration: the contest defines 2 explicitly invalid candidates, but only one is allowed."
        )
    },
})

/** sequent-core rejects the encryption policy and the production error page shows it. */
export const EncryptionFailure = screenStory(scenario, PreviewScreen.VOTE, {
    snapshot: (snapshot) => {
        const [style] = snapshot.preview.ballot_styles
        style.election_event_presentation = {
            ...(style.election_event_presentation as JsonObject),
            contest_encryption_policy: "unsupported-encryption-policy",
        }
        return snapshot
    },
    play: async ({canvasElement}) => {
        await expect(
            await within(canvasElement).findByText("UNABLE_TO_ENCRYPT_BALLOT", {exact: true})
        ).toBeVisible()
    },
})
