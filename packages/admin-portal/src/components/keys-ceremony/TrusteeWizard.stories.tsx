// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext} from "react"
import type {Meta, StoryObj} from "@storybook/react"
import {expect, fn, spyOn, userEvent, waitFor, within} from "storybook/test"
import type {RaRecord} from "react-admin"
import {
    AdminStoryProvider,
    EVENT_ID,
    TENANT_ID,
    graphqlBoundary,
} from "@/__stories__/AdminStoryProvider"
import {dataBoundary} from "@/__stories__/dataBoundary"
import {AuthContext} from "@/providers/AuthContextProvider"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import type {Sequent_Backend_Election_Event, Sequent_Backend_Keys_Ceremony} from "@/gql/graphql"
import {TrusteeWizard} from "./TrusteeWizard"

const KEYS_ID = "44444444-4444-4444-8444-444444444444"
const KEY = "c3ludGhldGljLWVuY3J5cHRlZC1rZXk="
const event = {
    id: EVENT_ID,
    tenant_id: TENANT_ID,
    presentation: {i18n: {en: {name: "Council"}}, ceremonies_policy: "manual-ceremonies"},
} as unknown as Sequent_Backend_Election_Event
const ceremony = {
    id: KEYS_ID,
    tenant_id: TENANT_ID,
    election_event_id: EVENT_ID,
    execution_status: "IN_PROGRESS",
    threshold: 2,
    settings: {policy: "manual-ceremonies"},
    status: {
        public_key: "synthetic-public-key",
        trustees: [
            {name: "Alice", status: "KEY_GENERATED"},
            {name: "Bob", status: "KEY_GENERATED"},
        ],
        logs: [],
    },
} as unknown as Sequent_Backend_Keys_Ceremony
interface Scenario {
    download: "success" | "empty" | "fail-once"
    checkFailure: boolean
}
let boundary: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof dataBoundary>
let blobs: Blob[]
let downloads: {name: string; href: string}[]
const goBack = fn()
function Fixture() {
    const auth = useContext(AuthContext)
    const settings = useContext(SettingsContext)
    return (
        <AdminStoryProvider boundary={boundary} dataProvider={data.provider}>
            <AuthContext.Provider
                value={{...auth, trustee: "Alice", username: "alice", isAuthorized: () => false}}
            >
                <SettingsContext.Provider
                    value={{
                        ...settings,
                        globalSettings: {
                            ...settings.globalSettings,
                            QUERY_FAST_POLL_INTERVAL_MS: 3_600_000,
                        },
                    }}
                >
                    <TrusteeWizard
                        electionEvent={event}
                        currentCeremony={ceremony}
                        goBack={goBack}
                    />
                </SettingsContext.Provider>
            </AuthContext.Provider>
        </AdminStoryProvider>
    )
}
const meta = {
    title: "Admin/Trustee key backup",
    component: Fixture,
    args: {download: "success", checkFailure: false},
    beforeEach: ({args}) => {
        blobs = []
        downloads = []
        goBack.mockClear()
        let attempts = 0
        boundary = graphqlBoundary({
            GetPrivateKey: () => {
                if (args.download === "fail-once" && attempts++ === 0)
                    throw new Error("Synthetic download unavailable")
                return {
                    data: {
                        get_private_key: {private_key_base64: args.download === "empty" ? "" : KEY},
                    },
                }
            },
            CheckPrivateKey: ({variables}) => {
                if (args.checkFailure) throw new Error("Synthetic check service unavailable")
                return {data: {check_private_key: {is_valid: variables.privateKeyBase64 === KEY}}}
            },
        })
        data = dataBoundary({
            getOne: async <RecordType extends RaRecord>(
                resource: string,
                {id}: {id: string | number}
            ) => {
                if (resource !== "sequent_backend_keys_ceremony" || id !== KEYS_ID) {
                    data.unexpected.push(`${resource}/${id}`)
                    throw new Error("Unexpected ceremony read")
                }
                return {data: ceremony as unknown as RecordType}
            },
        })
        const objectUrl = spyOn(URL, "createObjectURL").mockImplementation((value) => {
            if (!(value instanceof Blob)) throw new Error("Expected downloadable key bytes")
            blobs.push(value)
            return "blob:synthetic-key-download"
        })
        const anchor = spyOn(HTMLAnchorElement.prototype, "click").mockImplementation(function (
            this: HTMLAnchorElement
        ) {
            downloads.push({name: this.download, href: this.href})
        })
        const services = boundary
        const records = data
        return () => {
            try {
                expect(services.unexpected).toEqual([])
                expect(records.unexpected).toEqual([])
            } finally {
                objectUrl.mockRestore()
                anchor.mockRestore()
            }
        }
    },
    render: () => <Fixture />,
} satisfies Meta<Scenario>
export default meta
type Story = StoryObj<typeof meta>
async function downloadStep(canvasElement: HTMLElement) {
    const canvas = within(canvasElement)
    await expect(await canvas.findByRole("heading", {name: "Trustee Key Ceremony"})).toBeVisible()
    await userEvent.click(canvas.getByRole("button", {name: "Next"}))
    const download = await canvas.findByRole("button", {
        name: "Download your Encrypted Private Key",
    })
    expect(canvas.getByRole("button", {name: "Next"})).toBeDisabled()
    return {canvas, download}
}
async function downloadKey(canvasElement: HTMLElement) {
    const {canvas, download} = await downloadStep(canvasElement)
    await userEvent.click(download)
    await waitFor(() => expect(canvas.getByRole("button", {name: "Next"})).toBeEnabled())
    expect(downloads).toEqual([
        {
            name: "encrypted_private_key_trustee_alice_Council.txt",
            href: "blob:synthetic-key-download",
        },
    ])
    expect(await blobs[0].text()).toBe(KEY)
    expect(blobs[0].type).toBe("text/plain")
    expect(boundary.calls).toEqual([
        {
            name: "GetPrivateKey",
            variables: {electionEventId: EVENT_ID, keysCeremonyId: KEYS_ID},
            headers: {},
        },
    ])
    await userEvent.click(canvas.getByRole("button", {name: "Next"}))
    const dialog = await within(document.body).findByRole("dialog")
    await waitFor(() => expect(dialog).toBeVisible())
    return within(dialog)
}
async function checkStep(canvasElement: HTMLElement) {
    const dialog = await downloadKey(canvasElement)
    await userEvent.click(dialog.getByRole("checkbox", {name: "First backup secured"}))
    await userEvent.click(dialog.getByRole("checkbox", {name: "Second backup secured"}))
    await userEvent.click(dialog.getByRole("button", {name: "Confirm Backups and Continue"}))
    await waitFor(() => expect(within(document.body).queryByRole("dialog")).not.toBeInTheDocument())
    await within(canvasElement).findByRole("heading", {
        name: "Check your Encrypted Private Key Backups",
    })
}
async function uploadKey(canvasElement: HTMLElement, key: string) {
    const input = canvasElement.querySelector<HTMLInputElement>('input[type="file"]')
    if (!input) throw new Error("Missing key file chooser")
    await userEvent.upload(input, new File([key], "backup.txt", {type: "text/plain"}))
}
export const DownloadRequiresTwoBackups: Story = {
    play: async ({canvasElement}) => {
        const dialog = await downloadKey(canvasElement)
        const confirm = dialog.getByRole("button", {name: "Confirm Backups and Continue"})
        expect(confirm).toBeDisabled()
        await userEvent.click(dialog.getByRole("checkbox", {name: "First backup secured"}))
        expect(confirm).toBeDisabled()
        await userEvent.click(dialog.getByRole("checkbox", {name: "Second backup secured"}))
        expect(confirm).toBeEnabled()
        await userEvent.click(confirm)
        await waitFor(() =>
            expect(within(document.body).queryByRole("dialog")).not.toBeInTheDocument()
        )
        await expect(
            await within(canvasElement).findByRole("heading", {
                name: "Check your Encrypted Private Key Backups",
            })
        ).toBeVisible()
        expect(within(canvasElement).getByRole("button", {name: "Next"})).toBeDisabled()
    },
}
export const ValidBackupIsVerified: Story = {
    play: async ({canvasElement}) => {
        await checkStep(canvasElement)
        await uploadKey(canvasElement, KEY)
        const canvas = within(canvasElement)
        await waitFor(() => expect(canvas.getByText("Backup verified successfully.")).toBeVisible())
        expect(canvas.getByRole("button", {name: "Next"})).toBeEnabled()
        expect(boundary.calls[1]).toEqual({
            name: "CheckPrivateKey",
            variables: {electionEventId: EVENT_ID, keysCeremonyId: KEYS_ID, privateKeyBase64: KEY},
            headers: {},
        })
    },
}
export const WrongBackupCanBeReplaced: Story = {
    play: async ({canvasElement}) => {
        await checkStep(canvasElement)
        await uploadKey(canvasElement, "wrong trustee backup")
        const canvas = within(canvasElement)
        await waitFor(() =>
            expect(
                canvas.getByText("Invalid Encrypted Private Key Backup, please try again")
            ).toBeVisible()
        )
        expect(canvas.getByRole("button", {name: "Next"})).toBeDisabled()
        expect(boundary.calls[1].variables.privateKeyBase64).toBe("wrong trustee backup")
        await uploadKey(canvasElement, KEY)
        await waitFor(() => expect(canvas.getByRole("button", {name: "Next"})).toBeEnabled())
        expect(boundary.calls.filter((call) => call.name === "CheckPrivateKey")).toHaveLength(2)
    },
}
export const DownloadErrorCanBeRetried: Story = {
    args: {download: "fail-once"},
    play: async ({canvasElement}) => {
        const {canvas, download} = await downloadStep(canvasElement)
        await userEvent.click(download)
        await waitFor(() =>
            expect(
                canvas.getByText("The private key could not be downloaded. Please try again.")
            ).toBeVisible()
        )
        expect(canvas.getByRole("button", {name: "Next"})).toBeDisabled()
        expect(downloads).toEqual([])
        await userEvent.click(download)
        await waitFor(() => expect(canvas.getByRole("button", {name: "Next"})).toBeEnabled())
        expect(downloads).toHaveLength(1)
        expect(boundary.calls).toHaveLength(2)
    },
}
export const EmptyKeyCannotContinue: Story = {
    args: {download: "empty"},
    play: async ({canvasElement}) => {
        const {canvas, download} = await downloadStep(canvasElement)
        await userEvent.click(download)
        await waitFor(() => expect(canvas.getByText("Download error, empty file")).toBeVisible())
        expect(canvas.getByRole("button", {name: "Next"})).toBeDisabled()
        expect(downloads).toEqual([])
    },
}
export const CancelBackupConfirmationClearsAcknowledgements: Story = {
    play: async ({canvasElement}) => {
        const dialog = await downloadKey(canvasElement)
        await userEvent.click(dialog.getByRole("checkbox", {name: "First backup secured"}))
        await userEvent.click(dialog.getByRole("button", {name: "Go Back"}))
        await waitFor(() =>
            expect(within(document.body).queryByRole("dialog")).not.toBeInTheDocument()
        )
        await userEvent.click(within(canvasElement).getByRole("button", {name: "Next"}))
        const reopened = within(await within(document.body).findByRole("dialog"))
        expect(reopened.getByRole("checkbox", {name: "First backup secured"})).not.toBeChecked()
        expect(reopened.getByRole("button", {name: "Confirm Backups and Continue"})).toBeDisabled()
    },
}
