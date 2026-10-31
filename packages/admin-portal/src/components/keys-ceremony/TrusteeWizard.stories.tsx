// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext} from "react"
import type {StoryObj} from "@storybook/react"
import type {WidgetMeta} from "@/__stories__/widgetStory"
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
import {CUSTODY_REQUEST as REQUEST_FIXTURE} from "./__stories__/KeysCeremonyFixture"
import {sha256} from "./__stories__/CustodyProofSigningFixture"
import {TrusteeWizard} from "./TrusteeWizard"

const KEYS_ID = "44444444-4444-4444-8444-444444444444"
const CUSTODY_REQUEST = JSON.stringify({
    ...JSON.parse(REQUEST_FIXTURE),
    election_event_id: EVENT_ID,
    keys_ceremony_id: KEYS_ID,
    trustee_name: "Alice",
})
const KEY = JSON.stringify({
    request: JSON.parse(CUSTODY_REQUEST),
    signature: "synthetic-custody-signature",
})
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
        public_key_hash: "synthetic-public-key-hash",
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
    custody: "required" | "confirmed"
    execution: "IN_PROGRESS" | "SUCCESS" | "CANCELLED" | "FAILED"
    automated: boolean
}
let currentCeremony: Sequent_Backend_Keys_Ceremony
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
                        electionEvent={{
                            ...event,
                            presentation: {
                                ...event.presentation,
                                ceremonies_policy: currentCeremony.settings.policy,
                            },
                        }}
                        currentCeremony={currentCeremony}
                        goBack={goBack}
                    />
                </SettingsContext.Provider>
            </AuthContext.Provider>
        </AdminStoryProvider>
    )
}
const meta = {
    title: "Admin/Keys ceremony/TrusteeWizard",
    component: TrusteeWizard,
    args: {
        download: "success",
        checkFailure: false,
        custody: "required",
        execution: "IN_PROGRESS",
        automated: false,
    },
    beforeEach: ({args}) => {
        currentCeremony = structuredClone(ceremony)
        currentCeremony.execution_status = args.execution
        if (args.custody === "confirmed") {
            currentCeremony.status.trustees[0].status = "CUSTODY_CONFIRMED"
        }
        if (args.execution === "SUCCESS") {
            currentCeremony.status.trustees.forEach((trustee: {status: string}) => {
                trustee.status = "CUSTODY_CONFIRMED"
            })
        }
        if (args.automated) {
            currentCeremony.settings.policy = "automated-ceremonies"
        }
        blobs = []
        downloads = []
        goBack.mockClear()
        let attempts = 0
        boundary = graphqlBoundary({
            GetCustodyRequest: () => {
                if (args.download === "fail-once" && attempts++ === 0)
                    throw new Error("Synthetic download unavailable")
                return {
                    data: {
                        get_custody_request: {
                            custody_request_json: args.download === "empty" ? "" : CUSTODY_REQUEST,
                        },
                    },
                }
            },
            ConfirmKeyCustody: ({variables}) => {
                if (args.checkFailure) throw new Error("Synthetic check service unavailable")
                return {data: {confirm_key_custody: {is_valid: variables.custodyProofJson === KEY}}}
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
                return {data: currentCeremony as unknown as RecordType}
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
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>
async function downloadStep(canvasElement: HTMLElement) {
    const canvas = within(canvasElement)
    await expect(await canvas.findByRole("heading", {name: "Trustee Key Ceremony"})).toBeVisible()
    await userEvent.click(canvas.getByRole("button", {name: "Next"}))
    const download = await canvas.findByRole("button", {
        name: "Download Custody File",
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
            name: "Alice-custody.json",
            href: "blob:synthetic-key-download",
        },
    ])
    expect(await blobs[0].text()).toBe(CUSTODY_REQUEST)
    expect(blobs[0].type).toBe("application/json")
    expect(boundary.calls).toEqual([
        {
            name: "GetCustodyRequest",
            variables: {electionEventId: EVENT_ID, keysCeremonyId: KEYS_ID},
            headers: {"x-hasura-role": "trustee-ceremony"},
        },
    ])
    await userEvent.click(canvas.getByRole("button", {name: "Next"}))
    const dialog = await within(document.body).findByRole("dialog")
    await waitFor(() => expect(dialog).toBeVisible())
    return within(dialog)
}
async function checkStep(canvasElement: HTMLElement) {
    const dialog = await downloadKey(canvasElement)
    await userEvent.click(dialog.getByRole("checkbox", {name: "First copy saved"}))
    await userEvent.click(dialog.getByRole("checkbox", {name: "Second copy saved"}))
    await userEvent.click(dialog.getByRole("button", {name: "Confirm Saved Copies and Continue"}))
    await waitFor(() => expect(within(document.body).queryByRole("dialog")).not.toBeInTheDocument())
    await within(canvasElement).findByRole("heading", {
        name: "Confirm Trustee Custody",
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
        const confirm = dialog.getByRole("button", {name: "Confirm Saved Copies and Continue"})
        expect(confirm).toBeDisabled()
        await userEvent.click(dialog.getByRole("checkbox", {name: "First copy saved"}))
        expect(confirm).toBeDisabled()
        await userEvent.click(dialog.getByRole("checkbox", {name: "Second copy saved"}))
        expect(confirm).toBeEnabled()
        await userEvent.click(confirm)
        await waitFor(() =>
            expect(within(document.body).queryByRole("dialog")).not.toBeInTheDocument()
        )
        await expect(
            await within(canvasElement).findByRole("heading", {
                name: "Confirm Trustee Custody",
            })
        ).toBeVisible()
        expect(within(canvasElement).getByRole("button", {name: "Next"})).toBeDisabled()
        await expect(within(canvasElement).getByText("Custody confirmation required")).toBeVisible()
        expect(within(canvasElement).queryByRole("button", {name: "More actions"})).toBeNull()
    },
}

export const ConfirmedCustodyIsOptionalWhileOtherTrusteesFinish: Story = {
    args: {custody: "confirmed"},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText("Custody confirmed")).toBeVisible()
        await expect(
            canvas.getByText("Your part is complete. No further action is required from you.")
        ).toBeVisible()
        await expect(
            canvas.getByText(
                "Waiting for the remaining trustees to complete their custody confirmation and any required approval."
            )
        ).toBeVisible()
        expect(canvas.queryByRole("button", {name: "Next"})).toBeNull()
        expect(canvas.queryByText("Reconfirm custody (optional)")).toBeNull()
        const moreActions = canvas.getByRole("button", {name: "More actions"})
        await userEvent.click(moreActions)
        const menu = within(await within(document.body).findByRole("menu"))
        const reconfirm = menu.getByRole("menuitem", {name: "Reconfirm custody (optional)"})
        await waitFor(() => expect(reconfirm).toBeVisible())
        await userEvent.click(reconfirm)
        await expect(
            await canvas.findByRole("heading", {name: "Reconfirm custody (optional)"})
        ).toBeVisible()
        expect(canvas.queryByText("Custody confirmation required")).toBeNull()
        await userEvent.click(canvas.getByRole("button", {name: "Back"}))
        await expect(await canvas.findByText("Custody confirmed")).toBeVisible()
        expect(boundary.calls).toEqual([])
    },
}

export const CompletedCeremonyNeedsNoFurtherCustody: Story = {
    args: {custody: "confirmed", execution: "SUCCESS"},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText("Custody confirmed")).toBeVisible()
        expect(canvas.queryByText(/Waiting for the remaining trustees/)).toBeNull()
        expect(canvas.queryByRole("button", {name: "Next"})).toBeNull()
        await userEvent.click(canvas.getByRole("button", {name: "More actions"}))
        const reconfirm = await within(document.body).findByRole("menuitem", {
            name: "Reconfirm custody (optional)",
        })
        await waitFor(() => expect(reconfirm).toBeVisible())
    },
}

export const AutomatedCeremonyDoesNotOfferManualReconfirmation: Story = {
    args: {custody: "confirmed", execution: "SUCCESS", automated: true},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByText("Status: SUCCESS")
        expect(canvas.queryByText("Custody confirmed")).toBeNull()
        expect(canvas.queryByRole("button", {name: "More actions"})).toBeNull()
    },
}

