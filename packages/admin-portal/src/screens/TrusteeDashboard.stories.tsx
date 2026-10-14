// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, within, userEvent, waitFor, spyOn} from "storybook/test"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {storyFetch} from "@/__stories__/storyNetwork"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {TrusteeDashboard} from "./TrusteeDashboard"

type Scenario = Record<string, never>
const meta = {
    title: "Admin/Screens/TrusteeDashboard",
    component: TrusteeDashboard,
    render: () => (
        <AdminStoryProvider boundary={graphqlBoundary({})}>
            <TrusteeDashboard />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>
export const NativeTrustee: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByRole("heading", {name: "Trustee dashboard"})).toBeVisible()
        await expect(canvas.getByText(/trustee generate --trustee-config/)).toBeVisible()
        await expect(canvas.getByText(/trustee attest --request/)).toBeVisible()
        expect(canvas.queryByRole("button", {name: "Generate proof file manually"})).toBeNull()
        expect(canvas.queryByRole("textbox", {name: "Tally session ID"})).toBeNull()
    },
}

export const BrowserIdentity: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await userEvent.click(canvas.getByRole("button", {name: "Generate browser keys"}))
        const signing = await canvas.findByRole("textbox", {name: "Signing public key"})
        await expect(signing).not.toHaveValue("")
        await expect(
            canvas.getByRole("textbox", {name: "Share encryption public key"})
        ).not.toHaveValue("")
        await expect(canvas.getByRole("button", {name: "Download private keys"})).toBeEnabled()
        await userEvent.click(canvas.getByRole("button", {name: "Lock keys"}))
        await expect(
            canvas.queryByRole("textbox", {name: "Signing public key"})
        ).not.toBeInTheDocument()
        await expect(canvas.getByRole("button", {name: "Download private keys"})).toBeDisabled()
    },
}

const CUSTODY_PLATFORM = "https://custody-fixture.invalid/graphql"
let queuedRequest: Record<string, unknown> | undefined
let submittedProof: string | undefined
export const AutomaticTallyCustody: Story = {
    render: () => (
        <AdminStoryProvider
            boundary={graphqlBoundary({})}
            auth={{trustee: "trustee1", username: "trustee1", isAuthenticated: true}}
            settings={{HASURA_URL: CUSTODY_PLATFORM}}
        >
            <TrusteeDashboard />
        </AdminStoryProvider>
    ),
    beforeEach: () => {
        queuedRequest = undefined
        submittedProof = undefined
        const previousToken = localStorage.getItem("token")
        localStorage.setItem("token", "synthetic-trustee-token")
        storyFetch({
            [CUSTODY_PLATFORM]: ({body}) => {
                const input = JSON.parse(body || "{}") as {
                    query: string
                    variables?: {result: {status: string; custody_proof_json?: string}}
                }
                if (input.query.includes("trustee_event_updates"))
                    return {
                        status: 200,
                        body: JSON.stringify({
                            data: {
                                trustee_event_updates: {
                                    boards: [],
                                    actions:
                                        queuedRequest && !submittedProof
                                            ? [
                                                  {
                                                      id: "queued-tally",
                                                      kind: "CREATE_CUSTODY_PROOF",
                                                      request: queuedRequest,
                                                  },
                                              ]
                                            : [],
                                },
                            },
                        }),
                    }
                if (input.query.includes("submit_trustee_action_result")) {
                    expect(input.variables?.result.status).toBe("READY")
                    submittedProof = input.variables?.result.custody_proof_json
                    return {
                        status: 200,
                        body: JSON.stringify({
                            data: {submit_trustee_action_result: {accepted: true}},
                        }),
                    }
                }
                throw new Error("Unexpected custody fixture operation")
            },
        })
        return () => {
            if (previousToken === null) localStorage.removeItem("token")
            else localStorage.setItem("token", previousToken)
        }
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await userEvent.click(canvas.getByRole("button", {name: "Generate browser keys"}))
        const signing = await canvas.findByRole("textbox", {name: "Signing public key"})
        const encryption = canvas.getByRole("textbox", {name: "Share encryption public key"})
        queuedRequest = {
            tenant_id: "11111111-1111-4111-8111-111111111111",
            election_event_id: "22222222-2222-4222-8222-222222222222",
            keys_ceremony_id: "33333333-3333-4333-8333-333333333333",
            tally_session_id: "44444444-4444-4444-8444-444444444444",
            trustee_name: "trustee1",
            signing_public_key: (signing as HTMLInputElement).value,
            share_encryption_public_key: (encryption as HTMLInputElement).value,
        }
        // The tally scope comes from queued work, even with the manual field blank.
        expect(canvas.queryByRole("textbox", {name: "Tally session ID"})).toBeNull()
        await userEvent.click(canvas.getByRole("button", {name: "Start browser trustee"}))
        await waitFor(() => expect(submittedProof).toBeTruthy())
        const proof = JSON.parse(submittedProof || "{}") as {request: unknown; signature: string}
        await expect(proof.request).toEqual(queuedRequest)
        await expect(proof.signature).not.toBe("")
        await userEvent.click(canvas.getByRole("button", {name: "Advanced"}))
        await expect(
            await canvas.findByRole("button", {name: "Generate proof file manually"})
        ).toBeDisabled()
        expect(canvas.getByRole("textbox", {name: "Tally session ID"})).toBeDisabled()
        await userEvent.click(canvas.getByRole("button", {name: "Stop browser trustee"}))
        await waitFor(() =>
            expect(canvas.getByRole("button", {name: "Start browser trustee"})).toBeEnabled()
        )
    },
}

