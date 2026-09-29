// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, within} from "storybook/test"
import {EScopeSelector} from "./types"
import {MonitoringSelectors} from "./MonitoringSelectors"
import {POSTS, getDashboardResponse} from "./__stories__/MonitoringFixture"

const {scope_options} = getDashboardResponse()

const meta = {
    title: "Admin/Monitoring/MonitoringSelectors",
    component: MonitoringSelectors,
    args: {
        selectors: [EScopeSelector.REGION, EScopeSelector.POST, EScopeSelector.COUNTRY],
        scope: {},
        onChange: fn(),
        options: scope_options,
        restricted: false,
    },
} satisfies Meta<typeof MonitoringSelectors>
export default meta
type Story = StoryObj<typeof meta>

const combobox = (canvasElement: HTMLElement, name: string) =>
    within(canvasElement).getByRole("combobox", {name})

export const AllScopes: Story = {
    play: async ({canvasElement}) => {
        await expect(combobox(canvasElement, "Region")).toHaveTextContent("All regions")
        await expect(combobox(canvasElement, "Post")).toHaveTextContent("All Posts")
        await expect(combobox(canvasElement, "Country")).toHaveTextContent("All countries")
    },
}

export const RestrictedViewer: Story = {
    args: {restricted: true, options: getDashboardResponse({restricted: true}).scope_options},
    play: async ({canvasElement}) => {
        await expect(combobox(canvasElement, "Post")).toHaveTextContent("All authorized Posts")
        await userEvent.click(combobox(canvasElement, "Post"))
        const options = within(await within(document.body).findByRole("listbox")).getAllByRole(
            "option"
        )
        expect(options.map((option) => option.textContent)).toEqual([
            "All authorized Posts",
            "Madrid",
        ])
    },
}

export const PinnedPost: Story = {
    args: {pinnedPost: POSTS.madrid},
    play: async ({canvasElement}) => {
        // The election's Post is fixed, and with it its Region: only Country is left.
        expect(within(canvasElement).queryByRole("combobox", {name: "Post"})).toBeNull()
        expect(within(canvasElement).queryByRole("combobox", {name: "Region"})).toBeNull()
        await expect(combobox(canvasElement, "Country")).toBeVisible()
    },
}

export const SettingsWords: Story = {
    args: {
        settings: {time_zone: "UTC", selectors: {region: {label: "Faculty", all: "All faculties"}}},
    },
    play: async ({canvasElement}) => {
        await expect(combobox(canvasElement, "Faculty")).toHaveTextContent("All faculties")
    },
}

export const RegionNarrowsPosts: Story = {
    args: {scope: {post: POSTS.paris}},
    play: async ({canvasElement, args}) => {
        await userEvent.click(combobox(canvasElement, "Region"))
        await userEvent.click(await within(document.body).findByRole("option", {name: "North"}))
        // Paris is in the south, so choosing the north drops it.
        await expect(args.onChange).toHaveBeenCalledWith({region: "north"})
    },
}
