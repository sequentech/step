// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import type {Operation} from "@apollo/client"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {i18n} from "@sequentech/ui-core"
import {EVENT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {CertificateCheckId} from "@/lib/signing/types"
import {RegisterCertificateDialog} from "./RegisterCertificateDialog"
import {certificateFingerprint, personName} from "./signingSettings"
import {
    Organization,
    SignaturesStory,
    organizationOf,
    refusal,
    signingHandlers,
    signingRecords,
    type ISigningOrganization,
} from "./__stories__/SignaturesFixture"

interface Scenario {
    /** How the server answers a registration without `linked_to`. */
    answer:
        | "registered"
        | "registered-to-other"
        | "registered-to-other-named"
        | "untrusted"
        | "forbidden"
        | "conflict"
    onClose: () => void
    onRegistered: () => void
}

const PEM = "-----BEGIN CERTIFICATE-----\nc3ludGhldGljIHN0YWZm\n-----END CERTIFICATE-----\n"
const pemFile = () => new File([PEM], "staff.pem", {type: "application/x-pem-file"})

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof signingRecords>
let organization: ISigningOrganization

/** An account the refusal names, which the event's registrations don't know. */
const HOLDER = {userId: "holder-account", name: "Holder Of The Key"}

const meta = {
    title: "Admin/Election event/Signatures/RegisterCertificateDialog",
    component: RegisterCertificateDialog,
    args: {answer: "registered", onClose: fn(), onRegistered: fn()},
    argTypes: {
        answer: {
            control: "inline-radio",
            options: [
                "registered",
                "registered-to-other",
                "registered-to-other-named",
                "untrusted",
                "forbidden",
                "conflict",
            ],
        },
    },
    parameters: {widgets: ["FileButton"]},
    beforeEach: async ({args}) => {
        organization = organizationOf(Organization.Overseas)
        // The certificate the story uploads is Jose's, registered to his first account.
        organization.certificates[1].fingerprint_sha256 = (await certificateFingerprint(
            PEM
        )) as string
        const refused = {
            "registered": null,
            "registered-to-other": refusal({
                code: "signing-refused",
                check: CertificateCheckId.RegisteredToOther,
            }),
            // The contract's refusal names the holder's account.
            "registered-to-other-named": refusal({
                code: "signing-refused",
                check: CertificateCheckId.RegisteredToOther,
                user_id: HOLDER.userId,
                display_name: HOLDER.name,
            }),
            "untrusted": refusal({
                code: "signing-refused",
                check: CertificateCheckId.TrustedIssuer,
            }),
            "forbidden": refusal({code: "forbidden"}),
            "conflict": refusal({code: "conflict"}),
        }[args.answer]
        graphql = graphqlBoundary({
            ...signingHandlers(organization),
            SigningRegisterCertificate: ({variables}: Operation) =>
                refused && !variables.linked_to
                    ? refused
                    : {data: {signingRegisterCertificate: {certificate_id: "new"}}},
        })
        data = signingRecords(organization)
    },
    render: ({onClose, onRegistered}) => (
        <SignaturesStory boundary={graphql} data={data} role="securityOfficer">
            <RegisterCertificateDialog
                open
                electionEventId={EVENT_ID}
                certificates={organization.certificates}
                onClose={onClose}
                onRegistered={onRegistered}
            />
        </SignaturesStory>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const label = (key: string, options?: Record<string, unknown>) =>
    i18n.t(`signing.certificates.${key}`, options)

const registrations = () =>
    graphql.calls
        .filter(({name}) => name === "SigningRegisterCertificate")
        .map(({variables}) => variables)

async function notified(text: string) {
    const message = await within(document.body).findByText(text)
    await waitFor(() => expect(message).toBeVisible())
}

/** Finds the person by username, picks a Post and the certificate file, and registers. */
async function register(username: string, post: string | null) {
    const dialog = within(await within(document.body).findByRole("dialog"))
    const submit = dialog.getByRole("button", {name: label("registerSubmit")})
    await expect(submit).toBeDisabled()
    await userEvent.type(dialog.getByRole("combobox", {name: label("person")}), username)
    await userEvent.click(
        await within(document.body).findByRole("option", {name: new RegExp(`\\(${username}\\)$`)})
    )
    if (post) {
        await userEvent.click(dialog.getByRole("combobox", {name: label("columns.post")}))
        await userEvent.click(await within(document.body).findByRole("option", {name: post}))
    }
    await userEvent.upload(dialog.getByLabelText(label("chooseFile")), pemFile())
    await waitFor(() => expect(dialog.getByRole("textbox", {name: label("pem")})).toHaveValue(PEM))
    await userEvent.click(submit)
    return dialog
}

export const RegisterToAPerson: Story = {
    parameters: {widgets: ["FileButton"]},
    play: async ({args}) => {
        const [person] = organization.people
        const post = organization.posts[2]
        await register(person.username, post.name)
        await waitFor(() => expect(args.onRegistered).toHaveBeenCalledOnce())
        expect(registrations()).toEqual([
            {
                election_event_id: EVENT_ID,
                user_id: person.id,
                election_id: post.id,
                pem: PEM,
                linked_to: null,
            },
        ])
        await notified(label("registerDone"))
    },
}

export const LinkASecondAccount: Story = {
    args: {answer: "registered-to-other"},
    play: async ({args}) => {
        const jose = organization.certificates[1]
        const second = organization.people[1]
        const dialog = await register(second.username, null)
        // The refusal names who holds it, and offers the link (decided O1).
        const name = personName(jose.user_display_name, jose.username)
        await expect(await dialog.findByText(label("registeredToOther", {name}))).toBeVisible()
        expect(args.onRegistered).not.toHaveBeenCalled()
        await userEvent.click(dialog.getByRole("button", {name: label("linkAccount")}))
        await waitFor(() => expect(args.onRegistered).toHaveBeenCalledOnce())
        expect(registrations().map(({user_id, linked_to}) => ({user_id, linked_to}))).toEqual([
            {user_id: second.id, linked_to: null},
            {user_id: second.id, linked_to: jose.user_id},
        ])
    },
}

export const LinkTheAccountTheRefusalNames: Story = {
    args: {answer: "registered-to-other-named"},
    play: async ({args}) => {
        const second = organization.people[1]
        const dialog = await register(second.username, null)
        await expect(
            await dialog.findByText(label("registeredToOther", {name: HOLDER.name}))
        ).toBeVisible()
        await userEvent.click(dialog.getByRole("button", {name: label("linkAccount")}))
        await waitFor(() => expect(args.onRegistered).toHaveBeenCalledOnce())
        expect(registrations().at(-1)).toMatchObject({linked_to: HOLDER.userId})
    },
}

export const RefusedCertificate: Story = {
    args: {answer: "untrusted"},
    play: async ({args}) => {
        await register(organization.people[0].username, null)
        await notified(label("registerRefused"))
        expect(args.onRegistered).not.toHaveBeenCalled()
        expect(within(document.body).queryByRole("button", {name: label("linkAccount")})).toBeNull()
    },
}

export const RefusedPermission: Story = {
    args: {answer: "forbidden"},
    play: async ({args}) => {
        await register(organization.people[0].username, null)
        await notified(i18n.t("signing.errors.forbidden"))
        expect(args.onRegistered).not.toHaveBeenCalled()
    },
}

export const AlreadyRegistered: Story = {
    args: {answer: "conflict"},
    play: async ({args}) => {
        await register(organization.people[0].username, null)
        await notified(label("alreadyRegistered"))
        expect(args.onRegistered).not.toHaveBeenCalled()
    },
}
