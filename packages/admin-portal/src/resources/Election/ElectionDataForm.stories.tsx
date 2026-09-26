// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {TENANT_ID} from "@/__stories__/AdminStoryProvider"
import {ElectionDataForm} from "./ElectionDataForm"
import {
    ELECTION_ID,
    PolicyFormFixture,
    choose,
    openPolicies,
    save,
    setUpPolicyForm,
    type PolicyScenario,
} from "../Contest/__stories__/PolicyFormFixture"

type Scenario = Omit<PolicyScenario, "kind">
const meta = {
    title: "Admin/Election/ElectionDataForm",
    component: ElectionDataForm,
    args: {canEdit: true, preferential: false},
    beforeEach: setUpPolicyForm,
    render: (args) => <PolicyFormFixture kind="election" {...args} />,
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

export const AllowTallyPolicySave: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByDisplayValue("Council election")
        await userEvent.click(canvas.getByRole("button", {name: "Advanced Configuration"}))
        await waitFor(() => expect(canvas.getByDisplayValue("Council election")).not.toBeVisible())
        await choose(canvasElement, "Allow Tally", "Requires Voting Period End")
        await userEvent.click(canvas.getByRole("button", {name: "Save"}))
        await waitFor(() => expect(save).toHaveBeenCalledTimes(1))
        expect(save.mock.calls[0][0]).toEqual(
            expect.objectContaining({
                id: ELECTION_ID,
                status: {allow_tally: "requires-voting-period-end"},
            })
        )
    },
}

export const AuditPoliciesSaveEveryChoice: Story = {
    play: async ({canvasElement}) => {
        const canvas = await openPolicies(canvasElement, "election")
        for (const [option, wireValue] of [
            ["Not Show", "not-show"],
            ["Show In Help Dialog", "show-in-help"],
            ["Show", "show"],
        ]) {
            save.mockClear()
            await choose(canvasElement, "Audit Button Display Options", option)
            await userEvent.click(canvas.getByRole("button", {name: "Save"}))
            await waitFor(() => expect(save).toHaveBeenCalledTimes(1))
            expect(save.mock.calls[0][0]).toMatchObject({
                id: ELECTION_ID,
                tenant_id: TENANT_ID,
                presentation: {audit_button_cfg: wireValue},
            })
        }
    },
}
export const AdvancedPoliciesSaveAndRestore: Story = {
    play: async ({canvasElement}) => {
        const canvas = await openPolicies(canvasElement, "election")
        await userEvent.click(canvas.getByRole("button", {name: "Advanced Configuration"}))
        await userEvent.click(canvas.getByRole("switch", {name: "Cast Vote Confirmation Modal"}))
        const allowedVotes = canvas.getByRole("spinbutton", {name: "Number of allowed votes"})
        await userEvent.clear(allowedVotes)
        await userEvent.type(allowedVotes, "3")
        for (const [label, option] of [
            ["Gold level Authentication Policy", "Gold level Authentication"],
            ["Start Screen Title Policy", "Election event title"],
            ["Security Confirmation Checkbox Policy", "Mandatory"],
            ["Grace Period Policy", "Grace period without alert"],
            ["Voting Screen Back Button Policy", "Go to the election start screen"],
            ["Blank Ballots Policy", "Enabled"],
            ["Consolidated Report Policy", "Generate"],
            ["Initialize Report Policy", "Required"],
            ["Allow Tally", "Disallowed"],
        ])
            await choose(canvasElement, label, option)
        const grace = canvas.getByRole("spinbutton", {name: "Grace period in seconds"})
        await expect(grace).toBeEnabled()
        await userEvent.clear(grace)
        await userEvent.type(grace, "45")
        await userEvent.click(canvas.getByRole("button", {name: "Save"}))
        await waitFor(() => expect(save).toHaveBeenCalledTimes(1))
        expect(save.mock.calls[0][0]).toMatchObject({
            num_allowed_revotes: 3,
            status: {allow_tally: "disallowed"},
            presentation: {
                cast_vote_confirm: true,
                cast_vote_gold_level: "gold-level",
                start_screen_title_policy: "election-event",
                security_confirmation_policy: "mandatory",
                grace_period_policy: "grace-period-without-alert",
                grace_period_secs: 45,
                voting_screen_back_policy: "start-screen",
                blank_ballots_policy: "enabled",
                consolidated_report_policy: "generate",
                initialization_report_policy: "required",
            },
        })
        save.mockClear()
        for (const [label, option] of [
            ["Gold level Authentication Policy", "No Gold level Authentication"],
            ["Start Screen Title Policy", "Election title"],
            ["Security Confirmation Checkbox Policy", "None"],
            ["Grace Period Policy", "No grace period"],
            ["Voting Screen Back Button Policy", "Go to the election selection screen"],
            ["Blank Ballots Policy", "Disabled"],
            ["Consolidated Report Policy", "Do Not Generate"],
            ["Initialize Report Policy", "Not Required"],
            ["Allow Tally", "Allowed"],
        ])
            await choose(canvasElement, label, option)
        await expect(grace).toBeDisabled()
        await userEvent.click(canvas.getByRole("switch", {name: "Cast Vote Confirmation Modal"}))
        await userEvent.click(canvas.getByRole("button", {name: "Save"}))
        await waitFor(() => expect(save).toHaveBeenCalledTimes(1))
        expect(save.mock.calls[0][0]).toMatchObject({
            status: {allow_tally: "allowed"},
            presentation: {
                cast_vote_confirm: false,
                cast_vote_gold_level: "no-gold-level",
                start_screen_title_policy: "election",
                security_confirmation_policy: "none",
                grace_period_policy: "no-grace-period",
                voting_screen_back_policy: "election-selection-screen",
                blank_ballots_policy: "disabled",
                consolidated_report_policy: "do-not-generate",
                initialization_report_policy: "not-required",
            },
        })
    },
}
