// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {AdminStoryProvider, graphqlBoundary, TENANT_ID} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {IPermissions} from "@/types/keycloak"
import {TenantResetCard, type TenantResetPreview} from "./TenantResetCard"

enum EResetStory {
    READY = "READY",
    BALLOTS = "BALLOTS",
    UNFINISHED = "UNFINISHED",
}

interface Scenario {
    preview: EResetStory
}

const preview = (over: Partial<TenantResetPreview> = {}): TenantResetPreview => ({
    enabled: true,
    has_baseline: true,
    baseline_captured_at: "2026-11-28 09:00:00+00",
    resetting: false,
    election_events: [{id: "33333333-3333-4333-8333-333333333333", name: "General election"}],
    rows: {election_event: 1, election: 2, area: 4},
    cast_ballots: 0,
    sealed_ballots: 0,
    refusals: [],
    erase_ballots_phrase: "ERASE BALLOTS",
    slug: "acme",
    fresh_login_required: false,
    ...over,
})

const PREVIEWS: Record<EResetStory, TenantResetPreview> = {
    [EResetStory.READY]: preview(),
    [EResetStory.BALLOTS]: preview({
        cast_ballots: 12,
        refusals: [{kind: "ballots", cast: 12, sealed: 0, overridable: true}],
    }),
    [EResetStory.UNFINISHED]: preview({resetting: true}),
}

const meta = {
    title: "Admin/Settings/TenantResetCard",
    component: TenantResetCard,
    args: {preview: EResetStory.READY},
    argTypes: {preview: {control: "inline-radio", options: Object.values(EResetStory)}},
    render: (args) => (
        <AdminStoryProvider
            boundary={graphqlBoundary({
                TenantResetPreview: () => ({
                    data: {tenant_reset_preview: PREVIEWS[args.preview]},
                }),
            })}
            roles={[IPermissions.TENANT_RESET]}
        >
            <TenantResetCard tenantId={TENANT_ID} />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const openDialog = async (canvasElement: HTMLElement, button: string) => {
    const open = within(canvasElement).getByRole("button", {name: button})
    // It waits for the preview.
    await waitFor(() => expect(open).toBeEnabled())
    await userEvent.click(open)
    return within(await within(document.body).findByRole("dialog"))
}

export const Ready: Story = {
    parameters: {widgets: ["TenantResetCardFor"]},
    play: async ({canvasElement}) => {
        const dialog = await openDialog(canvasElement, "Reset tenant…")
        await expect(await dialog.findByText(/This deletes 1 election event/)).toBeVisible()
        const confirm = dialog.getByRole("button", {name: "Reset tenant"})
        await expect(confirm).toBeDisabled()
        await userEvent.type(dialog.getByRole("textbox", {name: "Type acme to confirm"}), "acme")
        await expect(confirm).toBeEnabled()
    },
}

export const BallotsToErase: Story = {
    parameters: {widgets: ["TenantResetCardFor"]},
    args: {preview: EResetStory.BALLOTS},
    play: async ({canvasElement}) => {
        const dialog = await openDialog(canvasElement, "Reset tenant…")
        await userEvent.type(dialog.getByRole("textbox", {name: "Type acme to confirm"}), "acme")
        const confirm = dialog.getByRole("button", {name: "Reset tenant"})
        await expect(confirm).toBeDisabled()
        await userEvent.type(
            dialog.getByRole("textbox", {name: /To erase them too, type ERASE BALLOTS/}),
            "ERASE BALLOTS"
        )
        await expect(confirm).toBeEnabled()
    },
}

export const Unfinished: Story = {
    parameters: {widgets: ["TenantResetCardFor"]},
    args: {preview: EResetStory.UNFINISHED},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByTestId("tenant-reset-unfinished")).toBeVisible()
        await expect(canvas.getByRole("button", {name: "Finish the reset…"})).toBeVisible()
    },
}
