// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {AdminStoryProvider, TENANT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {trusteeRecords} from "@/__stories__/fixtures"
import SelectActedTrustee from "./SelectActedTrustee"

interface Scenario {
    trustees: "listed" | "none" | "failure"
    defaultValue?: string
    label?: string
    onSelectTrustee: (trustee: string) => void
}

let graphql: ReturnType<typeof graphqlBoundary>

const meta = {
    title: "Admin/User/SelectActedTrustee",
    component: SelectActedTrustee,
    args: {trustees: "listed", label: "Acted trustee", onSelectTrustee: fn()},
    argTypes: {trustees: {control: "inline-radio", options: ["listed", "none", "failure"]}},
    parameters: {
        expectedFailure: {
            reason: "The select name has a space, so its aria-labelledby points at ids that do not exist.",
            a11y: ["aria-input-field-name"],
        },
    },
    beforeEach: async ({args}) => {
        graphql = graphqlBoundary(
            {
                TrusteeNames: () => {
                    if (args.trustees === "failure")
                        throw new Error("Synthetic trustee service failure")
                    const listed = args.trustees === "listed" ? trusteeRecords : []
                    return {
                        data: {
                            sequent_backend_trustee: listed.map(({id, name}) => ({id, name})),
                        },
                    }
                },
            },
            {schema: true}
        )
        await graphql.ready
    },
    render: ({defaultValue, label, onSelectTrustee}) => (
        <AdminStoryProvider boundary={graphql}>
            <SelectActedTrustee
                tenantId={TENANT_ID}
                source="acted_trustee"
                defaultValue={defaultValue}
                label={label}
                onSelectTrustee={onSelectTrustee}
            />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const blankOption = {
    expectedFailure: {
        reason: "The blank first option has no accessible name.",
        a11y: ["aria-toggle-field-name"],
    },
}

const queried = [{name: "TrusteeNames", variables: {tenantId: TENANT_ID}, headers: {}}]

async function options(canvasElement: HTMLElement) {
    await waitFor(() => expect(graphql.calls).toEqual(queried))
    await userEvent.click(within(canvasElement).getByRole("combobox"))
    return within(await within(document.body).findByRole("listbox")).getAllByRole("option")
}

export const Populated: Story = {
    parameters: blankOption,
    play: async ({canvasElement}) => {
        const [label] = within(canvasElement).getAllByText("Acted trustee")
        await expect(label).toBeVisible()
        const listed = await options(canvasElement)
        expect(listed.map((option) => option.textContent?.trim())).toEqual([
            "",
            ...trusteeRecords.map(({name}) => name),
        ])
    },
}

export const SelectATrustee: Story = {
    play: async ({args, canvasElement}) => {
        const [, , second] = await options(canvasElement)
        await userEvent.click(second)
        await expect(within(canvasElement).getByRole("combobox")).toHaveTextContent("trustee2")
        expect(args.onSelectTrustee).toHaveBeenCalledWith("trustee2")
    },
}

export const PreselectedTrustee: Story = {
    args: {defaultValue: "trustee3"},
    play: async ({args, canvasElement}) => {
        await waitFor(() =>
            expect(within(canvasElement).getByRole("combobox")).toHaveTextContent("trustee3")
        )
        expect(args.onSelectTrustee).toHaveBeenLastCalledWith("trustee3")
        expect(graphql.calls).toEqual(queried)
    },
}

export const UnknownPreselectedTrustee: Story = {
    args: {defaultValue: "retired-trustee"},
    play: async ({args}) => {
        // A trustee that no longer exists is cleared instead of kept.
        await waitFor(() => expect(args.onSelectTrustee).toHaveBeenLastCalledWith(""))
        expect(graphql.calls).toEqual(queried)
    },
}

export const WithoutTrustees: Story = {
    args: {trustees: "none"},
    parameters: blankOption,
    play: async ({canvasElement}) => {
        const listed = await options(canvasElement)
        expect(listed.map((option) => option.textContent?.trim())).toEqual([""])
    },
}

export const TrusteeServiceFailure: Story = {
    args: {trustees: "failure"},
    parameters: blankOption,
    play: async ({args, canvasElement}) => {
        const listed = await options(canvasElement)
        expect(listed).toHaveLength(1)
        expect(args.onSelectTrustee).not.toHaveBeenCalled()
    },
}

export const WithoutLabel: Story = {
    args: {label: undefined},
    parameters: blankOption,
    play: async ({canvasElement}) => {
        expect(within(canvasElement).queryByText("Acted trustee")).toBeNull()
        await options(canvasElement)
    },
}
