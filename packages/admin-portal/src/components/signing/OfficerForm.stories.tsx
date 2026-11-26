// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, within} from "storybook/test"
import {AdminStoryProvider, EVENT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import type {LocalOfficerApi, LocalOfficerRef} from "@/lib/signing/localInitializeOfficerApi"
import {OfficerForm} from "./LocalInitializeEnrollment"
import {ANA, ELECTION_ID, MARIA, REQUEST_ID, signedInAs} from "./__stories__/fixtures"

interface Scenario {
    gold: boolean
    onAttached: () => void
}

const nomination: LocalOfficerRef = {
    request_id: REQUEST_ID,
    payload_sha256: "ab".repeat(32),
    official_id: ANA.userId,
    election_id: ELECTION_ID,
}
let api: LocalOfficerApi

const meta = {
    title: "Admin/Signing/OfficerForm",
    component: OfficerForm,
    args: {gold: true, onAttached: fn()},
    beforeEach: () => {
        api = {
            nomineeStatus: fn(async () => "unavailable" as const),
            nomineeResume: fn(async () => undefined),
            presence: fn(async () => undefined),
            pending: fn(async () => [nomination, {...nomination, official_id: MARIA.userId}]),
            review: fn(async (ref) => ({
                ...ref,
                enrollment: "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa",
                csr_pem: "Synthetic public CSR, not a real device request",
                csr_sha256: "bc".repeat(32),
                spki_sha256: "cd".repeat(32),
            })),
            attach: fn(async () => {
                throw new Error("Unexpected certificate attachment in a read-only review")
            }),
        }
    },
    render: ({gold, onAttached}) => (
        <AdminStoryProvider
            boundary={graphqlBoundary({})}
            roles={["signing-certificates-register"]}
            auth={{...signedInAs(MARIA), isGoldUser: () => gold}}
        >
            <OfficerForm
                api={api}
                event={EVENT_ID}
                officer={MARIA.userId}
                onAttached={onAttached}
            />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

export const ReviewAnotherNomineesPublicRequest: Story = {
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        expect(api.pending).not.toHaveBeenCalled()
        await userEvent.click(canvas.getByRole("button", {name: "Check pending requests"}))
        const review = await canvas.findByRole("button", {
            name: `Review Post ${ELECTION_ID}, nominee ${ANA.userId}`,
        })
        expect(api.pending).toHaveBeenCalledWith(EVENT_ID)
        expect(canvas.queryByRole("button", {name: new RegExp(MARIA.userId)})).toBeNull()
        expect(api.review).not.toHaveBeenCalled()

        await userEvent.click(review)
        await expect(
            await canvas.findByRole("button", {name: "Download public certificate request"})
        ).toBeVisible()
        const selector = canvas.getByRole("button", {name: "Select issued public certificate"})
        expect(selector.tagName).toBe("BUTTON")
        expect(api.review).toHaveBeenCalledWith(nomination)

        const input = canvasElement.querySelector<HTMLInputElement>('input[type="file"]')
        expect(input).not.toBeNull()
        await userEvent.upload(input!, new File(["not a public certificate"], "invalid.pem"))
        await expect(await canvas.findByRole("alert")).toHaveTextContent(
            "Select exactly one public X.509 certificate"
        )
        await expect(
            canvas.getByRole("button", {name: "Register and attach certificate"})
        ).toBeDisabled()
        expect(api.attach).not.toHaveBeenCalled()
        expect(args.onAttached).not.toHaveBeenCalled()
    },
}

export const PendingRequestsRequireGold: Story = {
    args: {gold: false},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await userEvent.click(canvas.getByRole("button", {name: "Check pending requests"}))
        await expect(await canvas.findByRole("alert")).toHaveTextContent("Verify with Gold")
        expect(api.pending).not.toHaveBeenCalled()
        expect(api.review).not.toHaveBeenCalled()
        expect(api.attach).not.toHaveBeenCalled()
    },
}
