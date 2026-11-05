// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {Button} from "@mui/material"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {
    AdminStoryProvider,
    EVENT_ID,
    TENANT_ID,
    graphqlBoundary,
} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import type {ISigningApi} from "@/lib/signing/api"
import {RESUME_KEY} from "@/lib/signing/request"
import {SigningAction, SigningRequestStatus} from "@/lib/signing/types"
import {SigningProvider, useSigningRequest} from "./SigningProvider"
import {
    ANA,
    JOSE,
    MARIA,
    ORGANIZATIONS,
    REQUEST_ID,
    SIGN_ROLES,
    fakeApi,
    makePanel,
    memoryStorage,
    signedInAs,
    type IPanelOptions,
    type IStoryPerson,
} from "./__stories__/fixtures"

interface Scenario {
    viewer: IStoryPerson
    panel: IPanelOptions
    /** The handover note the member who signed out left. */
    note?: "valid" | "other-tenant" | "other-event" | "expired"
    /** A completion action that fails to render. */
    brokenCompletion?: boolean
}

const noteFor = (kind: NonNullable<Scenario["note"]>) => ({
    requestId: REQUEST_ID,
    tenantId: kind === "other-tenant" ? "99999999-9999-4999-8999-999999999999" : TENANT_ID,
    eventId: kind === "other-event" ? "99999999-9999-4999-8999-999999999998" : EVENT_ID,
    expiresAt: new Date(Date.now() + (kind === "expired" ? -60_000 : 600_000)).toISOString(),
})

let boundary: ReturnType<typeof graphqlBoundary>
let api: ReturnType<typeof fakeApi>
let storage: Storage

/** A completion action that throws while rendering. */
function Broken(): React.ReactElement {
    throw new Error("Synthetic render failure")
}

/** Any screen that got a `signing_request` back from a guarded route. */
function StartsARequest() {
    const signing = useSigningRequest()
    return (
        <Button variant="contained" onClick={() => signing.open(REQUEST_ID)}>
            Generate election returns
        </Button>
    )
}

const meta = {
    title: "Admin/Signing/SigningProvider",
    component: SigningProvider,
    args: {viewer: MARIA, panel: {}},
    beforeEach: async ({args}) => {
        boundary = graphqlBoundary({})
        api = fakeApi(await makePanel(args.panel))
        storage = memoryStorage(args.note ? {[RESUME_KEY]: JSON.stringify(noteFor(args.note))} : {})
    },
    render: ({viewer, brokenCompletion}) => (
        <AdminStoryProvider
            boundary={boundary}
            roles={SIGN_ROLES}
            auth={signedInAs(viewer)}
            tenantRecord={ORGANIZATIONS.first}
        >
            <SigningProvider
                api={api as ISigningApi}
                storage={storage}
                completionActions={{
                    [SigningAction.GenerateElectionReturns]: () =>
                        brokenCompletion ? (
                            <Broken />
                        ) : (
                            <Button variant="contained">Download signed PDF</Button>
                        ),
                }}
            >
                <StartsARequest />
            </SigningProvider>
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

export const OpensFromAnywhere: Story = {
    play: async ({canvasElement}) => {
        const body = within(document.body)
        await expect(body.queryByRole("heading", {name: /^Election returns/})).toBeNull()
        await userEvent.click(
            within(canvasElement).getByRole("button", {name: "Generate election returns"})
        )
        await expect(await body.findByRole("heading", {name: /^Election returns · /})).toBeVisible()
        await expect(api.getRequest).toHaveBeenCalledWith(REQUEST_ID)
        // Opened without asking to sign: the dialog waits for the Sign button.
        await expect(body.queryByRole("dialog", {name: "Sign the election returns"})).toBeNull()
    },
}

export const CompletionActionsOfTheAction: Story = {
    args: {
        viewer: ANA,
        panel: {status: SigningRequestStatus.Executed, signed: [MARIA, JOSE, ANA]},
    },
    play: async ({canvasElement}) => {
        await userEvent.click(
            within(canvasElement).getByRole("button", {name: "Generate election returns"})
        )
        await expect(
            await within(document.body).findByRole("button", {name: "Download signed PDF"})
        ).toBeVisible()
    },
}

/** Maria handed over; Jose signed in on the same computer and gets the request to sign. */
export const ResumesForTheNextMember: Story = {
    args: {viewer: JOSE, panel: {signed: [MARIA]}, note: "valid"},
    play: async () => {
        const body = within(document.body)
        const dialog = await body.findByRole("dialog", {name: "Sign the election returns"})
        await waitFor(() => expect(dialog).toBeVisible())
        await expect(
            within(dialog).getByText(new RegExp(`^You are signing as ${JOSE.name}`))
        ).toBeVisible()
        await expect(body.getByText(`${JOSE.name} (you)`)).toBeInTheDocument()
        // The note is used once.
        await expect(storage.getItem(RESUME_KEY)).toBeNull()
    },
}

/** The member who handed over signs back in: the panel opens, but they already signed. */
export const ResumesWithoutDialogForASigner: Story = {
    args: {viewer: MARIA, panel: {signed: [MARIA]}, note: "valid"},
    play: async () => {
        const body = within(document.body)
        await expect(await body.findByRole("heading", {name: /^Election returns · /})).toBeVisible()
        await expect(body.getByTestId("signing-status")).toHaveTextContent("Waiting · 1 of 3")
        await expect(body.queryByRole("dialog", {name: "Sign the election returns"})).toBeNull()
    },
}

/** Notes for another tenant, or past their expiry, are dropped unused. */
export const IgnoresAStaleNote: Story = {
    args: {viewer: JOSE, note: "expired"},
    play: async () => {
        await expect(storage.getItem(RESUME_KEY)).toBeNull()
        await expect(
            within(document.body).queryByRole("heading", {name: /^Election returns · /})
        ).toBeNull()
        await expect(api.getRequest).not.toHaveBeenCalled()
    },
}

export const IgnoresANoteOfAnotherTenant: Story = {
    args: {viewer: JOSE, note: "other-tenant"},
    play: async () => {
        await expect(storage.getItem(RESUME_KEY)).toBeNull()
        await expect(api.getRequest).not.toHaveBeenCalled()
    },
}

/** A note naming another event than the request's: the panel closes, nothing to sign. */
export const IgnoresANoteOfAnotherEvent: Story = {
    args: {viewer: JOSE, panel: {signed: [MARIA]}, note: "other-event"},
    play: async () => {
        await waitFor(() => expect(api.getRequest).toHaveBeenCalledTimes(1))
        await waitFor(() =>
            expect(
                within(document.body).queryByRole("dialog", {name: "Sign the election returns"})
            ).toBeNull()
        )
        await waitFor(() =>
            expect(
                within(document.body).queryByRole("heading", {name: /^Election returns · /})
            ).toBeNull()
        )
    },
}

/** A request that fails to render doesn't take the page with it. */
export const RenderFailureIsContained: Story = {
    args: {
        viewer: ANA,
        panel: {status: SigningRequestStatus.Executed, signed: [MARIA, JOSE, ANA]},
        brokenCompletion: true,
    },
    parameters: {widgets: ["SigningErrorBoundary", "RenderFailure"]},
    play: async ({canvasElement}) => {
        await userEvent.click(
            within(canvasElement).getByRole("button", {name: "Generate election returns"})
        )
        const failure = await within(document.body).findByText(
            "The signing request could not be shown. Close it and open it again."
        )
        await waitFor(() => expect(failure).toBeVisible())
        // The page around it is still there.
        await expect(
            within(canvasElement).getByRole("button", {name: "Generate election returns"})
        ).toBeVisible()
    },
}