export const CancelledCeremonyHasNoCustodyAction: Story = {
    args: {custody: "confirmed", execution: "CANCELLED"},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByText("Status: CANCELLED")
        expect(canvas.queryByText("Custody confirmed")).toBeNull()
        expect(canvas.queryByRole("button", {name: "More actions"})).toBeNull()
        expect(canvas.queryByRole("button", {name: "Next"})).toBeNull()
    },
}

export const FailedCeremonyHasNoCustodyAction: Story = {
    ...CancelledCeremonyHasNoCustodyAction,
    args: {custody: "confirmed", execution: "FAILED"},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByText("Status: FAILED")
        expect(canvas.queryByText("Custody confirmed")).toBeNull()
        expect(canvas.queryByRole("button", {name: "More actions"})).toBeNull()
        expect(canvas.queryByRole("button", {name: "Next"})).toBeNull()
    },
}
export const ValidBackupIsVerified: Story = {
    play: async ({canvasElement}) => {
        await checkStep(canvasElement)
        await uploadKey(canvasElement, KEY)
        const canvas = within(canvasElement)
        await waitFor(() =>
            expect(canvas.getByText("Trustee custody confirmed successfully.")).toBeVisible()
        )
        expect(canvas.getByRole("button", {name: "Next"})).toBeEnabled()
        expect(boundary.calls[1]).toEqual({
            name: "ConfirmKeyCustody",
            variables: {
                electionEventId: EVENT_ID,
                keysCeremonyId: KEYS_ID,
                custodyProofJson: KEY,
                custodyProofSha256: await sha256(KEY),
            },
            headers: {"x-hasura-role": "trustee-ceremony"},
        })
    },
}
export const WrongBackupCanBeReplaced: Story = {
    play: async ({canvasElement}) => {
        await checkStep(canvasElement)
        await uploadKey(canvasElement, "wrong trustee backup")
        const canvas = within(canvasElement)
        await waitFor(() =>
            expect(canvas.getByText("Invalid signed custody proof, please try again")).toBeVisible()
        )
        expect(canvas.getByRole("button", {name: "Next"})).toBeDisabled()
        expect(boundary.calls[1].variables.custodyProofJson).toBe("wrong trustee backup")
        await uploadKey(canvasElement, KEY)
        await waitFor(() => expect(canvas.getByRole("button", {name: "Next"})).toBeEnabled())
        expect(boundary.calls.filter((call) => call.name === "ConfirmKeyCustody")).toHaveLength(2)
    },
}
export const DownloadErrorCanBeRetried: Story = {
    args: {download: "fail-once"},
    play: async ({canvasElement}) => {
        const {canvas, download} = await downloadStep(canvasElement)
        await userEvent.click(download)
        await waitFor(() =>
            expect(
                canvas.getByText("The custody file could not be downloaded. Please try again.")
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
        await userEvent.click(dialog.getByRole("checkbox", {name: "First copy saved"}))
        await userEvent.click(dialog.getByRole("button", {name: "Go Back"}))
        await waitFor(() =>
            expect(within(document.body).queryByRole("dialog")).not.toBeInTheDocument()
        )
        await userEvent.click(within(canvasElement).getByRole("button", {name: "Next"}))
        const reopened = within(await within(document.body).findByRole("dialog"))
        expect(reopened.getByRole("checkbox", {name: "First copy saved"})).not.toBeChecked()
        expect(
            reopened.getByRole("button", {name: "Confirm Saved Copies and Continue"})
        ).toBeDisabled()
    },
}
