// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {TENANT_ID} from "@/__stories__/AdminStoryProvider"
import {ContestDataForm} from "./EditContestDataForm"
import {
    CONTEST_ID,
    ELECTION_ID,
    PolicyFormFixture,
    choose,
    contestPolicyChoices,
    openPolicies,
    policyOperations,
    save,
    setUpPolicyForm,
    type PolicyScenario,
} from "./__stories__/PolicyFormFixture"

type Scenario = Omit<PolicyScenario, "kind">
const meta = {
    title: "Admin/Contest/ContestDataForm",
    component: ContestDataForm,
    args: {canEdit: true, preferential: false},
    beforeEach: setUpPolicyForm,
    render: (args) => <PolicyFormFixture kind="contest" {...args} />,
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

export const PoliciesSaveWireValues: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await userEvent.click(await canvas.findByRole("button", {name: "Ballot Design"}))
        await waitFor(() => expect(canvas.getByDisplayValue("Council members")).not.toBeVisible())
        await choose(canvasElement, "Under Vote Policy", "Warn and Alert")
        await choose(canvasElement, "Invalid Vote Policy", "Not Allowed")
        await choose(canvasElement, "Blank Vote Policy", "Not Allowed")
        await userEvent.click(canvas.getByRole("button", {name: "Save"}))
        await waitFor(() => expect(save).toHaveBeenCalledTimes(1))
        expect(save.mock.calls[0][0]).toEqual(
            expect.objectContaining({
                id: CONTEST_ID,
                tenant_id: TENANT_ID,
                election_id: ELECTION_ID,
                presentation: expect.objectContaining({
                    under_vote_policy: "warn-and-alert",
                    invalid_vote_policy: "not-allowed",
                    blank_vote_policy: "not-allowed",
                }),
            })
        )
        expect(policyOperations()).toEqual([])
    },
}
export const LanguageTabs: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByDisplayValue("Council members")).toBeVisible()
        await userEvent.click(canvas.getByRole("tab", {name: "Spanish"}))
        await expect(await canvas.findByDisplayValue("Miembros del consejo")).toBeVisible()
        expect(save).not.toHaveBeenCalled()
    },
}
export const ReadOnlyHidesSave: Story = {
    args: {canEdit: false},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByDisplayValue("Council members")
        expect(canvas.queryByRole("button", {name: "Save"})).not.toBeInTheDocument()
        expect(save).not.toHaveBeenCalled()
    },
}
export const UnderVotePoliciesSaveEveryChoice = contestPolicyChoices(
    "Under Vote Policy",
    "under_vote_policy",
    [
        ["Warn", "warn"],
        ["Warn in Review", "warn-only-in-review"],
        ["Warn and Alert", "warn-and-alert"],
        ["Allowed", "allowed"],
    ]
)
export const InvalidVotePoliciesSaveEveryChoice = contestPolicyChoices(
    "Invalid Vote Policy",
    "invalid_vote_policy",
    [
        ["Not Allowed", "not-allowed"],
        ["Warn", "warn"],
        ["Warn Invalid Implicit And Explicit", "warn-invalid-implicit-and-explicit"],
        ["Allowed With Exclusive Explicit", "allowed-with-exclusive-explicit"],
        ["Allowed", "allowed"],
    ]
)
export const BlankVotePoliciesSaveEveryChoice = contestPolicyChoices(
    "Blank Vote Policy",
    "blank_vote_policy",
    [
        ["Not Allowed", "not-allowed"],
        ["Warn", "warn"],
        ["Warn in Review", "warn-only-in-review"],
        ["Allowed", "allowed"],
    ]
)
export const OverVotePoliciesSaveEveryChoice = contestPolicyChoices(
    "Over Vote Policy",
    "over_vote_policy",
    [
        ["Allowed with Warning Message", "allowed-with-msg"],
        ["Allowed with Warning message and Alert", "allowed-with-msg-and-alert"],
        ["Not Allowed with Warning message and Alert", "not-allowed-with-msg-and-alert"],
        [
            "Not Allowed with Warning message and Disable further selections",
            "not-allowed-with-msg-and-disable",
        ],
        ["Allowed", "allowed"],
    ]
)
export const PreferentialRankPoliciesSaveBothChoices: Story = {
    args: {preferential: true},
    play: async ({canvasElement}) => {
        const canvas = await openPolicies(canvasElement, "contest")
        for (const [option, wireValue] of [
            [
                "Show Warning and Dialog (voter not allowed to proceed)",
                "not-allowed-warn-and-dialog",
            ],
            ["Show Warning and Dialog (voter can proceed)", "allowed-warn-and-dialog"],
        ]) {
            save.mockClear()
            await choose(canvasElement, "Invalid Vote - Duplicate Rank Policy", option)
            await choose(canvasElement, "Invalid Vote - Skipped Ranks Policy", option)
            await userEvent.click(canvas.getByRole("button", {name: "Save"}))
            await waitFor(() => expect(save).toHaveBeenCalledTimes(1))
            expect(save.mock.calls[0][0]).toMatchObject({
                counting_algorithm: "instant-runoff",
                presentation: {
                    duplicated_rank_policy: wireValue,
                    preference_gaps_policy: wireValue,
                },
            })
        }
    },
}
export const CheckableListsSaveEveryChoice = contestPolicyChoices(
    /checkable lists/i,
    "enable_checkable_lists",
    [
        ["Lists Only", "allow-selecting-lists"],
        ["Candidates Only", "allow-selecting-candidates"],
        ["Disabled", "disabled"],
        ["Candidates And Lists", "allow-selecting-candidates-and-lists"],
    ]
)
export const CollapsibleListsSaveEveryChoice = contestPolicyChoices(
    "Collapsible Lists",
    "collapsible_lists",
    [
        ["Enabled (starts collapsed)", "enabled-collapsed"],
        ["Enabled (starts expanded)", "enabled-expanded"],
        ["Disabled", "disabled"],
    ]
)
export const CheckboxShapeSavesBothChoices = contestPolicyChoices(
    "Candidates checkbox icon shape",
    "candidates_icon_checkbox_policy",
    [
        ["Round Checkbox", "round-checkbox"],
        ["Square Checkbox", "square-checkbox"],
    ]
)
export const SelectionAndDisplaySettingsSave: Story = {
    play: async ({canvasElement}) => {
        const canvas = await openPolicies(canvasElement, "contest")
        expect(
            canvas.queryByRole("combobox", {name: "Invalid Vote - Duplicate Rank Policy"})
        ).not.toBeInTheDocument()
        await userEvent.click(canvas.getByRole("switch", {name: "Allow Write-Ins"}))
        await userEvent.click(canvas.getByRole("switch", {name: "Decided by acclamation"}))
        for (const [label, value] of [
            [/min votes/i, "1"],
            [/max votes/i, "6"],
            [/columns/i, "2"],
            [/winning candidates num/i, "2"],
            [/max selections per type/i, "1"],
        ] as const) {
            const input = canvas.getByRole("spinbutton", {name: label})
            await userEvent.clear(input)
            await userEvent.type(input, value)
        }
        await userEvent.type(canvas.getByRole("textbox", {name: "Page Name"}), "Council page")
        await userEvent.click(canvas.getByRole("button", {name: "Save"}))
        await waitFor(() => expect(save).toHaveBeenCalledTimes(1))
        expect(save.mock.calls[0][0]).toMatchObject({
            is_acclaimed: true,
            min_votes: 1,
            max_votes: 6,
            winning_candidates_num: 2,
            presentation: {
                allow_writeins: true,
                columns: 2,
                max_selections_per_type: 1,
                pagination_policy: "Council page",
            },
        })
    },
}
