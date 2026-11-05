// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import type {ISigningApi, ISigningPanelData} from "@/lib/signing/api"
import {RESUME_KEY} from "@/lib/signing/request"
import {SigningHandoverLauncher} from "./SigningHandoverLauncher"
import {
    MARIA,
    REQUEST_ID,
    fakeApi,
    makePanel,
    memoryStorage,
    signedInAs,
} from "./__stories__/fixtures"

interface Scenario {
    handoverFails: boolean
}

let boundary: ReturnType<typeof graphqlBoundary>
let data: ISigningPanelData
let api: ReturnType<typeof fakeApi>
let storage: Storage
let logout: ReturnType<typeof fn>

const meta = {
    title: "Admin/Signing/SigningHandoverLauncher",
    component: SigningHandoverLauncher,
    args: {handoverFails: false},
    beforeEach: async ({args}) => {
        boundary = graphqlBoundary({})
        data = await makePanel({signed: [MARIA]})
        api = fakeApi(data)
        if (args.handoverFails) api.handover.mockRejectedValue(new Error("Synthetic failure"))
        storage = memoryStorage()
        logout = fn()
    },
    render: () => (
        <AdminStoryProvider boundary={boundary} auth={signedInAs(MARIA, logout)}>
            <SigningHandoverLauncher data={data} api={api as ISigningApi} storage={storage} />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const confirmDialog = async (canvasElement: HTMLElement) => {
    await userEvent.click(within(canvasElement).getByRole("button", {name: "Next member signs in"}))
    const dialog = await within(document.body).findByRole("dialog", {name: "Next member signs in"})
    // Once its fade-in is over, so visibility checks mean something.
    await waitFor(() => expect(dialog).toBeVisible())
    return within(dialog)
}

export const HandsOver: Story = {
    play: async ({canvasElement}) => {
        const dialog = await confirmDialog(canvasElement)
        await expect(
            dialog.getByText(
                /^You will be signed out\. The next member signs in on this computer and returns to this request to sign\. The request stays open until ([01]\d|2[0-3]):[0-5]\d UTC\.$/
            )
        ).toBeVisible()
        await userEvent.click(dialog.getByRole("button", {name: "Sign out"}))

        await expect(api.handover).toHaveBeenCalledWith(REQUEST_ID)
        const note = JSON.parse(storage.getItem(RESUME_KEY) ?? "null")
        await expect(note).toMatchObject({
            requestId: REQUEST_ID,
            tenantId: data.request.tenant_id,
            eventId: data.request.election_event_id,
        })
        // The request expires much later, so the note lasts 30 minutes.
        const lasts = Date.parse(note.expiresAt) - Date.now()
        await expect(lasts).toBeGreaterThan(29 * 60_000)
        await expect(lasts).toBeLessThanOrEqual(30 * 60_000)
        // Back to this page after the next member signs in.
        await expect(logout).toHaveBeenCalledWith(window.location.href)
    },
}

export const StaysSignedIn: Story = {
    play: async ({canvasElement}) => {
        const dialog = await confirmDialog(canvasElement)
        await userEvent.click(dialog.getByRole("button", {name: "Stay signed in"}))
        await expect(api.handover).not.toHaveBeenCalled()
        await expect(logout).not.toHaveBeenCalled()
        await expect(storage.getItem(RESUME_KEY)).toBeNull()
    },
}

export const HandoverNotRecorded: Story = {
    args: {handoverFails: true},
    play: async ({canvasElement}) => {
        const dialog = await confirmDialog(canvasElement)
        await userEvent.click(dialog.getByRole("button", {name: "Sign out"}))
        await expect(
            await dialog.findByText("The handover could not be recorded. Try again.")
        ).toBeVisible()
        // Nobody is signed out without the handover in the log.
        await expect(logout).not.toHaveBeenCalled()
        await expect(storage.getItem(RESUME_KEY)).toBeNull()
    },
}
