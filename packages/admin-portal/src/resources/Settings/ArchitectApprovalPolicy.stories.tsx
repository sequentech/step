// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, within} from "storybook/test"
import {i18n} from "@sequentech/ui-core"
import {AdminStoryProvider, TENANT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {ArchitectApprovalPolicy} from "./ArchitectApprovalPolicy"
import {PACKAGE_PERMISSIONS} from "./configurationPackage"

interface Scenario {
    state: "ready" | "loading" | "failed"
}

let graphql: ReturnType<typeof graphqlBoundary>
const meta = {
    title: "Admin/Settings/ArchitectApprovalPolicy",
    component: ArchitectApprovalPolicy,
    args: {state: "ready"},
    beforeEach: async ({args}) => {
        graphql = graphqlBoundary({
            ArchitectApprovalPolicy: () => {
                if (args.state === "loading") return new Promise<never>(() => {})
                if (args.state === "failed") throw new Error("Policy read failed")
                return {
                    data: {
                        get_architect_approval_policy: {required_approvals: null, generation: "0"},
                    },
                }
            },
            SetArchitectApprovalPolicy: ({variables}) => ({
                data: {
                    set_architect_approval_policy: {
                        required_approvals: variables.count ?? null,
                        generation: String(Number(variables.generation) + 1),
                    },
                },
            }),
        })
        await graphql.ready
    },
    render: () => (
        <AdminStoryProvider boundary={graphql} roles={PACKAGE_PERMISSIONS}>
            <ArchitectApprovalPolicy tenantId={TENANT_ID} />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>

export default meta
type Story = StoryObj<Scenario>

export const Loading: Story = {
    args: {state: "loading"},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText(i18n.t("architectApprovalPolicy.loading"))).toBeVisible()
        await expect(canvas.queryByRole("textbox")).not.toBeInTheDocument()
    },
}

export const ReadFailure: Story = {
    args: {state: "failed"},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByRole("alert")).toHaveTextContent(
            i18n.t("architectApprovalPolicy.unavailable")
        )
        await expect(canvas.queryByRole("textbox")).not.toBeInTheDocument()
    },
}

export const DefaultAndSave: Story = {
    parameters: {widgets: ["PolicyForm"]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const input = await canvas.findByRole("textbox", {
            name: i18n.t("architectApprovalPolicy.field"),
        })
        const save = canvas.getByRole("button", {name: i18n.t("architectApprovalPolicy.save")})
        await expect(input).toHaveValue("")
        await expect(input).toHaveAttribute("placeholder", "2")
        await expect(save).toBeDisabled()
        await userEvent.type(input, "0")
        await expect(save).toBeDisabled()
        await userEvent.clear(input)
        await userEvent.type(input, "3")
        await userEvent.click(save)
        await expect(await canvas.findByRole("alert")).toHaveTextContent(
            i18n.t("architectApprovalPolicy.saved")
        )
        await userEvent.clear(input)
        await userEvent.click(save)
        await expect(await canvas.findByRole("alert")).toHaveTextContent(
            i18n.t("architectApprovalPolicy.saved")
        )
        const writes = graphql.calls.filter(({name}) => name === "SetArchitectApprovalPolicy")
        expect(writes.map(({variables}) => variables)).toEqual([
            {tenantId: TENANT_ID, count: 3, generation: "0"},
            {tenantId: TENANT_ID, count: null, generation: "1"},
        ])
        expect(
            graphql.calls.every(({headers}) => headers["x-hasura-role"] === "tenant-write")
        ).toBe(true)
        expect(graphql.unexpected).toEqual([])
    },
}
