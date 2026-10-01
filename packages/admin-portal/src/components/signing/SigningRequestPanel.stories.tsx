// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {Button} from "@mui/material"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import type {ISigningApi, ISigningPanelData} from "@/lib/signing/api"
import {CancelReason, SigningRequestStatus} from "@/lib/signing/types"
import {IPermissions} from "@/types/keycloak"
import {SigningRequestPanel} from "./SigningRequestPanel"
import {
    ANA,
    CODE,
    COUNTRY,
    JOSE,
    MARIA,
    ORGANIZATIONS,
    POST,
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
    roles: string[]
    panel: IPanelOptions
    /** The request as it reads after a change (a cancellation). */
    after?: IPanelOptions
    loadFails: boolean
    onClose: () => void
    onChange: (data: ISigningPanelData) => void
}

let boundary: ReturnType<typeof graphqlBoundary>
let api: ReturnType<typeof fakeApi>
let completionActions: ReturnType<typeof fn>

const meta = {
    title: "Admin/Signing/SigningRequestPanel",
    component: SigningRequestPanel,
    args: {
        viewer: MARIA,
        roles: SIGN_ROLES,
        panel: {},
        loadFails: false,
        onClose: fn(),
        onChange: fn(),
    },
    parameters: {widgets: ["StatusLine", "SignerList"]},
    beforeEach: async ({args}) => {
        boundary = graphqlBoundary({})
        api = fakeApi(await makePanel(args.panel))
        if (args.loadFails) {
            api.getRequest.mockRejectedValueOnce(new Error("Synthetic Harvest failure"))
        }
        completionActions = fn(() => <Button variant="contained">Download signed PDF</Button>)
    },
    render: ({viewer, roles, onClose, onChange}) => (
        <AdminStoryProvider
            boundary={boundary}
            roles={roles}
            auth={signedInAs(viewer)}
            tenantRecord={ORGANIZATIONS.first}
        >
            <SigningRequestPanel
                requestId="55555555-5555-4555-8555-555555555555"
                api={api as ISigningApi}
                open
                onClose={onClose}
                onChange={onChange}
                completionActions={completionActions}
                storage={memoryStorage()}
            />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const TITLE = `Election returns · ${POST} · ${COUNTRY}`

const panel = async () => {
    const heading = await within(document.body).findByRole("heading", {name: TITLE})
    await waitFor(() => expect(heading).toBeVisible())
    return within(heading.closest(".MuiDrawer-paper") as HTMLElement)
}

const signerRow = (view: ReturnType<typeof within>, person: IStoryPerson) =>
    within(
        view
            .getByRole("list", {name: "Signers"})
            .querySelector(`[data-signer="${person.username}"]`) as HTMLElement
    )

export const WaitingNoSignatures: Story = {
    play: async ({args}) => {
        const view = await panel()
        await expect(
            view.getByText(
                "Started in Reports. Releases the signed election returns for printing and transmission."
            )
        ).toBeVisible()
        await expect(view.getByTestId("signing-status")).toHaveTextContent("Waiting · 0 of 3")
        await expect(view.getByText("Expires at 11:30 UTC")).toBeVisible()
        await expect(
            view.getByText(
                `Needs 3 signatures from ${POST}'s signers, each with their digital certificate.`
            )
        ).toBeVisible()
        await expect(view.getByText(`Election returns, ${POST}, ${COUNTRY}.pdf`)).toBeVisible()
        await expect(view.getByTestId("signing-code")).toHaveTextContent(CODE)
        for (const person of [MARIA, JOSE, ANA]) {
            await expect(signerRow(view, person).getByText("Not signed")).toBeVisible()
        }
        await expect(signerRow(view, MARIA).getByText(`${MARIA.name} (you)`)).toBeVisible()
        await expect(view.getByRole("button", {name: "Sign"})).toBeVisible()
        await expect(view.getByRole("button", {name: "Next member signs in"})).toBeVisible()
        // Maria started it, so she may cancel it.
        await expect(view.getByRole("button", {name: "Cancel request"})).toBeVisible()
        await expect(completionActions).not.toHaveBeenCalled()

        await userEvent.click(view.getByRole("button", {name: "Close the request panel"}))
        await expect(args.onClose).toHaveBeenCalledTimes(1)
    },
}

/** Times follow the election event's zone: 11:30 UTC is 19:30 in Manila, on a 24-hour clock. */
export const EventTimeZone: Story = {
    args: {viewer: JOSE, panel: {signed: [MARIA], timeZone: "Asia/Manila"}},
    play: async () => {
        const view = await panel()
        await expect(view.getByText("Expires at 19:30 GMT+8")).toBeVisible()
        await expect(signerRow(view, MARIA).getByText("Signed 18:41 GMT+8")).toBeVisible()
    },
}

export const OneSignedYouNext: Story = {
    args: {viewer: JOSE, panel: {signed: [MARIA]}},
    play: async () => {
        const view = await panel()
        await expect(view.getByTestId("signing-status")).toHaveTextContent("Waiting · 1 of 3")
        const maria = signerRow(view, MARIA)
        await expect(maria.getByText(/^Signed ([01]\d|2[0-3]):[0-5]\d UTC$/)).toBeVisible()
        await expect(maria.getByText(`Certificate ${MARIA.certificate}`)).toBeVisible()
        await expect(maria.getByText(MARIA.title)).toBeVisible()
        await expect(signerRow(view, JOSE).getByText(`${JOSE.name} (you)`)).toBeVisible()
        await expect(signerRow(view, JOSE).getByText("Not signed")).toBeVisible()
        await expect(view.getByRole("button", {name: "Sign"})).toBeVisible()
        // Jose neither started it nor may cancel others' requests.
        await expect(view.queryByRole("button", {name: "Cancel request"})).toBeNull()

        await userEvent.click(view.getByRole("button", {name: "Sign"}))
        const dialog = await within(document.body).findByRole("dialog", {
            name: "Sign the election returns",
        })
        await waitFor(() => expect(dialog).toBeVisible())
    },
}

export const AlreadySigned: Story = {
    args: {viewer: MARIA, panel: {signed: [MARIA]}},
    play: async () => {
        const view = await panel()
        await expect(view.getByTestId("signing-status")).toHaveTextContent("Waiting · 1 of 3")
        await expect(view.queryByRole("button", {name: "Sign"})).toBeNull()
        await expect(view.getByRole("button", {name: "Next member signs in"})).toBeVisible()
    },
}

export const WithoutSignPermission: Story = {
    args: {
        viewer: JOSE,
        roles: [IPermissions.SIGNING_REQUESTS_READ, IPermissions.SIGNING_REQUESTS_CANCEL],
    },
    play: async () => {
        const view = await panel()
        await expect(view.queryByRole("button", {name: "Sign"})).toBeNull()
        await expect(view.queryByRole("button", {name: "Next member signs in"})).toBeNull()
        // An operator with the cancel permission can end a stuck request.
        await expect(view.getByRole("button", {name: "Cancel request"})).toBeVisible()
    },
}

export const AllSigned: Story = {
    args: {
        viewer: ANA,
        panel: {status: SigningRequestStatus.Executed, signed: [MARIA, JOSE, ANA]},
    },
    play: async () => {
        const view = await panel()
        // As the Signatures tab's Requests list names it.
        await expect(view.getByTestId("signing-status")).toHaveTextContent("Done · 3 of 3")
        await expect(view.getByText(/^Signed at ([01]\d|2[0-3]):[0-5]\d UTC$/)).toBeVisible()
        for (const person of [MARIA, JOSE, ANA]) {
            await expect(
                signerRow(view, person).getByText(`Certificate ${person.certificate}`)
            ).toBeVisible()
        }
        await expect(view.getByRole("button", {name: "Download signed PDF"})).toBeVisible()
        await expect(completionActions).toHaveBeenCalled()
        await expect(view.queryByRole("button", {name: "Sign"})).toBeNull()
        await expect(view.queryByRole("button", {name: "Cancel request"})).toBeNull()
    },
}

export const ActionRuns: Story = {
    args: {
        viewer: ANA,
        panel: {status: SigningRequestStatus.Completed, signed: [MARIA, JOSE, ANA]},
    },
    beforeEach: async () => {
        // The last signature dispatched the action; it runs a moment later.
        api.getRequest.mockResolvedValueOnce(
            await makePanel({status: SigningRequestStatus.Completed, signed: [MARIA, JOSE, ANA]})
        )
        api.getRequest.mockResolvedValue(
            await makePanel({status: SigningRequestStatus.Executed, signed: [MARIA, JOSE, ANA]})
        )
    },
    play: async ({args}) => {
        const view = await panel()
        await expect(view.getByTestId("signing-status")).toHaveTextContent("Signed · 3 of 3")
        // The panel follows the request until its action ran, without reopening.
        await waitFor(
            () => expect(view.getByTestId("signing-status")).toHaveTextContent("Done · 3 of 3"),
            {timeout: 5000}
        )
        await expect(args.onChange).toHaveBeenCalledTimes(1)
    },
}

export const Cancelled: Story = {
    args: {
        viewer: JOSE,
        panel: {
            status: SigningRequestStatus.Cancelled,
            signed: [MARIA],
            cancelReason: CancelReason.PayloadChanged,
        },
    },
    play: async () => {
        const view = await panel()
        await expect(view.getByTestId("signing-status")).toHaveTextContent(/^Cancelled$/)
        await expect(view.getByText("What it signs changed")).toBeVisible()
        await expect(
            view.getByText(
                "This request was cancelled: What it signs changed. Signatures given for it no longer count. Start it again to sign the current version."
            )
        ).toBeVisible()
        await expect(view.queryByRole("button", {name: "Sign"})).toBeNull()
        await expect(view.queryByRole("progressbar")).toBeNull()
    },
}

export const Expired: Story = {
    args: {viewer: JOSE, panel: {status: SigningRequestStatus.Expired, signed: [MARIA]}},
    play: async () => {
        const view = await panel()
        await expect(view.getByTestId("signing-status")).toHaveTextContent("Expired · 1 of 3")
        await expect(
            view.getByText(
                "This request expired. Signatures given for it no longer count. Start it again to sign."
            )
        ).toBeVisible()
        await expect(view.queryByRole("button", {name: "Sign"})).toBeNull()
        await expect(view.queryByRole("button", {name: "Next member signs in"})).toBeNull()
    },
}

export const CancelByRequester: Story = {
    parameters: {widgets: ["StatusLine", "SignerList", "CancelRequestDialog"]},
    play: async ({args}) => {
        const view = await panel()
        await userEvent.click(view.getByRole("button", {name: "Cancel request"}))
        const confirm = within(
            await within(document.body).findByRole("dialog", {name: "Cancel this request?"})
        )
        await userEvent.type(confirm.getByLabelText("Reason (optional)"), "Wrong template")
        await userEvent.click(confirm.getByRole("button", {name: "Cancel request"}))
        await waitFor(() =>
            expect(view.getByTestId("signing-status")).toHaveTextContent(/^Cancelled$/)
        )
        await expect(api.cancel).toHaveBeenCalledWith("55555555-5555-4555-8555-555555555555", {
            reason: "Wrong template",
        })
        await expect(args.onChange).toHaveBeenCalledTimes(1)
    },
}

export const LoadFailure: Story = {
    args: {loadFails: true},
    play: async () => {
        const body = within(document.body)
        const alert = await body.findByText("The request could not be loaded.")
        await expect(alert).toBeVisible()
        await userEvent.click(body.getByRole("button", {name: "Try again"}))
        await panel()
        await expect(api.getRequest).toHaveBeenCalledTimes(2)
    },
}
