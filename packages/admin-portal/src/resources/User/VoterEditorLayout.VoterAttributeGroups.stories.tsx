// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, within} from "storybook/test"
import {TextField} from "@mui/material"
import type {UserProfileAttribute, UserProfileAttributeGroup} from "@/gql/graphql"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {VoterAttributeGroups, groupVoterAttributes} from "./VoterEditorLayout"
import {PROFILE_ATTRIBUTES, PROFILE_GROUPS} from "./__stories__/UserProfileFixture"

interface Scenario {
    attributes: UserProfileAttribute[]
    groups: UserProfileAttributeGroup[]
}

const meta = {
    title: "Admin/User/VoterAttributeGroups",
    component: VoterAttributeGroups,
    args: {attributes: PROFILE_ATTRIBUTES, groups: PROFILE_GROUPS},
    render: ({attributes, groups}) => (
        <VoterAttributeGroups
            runs={groupVoterAttributes(attributes, groups)}
            getHeader={(run) => run.group?.display_header ?? ""}
            getDescription={(run) => run.group?.display_description ?? ""}
            renderField={(attribute) => (
                <TextField key={attribute.name} label={attribute.display_name} />
            )}
        />
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

export const GroupedAttributes: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const personal = canvas.getByRole("group", {name: "Personal data"})
        expect(personal).toHaveAccessibleDescription("Identity of the voter")
        expect(within(personal).getAllByRole("textbox")).toHaveLength(2)
        await expect(within(personal).getByLabelText("Date of birth")).toBeVisible()
        const address = canvas.getByRole("group", {name: "Address"})
        expect(address).not.toHaveAttribute("aria-describedby")
        expect(within(address).getByLabelText("City")).toBeVisible()
        // Username and email have no group and form the first, untitled run.
        expect(canvas.getAllByRole("group")).toHaveLength(3)
    },
}

export const UngroupedProfile: Story = {
    args: {
        attributes: PROFILE_ATTRIBUTES.map((attribute) => ({...attribute, group: null})),
        groups: [],
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const [run] = canvas.getAllByRole("group")
        expect(canvas.getAllByRole("group")).toHaveLength(1)
        expect(within(run).getAllByRole("textbox")).toHaveLength(PROFILE_ATTRIBUTES.length)
        expect(run.querySelector(".voter-attribute-group__legend")).toBeNull()
    },
}
