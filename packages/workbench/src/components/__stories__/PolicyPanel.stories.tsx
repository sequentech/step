// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, within} from "storybook/test"
import {EOverVotePolicy} from "@sequentech/ui-core"
import {
    RANKED_IDS,
    ScenarioId,
    scenarioSnapshot,
    type PreviewContest,
} from "@sequentech/ui-test-kit/fixtures/scenarios"
import {IDS} from "@sequentech/ui-test-kit/fixtures"
import {BoundKey, boundsIssue, contestPolicies, type ContestOverrides} from "../../policies"
import {PolicyPanel, type PolicyPanelContest} from "../PolicyPanel"

const [council, budget] = scenarioSnapshot(ScenarioId.RANKED_MULTI_CONTEST).preview.ballot_styles[0]
    .contests

const row = (
    contest: PreviewContest,
    preferential: boolean,
    overrides: ContestOverrides = {}
): PolicyPanelContest => ({
    id: contest.id,
    name: String(contest.name),
    preferential,
    baseline: {
        ...contestPolicies(contest),
        [BoundKey.MIN]: contest.min_votes,
        [BoundKey.MAX]: contest.max_votes,
    },
    overrides,
    boundsIssue: boundsIssue(contest, overrides),
})

const meta = {
    title: "Workbench/Policy panel",
    component: PolicyPanel,
    args: {contests: [row(council, false)], onChange: fn(), onClear: fn()},
} satisfies Meta<typeof PolicyPanel>
export default meta
type Story = StoryObj<typeof meta>

const region = (canvasElement: HTMLElement, name: string) =>
    within(within(canvasElement).getByRole("region", {name}))

export const Plurality: Story = {
    play: async ({args, canvasElement}) => {
        const contest = region(canvasElement, "Council representative")
        await expect(contest.queryByRole("combobox", {name: /^Duplicated rank/})).toBeNull()
        await userEvent.click(contest.getByRole("combobox", {name: /^Over vote/}))
        await userEvent.click(
            await within(canvasElement.ownerDocument.body).findByRole("option", {
                name: EOverVotePolicy.NOT_ALLOWED_WITH_MSG_AND_DISABLE,
            })
        )
        await expect(args.onChange).toHaveBeenCalledWith(
            IDS.contest,
            "over_vote_policy",
            EOverVotePolicy.NOT_ALLOWED_WITH_MSG_AND_DISABLE
        )
        await expect(
            within(canvasElement).getByRole("button", {name: "Clear overrides"})
        ).toBeDisabled()
    },
}

export const Preferential: Story = {
    args: {contests: [row(council, false), row(budget, true)]},
    play: async ({args, canvasElement}) => {
        const contest = region(canvasElement, "Budget priorities")
        await expect(contest.getByRole("combobox", {name: /^Duplicated rank/})).toBeVisible()
        await expect(contest.getByRole("combobox", {name: /^Preference gaps/})).toBeVisible()
        const maximum = contest.getByRole("spinbutton", {name: "Maximum votes"})
        await expect(maximum).toHaveValue(3)
        await userEvent.tripleClick(maximum)
        await userEvent.keyboard("2")
        await expect(args.onChange).toHaveBeenLastCalledWith(RANKED_IDS.contest, BoundKey.MAX, 2)
    },
}

export const Overridden: Story = {
    args: {
        contests: [
            row(council, false, {
                over_vote_policy: EOverVotePolicy.ALLOWED_WITH_MSG,
                [BoundKey.MIN]: 2,
            }),
        ],
    },
    play: async ({args, canvasElement}) => {
        const contest = region(canvasElement, "Council representative")
        await expect(contest.getByRole("combobox", {name: /^Over vote/})).toHaveTextContent(
            EOverVotePolicy.ALLOWED_WITH_MSG
        )
        await expect(contest.getAllByText("Overridden")).toHaveLength(2)
        await expect(contest.getByRole("alert")).toHaveTextContent(
            "The minimum exceeds the maximum; the scenario bounds stay in use."
        )
        await userEvent.click(within(canvasElement).getByRole("button", {name: "Clear overrides"}))
        await expect(args.onClear).toHaveBeenCalledOnce()
    },
}

export const NoContests: Story = {
    args: {contests: []},
    play: async ({canvasElement}) => {
        await expect(
            within(canvasElement).getByText("This area has no contests to configure.")
        ).toBeVisible()
    },
}