export const ManualProofFileIsAnAdvancedOption: Story = {
    render: AutomaticTallyCustody.render,
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        expect(canvas.queryByRole("button", {name: "Generate proof file manually"})).toBeNull()
        await userEvent.click(canvas.getByRole("button", {name: "Advanced"}))
        const manual = await canvas.findByRole("button", {name: "Generate proof file manually"})
        await expect(manual).toBeDisabled()
        await expect(
            canvas.getByText(/A running browser trustee generates custody proofs automatically/)
        ).toBeVisible()
        await userEvent.click(canvas.getByRole("button", {name: "Generate browser keys"}))
        await waitFor(() => expect(manual).toBeEnabled())
        const tallyId = "44444444-4444-4444-8444-444444444444"
        await userEvent.type(canvas.getByRole("textbox", {name: "Tally session ID"}), tallyId)
        const request = {
            tenant_id: "11111111-1111-4111-8111-111111111111",
            election_event_id: "22222222-2222-4222-8222-222222222222",
            keys_ceremony_id: "33333333-3333-4333-8333-333333333333",
            tally_session_id: null,
            trustee_name: "trustee1",
            signing_public_key: (
                canvas.getByRole("textbox", {name: "Signing public key"}) as HTMLInputElement
            ).value,
            share_encryption_public_key: (
                canvas.getByRole("textbox", {
                    name: "Share encryption public key",
                }) as HTMLInputElement
            ).value,
        }
        let downloaded: Blob | undefined
        let filename: string | undefined
        const objectUrl = spyOn(URL, "createObjectURL").mockImplementation((value) => {
            if (!(value instanceof Blob)) throw new Error("Expected proof file")
            downloaded = value
            return "blob:synthetic-custody-proof"
        })
        const anchor = spyOn(HTMLAnchorElement.prototype, "click").mockImplementation(function (
            this: HTMLAnchorElement
        ) {
            filename = this.download
        })
        try {
            await userEvent.upload(
                canvas.getByLabelText("Custody request"),
                new File([JSON.stringify(request)], "custody-request.json", {
                    type: "application/json",
                })
            )
            await waitFor(() => expect(downloaded).toBeDefined())
            const proof = JSON.parse(await downloaded!.text()) as {
                request: unknown
                signature: string
            }
            expect(filename).toBe("trustee1-custody-proof.json")
            expect(proof.request).toEqual({...request, tally_session_id: tallyId})
            expect(proof.signature).not.toBe("")
        } finally {
            objectUrl.mockRestore()
            anchor.mockRestore()
        }
    },
}
