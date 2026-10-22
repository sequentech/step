// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, within, userEvent, waitFor, spyOn} from "storybook/test"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {storyFetch} from "@/__stories__/storyNetwork"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {TrusteeDashboard} from "./TrusteeDashboard"
import {loadBrowserTrustee} from "@/services/BrowserTrustee"

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
        expect(
            canvas.queryByRole("button", {name: "Generate signed custody proof manually"})
        ).toBeNull()
        expect(canvas.queryByRole("textbox", {name: "Tally session ID"})).toBeNull()
        expect(canvas.queryByRole("button", {name: "Local session history"})).toBeNull()
        await expect(canvas.getByRole("button", {name: "Refresh local history"})).toBeVisible()
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

const HISTORY_PLATFORM = "https://history-fixture.invalid/graphql"
const parentBoard = "dkg_11111111111141118111111111111111"
const tallyBoard = "tally_22222222222242228222222222222222_1"
const historyRender = () => (
    <AdminStoryProvider
        boundary={graphqlBoundary({})}
        auth={{trustee: "trustee1", getAccessToken: () => "synthetic-trustee-token"}}
        settings={{HASURA_URL: HISTORY_PLATFORM, WBRAID_BOARD_URL: "/b4"}}
    >
        <TrusteeDashboard />
    </AdminStoryProvider>
)
const historyBoundary = () => {
    storyFetch({
        [HISTORY_PLATFORM]: () => ({
            status: 200,
            body: JSON.stringify({
                data: {
                    list_trustee_boards: {
                        boards: [{name: tallyBoard, kind: "TALLY", parent: parentBoard}],
                    },
                },
            }),
        }),
    })
}
const restoreHistory = async (canvasElement: HTMLElement) => {
    const module = await loadBrowserTrustee()
    const keys = module.BrowserTrustee.generateKeys()
    // Canonical Shares/PublicKey: tag, configuration hash, content hash, sender index.
    const shares = `01${"11".repeat(64)}${"22".repeat(64)}0000000000000001`
    const publicKey = `02${"11".repeat(64)}${"33".repeat(64)}0000000000000001`
    await userEvent.upload(
        within(canvasElement).getByLabelText("Restore browser backup"),
        new File(
            [
                JSON.stringify({
                    version: 1,
                    keys,
                    board_url: "/b4",
                    stores: [
                        {
                            board: parentBoard,
                            predicates: [shares, publicKey],
                            own_posts: [[shares, "33333333-3333-4333-8333-333333333333"]],
                        },
                    ],
                }),
            ],
            "trustee1-browser-sessions.json",
            {type: "application/json"}
        )
    )
    await within(canvasElement).findByRole("textbox", {name: "Signing public key"})
    await waitFor(() =>
        expect(
            within(canvasElement).getByRole("button", {name: "Refresh local history"})
        ).toBeEnabled()
    )
    return keys
}

export const RestoredLocalHistory: Story = {
    render: historyRender,
    beforeEach: historyBoundary,
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByRole("heading", {name: "Local session history"})).toBeVisible()
        const keys = await restoreHistory(canvasElement)
        await expect(
            await canvas.findByText("History available · 2 records · 1 own posts")
        ).toBeVisible()
        const signing = (
            canvas.getByRole("textbox", {name: "Signing public key"}) as HTMLInputElement
        ).value
        const database = `sequent-wbraid:/b4:${signing}:${parentBoard}`
        const entry = canvas.getByRole("button", {name: new RegExp(parentBoard)})
        expect(entry).toHaveAttribute("aria-expanded", "false")
        expect(canvas.queryByText(database)).toBeNull()
        entry.focus()
        await userEvent.keyboard("{Enter}")
        await expect(await canvas.findByText(database)).toBeVisible()
        await userEvent.click(entry)
        await waitFor(() => expect(canvas.queryByText(database)).toBeNull())
        await expect(canvas.getByText("History available · 2 records · 1 own posts")).toBeVisible()
        await expect(canvas.getByRole("button", {name: "Refresh local history"})).toBeVisible()
        await userEvent.click(canvas.getByRole("button", {name: "Refresh boards"}))
        await expect(
            await canvas.findByText("Empty history · 0 records · 0 own posts")
        ).toBeVisible()
        await expect(canvas.getByText("History available · 2 records · 1 own posts")).toBeVisible()
        // The same identity reopens the durable records after locking/importing keys.
        await userEvent.click(canvas.getByRole("button", {name: "Lock keys"}))
        await userEvent.upload(
            canvas.getByLabelText("Import private keys"),
            new File([keys], "keys.toml")
        )
        await expect(
            await canvas.findByText("History available · 2 records · 1 own posts")
        ).toBeVisible()
        // An absolute URL is a different storage namespace even on the same origin.
        await userEvent.click(canvas.getByRole("button", {name: "Lock keys"}))
        const url = canvas.getByRole("textbox", {name: "Board service URL"})
        await userEvent.clear(url)
        await userEvent.type(url, `${location.origin}/b4///`)
        await userEvent.upload(
            canvas.getByLabelText("Import private keys"),
            new File([keys], "keys.toml")
        )
        await userEvent.click(await canvas.findByRole("button", {name: new RegExp(parentBoard)}))
        await expect(await canvas.findByText(/Parent DKG history is empty/)).toBeVisible()
        await expect(canvas.queryByText(/History available/)).toBeNull()
        await userEvent.click(canvas.getByRole("button", {name: "Lock keys"}))
        await userEvent.clear(url)
        await userEvent.type(url, "/b4")
        await userEvent.upload(
            canvas.getByLabelText("Import private keys"),
            new File([keys], "keys.toml")
        )
        await expect(
            await canvas.findByText("History available · 2 records · 1 own posts")
        ).toBeVisible()
    },
}

export const MissingParentHistory: Story = {
    render: historyRender,
    beforeEach: historyBoundary,
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await userEvent.click(canvas.getByRole("button", {name: "Refresh boards"}))
        await userEvent.click(canvas.getByRole("button", {name: "Generate browser keys"}))
        await userEvent.click(await canvas.findByRole("button", {name: new RegExp(parentBoard)}))
        await expect(await canvas.findByText(/Parent DKG history is empty/)).toBeVisible()
        expect(canvas.getByRole("button", {name: new RegExp(tallyBoard)})).toHaveAttribute(
            "aria-expanded",
            "false"
        )
        const history = within(canvas.getByRole("region", {name: "Local session history"}))
        expect(history.getAllByText("Empty history · 0 records · 0 own posts")).toHaveLength(2)
        await expect(history.getByText(parentBoard)).toBeVisible()
        await expect(history.getByText(tallyBoard)).toBeVisible()
    },
}

export const UnreadableLocalHistory: Story = {
    render: historyRender,
    beforeEach: historyBoundary,
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const keys = await restoreHistory(canvasElement)
        await expect(
            await canvas.findByText("History available · 2 records · 1 own posts")
        ).toBeVisible()
        const signing = (
            canvas.getByRole("textbox", {name: "Signing public key"}) as HTMLInputElement
        ).value
        const db = await new Promise<IDBDatabase>((resolve, reject) => {
            const request = indexedDB.open(`sequent-wbraid:/b4:${signing}:${parentBoard}`)
            request.onsuccess = () => resolve(request.result)
            request.onerror = () => reject(request.error)
        })
        try {
            await new Promise<void>((resolve, reject) => {
                const tx = db.transaction("predicates", "readwrite")
                const invalid = new Uint8Array([255])
                tx.objectStore("predicates").put(invalid, invalid)
                tx.oncomplete = () => resolve()
                tx.onerror = () => reject(tx.error)
            })
        } finally {
            db.close()
        }
        await userEvent.click(canvas.getByRole("button", {name: "Refresh local history"}))
        await expect(
            await canvas.findByText(
                /Unable to read local history:.*failed to deserialize persisted predicate/
            )
        ).toBeVisible()
        expect(canvas.queryByText(/Empty history/)).toBeNull()
        expect(canvas.queryByText(/History available/)).toBeNull()
        const module = await loadBrowserTrustee()
        const engine = new module.BrowserTrustee("trustee1", keys, "/b4")
        try {
            await expect(engine.sessionHistory('["../invalid"]')).rejects.toBeTruthy()
            // A partial result must not be returned when a later board cannot be read.
            await expect(
                engine.sessionHistory(JSON.stringify([tallyBoard, parentBoard]))
            ).rejects.toContain("failed to deserialize persisted predicate")
        } finally {
            engine.free()
        }
    },
}

const CUSTODY_PLATFORM = "https://custody-fixture.invalid/graphql"
let queuedRequest: Record<string, unknown> | undefined
let submittedProof: string | undefined
export const AutomaticTallyCustody: Story = {
    render: () => (
        <AdminStoryProvider
            boundary={graphqlBoundary({})}
            auth={{
                trustee: "trustee1",
                username: "trustee1",
                isAuthenticated: true,
                getAccessToken: () => "synthetic-trustee-token",
            }}
            settings={{HASURA_URL: CUSTODY_PLATFORM}}
        >
            <TrusteeDashboard />
        </AdminStoryProvider>
    ),
    beforeEach: () => {
        queuedRequest = undefined
        submittedProof = undefined
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
            await canvas.findByRole("button", {name: "Generate signed custody proof manually"})
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
        expect(
            canvas.queryByRole("button", {name: "Generate signed custody proof manually"})
        ).toBeNull()
        await userEvent.click(canvas.getByRole("button", {name: "Advanced"}))
        const manual = await canvas.findByRole("button", {
            name: "Generate signed custody proof manually",
        })
        await expect(manual).toBeDisabled()
        await expect(
            canvas.getByText(
                /A running browser trustee generates signed custody proofs automatically/
            )
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
            // Custody files saved with the old filename remain usable.
            await userEvent.upload(
                canvas.getByLabelText("Custody file"),
                new File([JSON.stringify(request)], "custody-request.json", {
                    type: "application/json",
                })
            )
            await waitFor(() => expect(downloaded).toBeDefined())
            const proof = JSON.parse(await downloaded!.text()) as {
                request: unknown
                signature: string
            }
            expect(filename).toBe("trustee1-signed-custody-proof.json")
            expect(proof.request).toEqual({...request, tally_session_id: tallyId})
            expect(proof.signature).not.toBe("")
        } finally {
            objectUrl.mockRestore()
            anchor.mockRestore()
        }
    },
}
