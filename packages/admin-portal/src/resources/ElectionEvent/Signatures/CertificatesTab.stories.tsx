// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {i18n} from "@sequentech/ui-core"
import {EVENT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {
    CertificatePostBinding,
    CertificateRegistration,
    CrlUnavailablePolicy,
    RevocationCheck,
    type IStaffCertificate,
} from "@/lib/signing/types"
import {CertificatesTab} from "./CertificatesTab"
import {personName, signaturesAccess} from "./signingSettings"
import {
    Organization,
    SIGNING_ROLES,
    SignaturesStory,
    applyOverrides,
    councilText,
    organizationOf,
    signingHandlers,
    signingRecords,
    type SigningRole,
} from "./__stories__/SignaturesFixture"

interface Scenario {
    organization: Organization
    role: SigningRole
}

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof signingRecords>

const meta = {
    title: "Admin/Election event/Signatures/CertificatesTab",
    component: CertificatesTab,
    args: {organization: Organization.Overseas, role: "securityOfficer"},
    argTypes: {
        organization: {control: "inline-radio", options: Object.values(Organization)},
        role: {control: "select", options: Object.keys(SIGNING_ROLES)},
    },
    parameters: {
        widgets: [
            "Card",
            "TrustedIssuersCard",
            "ChecksCard",
            "RegisteredCertificatesCard",
            "PersonDetail",
        ],
    },
    beforeEach: ({args}) => {
        const organization = organizationOf(args.organization)
        graphql = graphqlBoundary(signingHandlers(organization))
        data = signingRecords(organization)
        return applyOverrides(organization)
    },
    render: ({role}) => {
        const roles = new Set<string>(SIGNING_ROLES[role])
        return (
            <SignaturesStory boundary={graphql} data={data} role={role}>
                <CertificatesTab
                    electionEventId={EVENT_ID}
                    access={signaturesAccess((permission) => roles.has(permission))}
                />
            </SignaturesStory>
        )
    },
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const label = (key: string, options?: Record<string, unknown>) =>
    i18n.t(`signing.certificates.${key}`, options)

const PEM = "-----BEGIN CERTIFICATE-----\nc3ludGhldGljIGlzc3Vlcg==\n-----END CERTIFICATE-----\n"
const pemFile = () => new File([PEM], "issuer.pem", {type: "application/x-pem-file"})

const overseas = () => organizationOf(Organization.Overseas)
const displayName = (certificate: IStaffCertificate) =>
    personName(certificate.user_display_name, certificate.username)

/** The row of the registered certificate of this username. */
async function certificateRow(canvasElement: HTMLElement, username: string) {
    const cell = await within(canvasElement).findByText(username)
    return within(cell.closest("tr") as HTMLElement)
}

/** The usernames the registered certificates table lists, in order. */
const registeredUsernames = (canvasElement: HTMLElement) => {
    const table = within(canvasElement).getByRole("table", {name: label("registeredTitle")})
    const usernames = new Set(overseas().certificates.map(({username}) => username))
    return within(table)
        .getAllByRole("row")
        .slice(1)
        .map(
            (row) =>
                Array.from(row.querySelectorAll("td *")).find((element) =>
                    usernames.has(element.textContent ?? "")
                )?.textContent
        )
}

const mutation = (name: string) => graphql.calls.filter((call) => call.name === name)

async function dialog(name: string) {
    return within(await within(document.body).findByRole("dialog", {name}))
}

/** Waits for a notification, which fades in. */
async function notified(text: string) {
    const message = await within(document.body).findByText(text)
    await waitFor(() => expect(message).toBeVisible())
}

/** The titles of the cards marked Read only. */
async function readOnlyCards(canvasElement: HTMLElement) {
    await within(canvasElement).findByRole("table", {name: label("issuers")})
    const chip = i18n.t("signing.readOnly.chip")
    return within(canvasElement)
        .getAllByRole("button", {expanded: true})
        .filter((summary) => within(summary).queryByText(chip))
        .map((summary) => summary.textContent?.replace(chip, ""))
}

/** Which controls a role gets: each has its own permission. */
async function controls(canvasElement: HTMLElement) {
    const canvas = within(canvasElement)
    await certificateRow(canvasElement, overseas().certificates[0].username)
    return {
        importIssuer: !!canvas.queryByRole("button", {name: label("import")}),
        removeIssuer: canvas.queryAllByRole("button", {name: /^Remove /}).length > 0,
        checks: !(canvas.getByLabelText(label("onePost")) as HTMLInputElement).disabled,
        register: !!canvas.queryByRole("button", {name: label("register")}),
        revoke: canvas.queryAllByRole("button", {name: /^Revoke /}).length > 0,
    }
}

export const SecurityOfficer: Story = {
    parameters: {
        widgets: [
            "Card",
            "TrustedIssuersCard",
            "ChecksCard",
            "RegisteredCertificatesCard",
            "PersonDetail",
        ],
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const organization = overseas()
        const issuers = await canvas.findByRole("table", {name: label("issuers")})
        // Issuer, type and issued by of each issuer the event trusts.
        expect(
            within(issuers)
                .getAllByRole("row")
                .slice(1)
                .map((row) =>
                    within(row)
                        .getAllByRole("cell")
                        .slice(0, 3)
                        .map((cell) => cell.textContent)
                )
        ).toEqual(
            organization.issuers.map((issuer) => [
                issuer.common_name,
                label(issuer.subject === issuer.issuer ? "root" : "intermediate"),
                issuer.issuer_common_name,
            ])
        )
        await expect(canvas.getByLabelText(label("checkRevocation"))).toBeChecked()
        await expect(canvas.getByLabelText(label("onePost"))).toBeChecked()
        await expect(canvas.getByLabelText(label("registration.on-first-use"))).toBeChecked()
        // Each revocation list with its last download, in the event's zone with its name
        // Asia/Manila: 10:00 UTC is 6:00 PM PhST in the canonical admin format.
        for (const crl of organization.crls) {
            await expect(canvas.getByText(new RegExp(`^${crl.url}: `))).toBeVisible()
        }
        await expect(
            canvas.getByText(
                label("crlUpdated", {
                    url: organization.crls[0].url,
                    time: "May 8, 2028, 6:00 PM PhST",
                })
            )
        ).toBeVisible()
        expect(await readOnlyCards(canvasElement)).toEqual([])

        // People by name with their username; certificates expiring within 30 days stand out.
        const [maria, , officer, revoked, soon] = organization.certificates
        const mariaRow = await certificateRow(canvasElement, maria.username)
        await expect(mariaRow.getByText(displayName(maria))).toBeVisible()
        // "username · title or role" (draft Settings 4), the title as the signing panel shows it.
        await expect(mariaRow.getByText(`· ${organization.titles[maria.user_id]}`)).toBeVisible()
        await expect(mariaRow.getByText(organization.posts[0].name)).toBeVisible()
        // Dates as the signing panel writes them, in the event's zone.
        await expect(mariaRow.getByText("Jan 11, 2030, 8:00 AM PhST")).toBeVisible()
        await expect(
            (await certificateRow(canvasElement, soon.username)).getByText(
                label("statuses.expires-soon")
            )
        ).toBeVisible()
        const revokedRow = await certificateRow(canvasElement, revoked.username)
        await expect(revokedRow.getByText(/^Revoked /)).toBeVisible()
        expect(revokedRow.queryByRole("button")).toBeNull()
        const officerRow = await certificateRow(canvasElement, officer.username)
        await expect(officerRow.getByText(label("allPosts"))).toBeVisible()
        await expect(
            officerRow.getByText(label("registeredBy", {name: officer.registered_by_name}))
        ).toBeVisible()
        expect(mutation("GetSigningCertificates")[0]).toMatchObject({
            headers: {"x-hasura-role": "signing-certificates-read"},
        })
        // The zone and the titles come with the reader's own permission.
        expect(mutation("SigningEventInfo")[0]).toMatchObject({
            variables: {electionEventId: EVENT_ID},
            headers: {"x-hasura-role": "signing-certificates-read"},
        })
    },
}

export const SearchAndFilter: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const [maria, , , revoked, soon] = overseas().certificates
        await certificateRow(canvasElement, maria.username)
        // A display name finds its person.
        await userEvent.type(
            canvas.getByRole("textbox", {name: label("search")}),
            displayName(revoked)
        )
        await waitFor(() => expect(registeredUsernames(canvasElement)).toEqual([revoked.username]))
        await userEvent.clear(canvas.getByRole("textbox", {name: label("search")}))
        await userEvent.click(canvas.getByRole("combobox", {name: label("status")}))
        await userEvent.click(
            await within(document.body).findByRole("option", {
                name: label("statuses.expires-soon"),
            })
        )
        await waitFor(() => expect(registeredUsernames(canvasElement)).toEqual([soon.username]))
    },
}

export const ImportAndRemoveIssuers: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await userEvent.click(await canvas.findByRole("button", {name: label("import")}))
        const importing = await dialog(label("import"))
        await userEvent.upload(importing.getByLabelText(label("chooseFile")), pemFile())
        await expect(await importing.findByText("issuer.pem")).toBeVisible()
        await userEvent.click(importing.getByRole("button", {name: i18n.t("common.label.import")}))
        await waitFor(() =>
            expect(mutation("SigningImportIssuers").map(({variables}) => variables)).toEqual([
                {election_event_id: EVENT_ID, pem: PEM, der_base64: null},
            ])
        )
        await notified(label("imported", {imported: 1, skipped: 0}))
        await waitFor(() => expect(within(document.body).queryByRole("dialog")).toBeNull())

        const root = overseas().issuers[0]
        await userEvent.click(
            canvas.getByRole("button", {name: label("deleteIssuer", {name: root.common_name})})
        )
        const confirm = await dialog(i18n.t("common.label.warning"))
        await userEvent.click(confirm.getByRole("button", {name: i18n.t("common.label.delete")}))
        await waitFor(() =>
            expect(mutation("SigningDeleteIssuer").map(({variables}) => variables)).toEqual([
                {election_event_id: EVENT_ID, issuer_id: root.id},
            ])
        )
    },
}

export const ImportADerIssuer: Story = {
    play: async ({canvasElement}) => {
        await userEvent.click(
            await within(canvasElement).findByRole("button", {name: label("import")})
        )
        const importing = await dialog(label("import"))
        // A binary .cer goes as its bytes, in base64.
        const der = new File([Uint8Array.from([0x30, 0x82, 0x01])], "issuer.cer")
        await userEvent.upload(importing.getByLabelText(label("chooseFile")), der)
        await expect(await importing.findByText("issuer.cer")).toBeVisible()
        await userEvent.click(importing.getByRole("button", {name: i18n.t("common.label.import")}))
        await waitFor(() =>
            expect(mutation("SigningImportIssuers").map(({variables}) => variables)).toEqual([
                {election_event_id: EVENT_ID, pem: null, der_base64: "MIIB"},
            ])
        )
    },
}

export const ChangeTheChecks: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await userEvent.click(await canvas.findByLabelText(label("onePost")))
        await waitFor(() =>
            expect(mutation("SigningPutChecks").map(({variables}) => variables)).toEqual([
                {
                    election_event_id: EVENT_ID,
                    revocation_check: RevocationCheck.Check,
                    crl_unavailable: CrlUnavailablePolicy.Refuse,
                    registration: CertificateRegistration.OnFirstUse,
                    post_binding: CertificatePostBinding.AnyPost,
                    expected_revision: overseas().checks.revision,
                },
            ])
        )
        await notified(label("checksSaved"))
    },
}

export const RevokeACertificate: Story = {
    play: async ({canvasElement}) => {
        const [maria] = overseas().certificates
        const row = await certificateRow(canvasElement, maria.username)
        await userEvent.click(
            row.getByRole("button", {name: label("revokeOf", {name: displayName(maria)})})
        )
        const revoke = await dialog(label("revokeTitle", {name: displayName(maria)}))
        const confirm = revoke.getByRole("button", {name: label("revoke")})
        await expect(confirm).toBeDisabled()
        await userEvent.type(revoke.getByRole("textbox", {name: label("revokeReason")}), "Lost")
        await userEvent.click(confirm)
        await waitFor(() =>
            expect(mutation("SigningRevokeCertificate").map(({variables}) => variables)).toEqual([
                {election_event_id: EVENT_ID, certificate_id: maria.id, reason: "Lost"},
            ])
        )
    },
}

export const Auditor: Story = {
    args: {role: "auditor"},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        expect(await readOnlyCards(canvasElement)).toEqual([
            label("issuers"),
            label("checks"),
            label("registeredTitle"),
        ])
        expect(await controls(canvasElement)).toEqual({
            importIssuer: false,
            removeIssuer: false,
            checks: false,
            register: false,
            revoke: false,
        })
        await expect(canvas.getByLabelText(label("registration.on-first-use"))).toBeDisabled()
        // Reads only: the certificates, and the event's zone and the titles.
        expect(new Set(graphql.calls.map(({name}) => name))).toEqual(
            new Set(["GetSigningCertificates", "SigningEventInfo"])
        )
    },
}

// One write permission each: swapping any two gates fails one of these.

export const IssuersOnly: Story = {
    args: {role: "issuersOnly"},
    play: async ({canvasElement}) => {
        expect(await controls(canvasElement)).toEqual({
            importIssuer: true,
            removeIssuer: true,
            checks: false,
            register: false,
            revoke: false,
        })
        expect(await readOnlyCards(canvasElement)).toEqual([
            label("checks"),
            label("registeredTitle"),
        ])
    },
}

export const ChecksOnly: Story = {
    args: {role: "checksOnly"},
    play: async ({canvasElement}) => {
        expect(await controls(canvasElement)).toEqual({
            importIssuer: false,
            removeIssuer: false,
            checks: true,
            register: false,
            revoke: false,
        })
        expect(await readOnlyCards(canvasElement)).toEqual([
            label("issuers"),
            label("registeredTitle"),
        ])
    },
}

export const RegisterOnly: Story = {
    args: {role: "registerOnly"},
    play: async ({canvasElement}) => {
        expect(await controls(canvasElement)).toEqual({
            importIssuer: false,
            removeIssuer: false,
            checks: false,
            register: true,
            revoke: false,
        })
        expect(await readOnlyCards(canvasElement)).toEqual([label("issuers"), label("checks")])
        await userEvent.click(within(canvasElement).getByRole("button", {name: label("register")}))
        await dialog(label("register"))
    },
}

export const RevokeOnly: Story = {
    args: {role: "revokeOnly"},
    play: async ({canvasElement}) => {
        expect(await controls(canvasElement)).toEqual({
            importIssuer: false,
            removeIssuer: false,
            checks: false,
            register: false,
            revoke: true,
        })
        expect(await readOnlyCards(canvasElement)).toEqual([label("issuers"), label("checks")])
    },
}

export const SecondOrganization: Story = {
    args: {organization: Organization.StudentCouncil},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const council = organizationOf(Organization.StudentCouncil)
        const issuers = await canvas.findByRole("table", {name: label("issuers")})
        await expect(
            within(issuers).getAllByText(council.issuers[0].common_name as string)[0]
        ).toBeVisible()
        // The tenant's term for a Post names the column, from its one override.
        await expect(
            canvas.getByRole("columnheader", {name: councilText("signing.terms.post")})
        ).toBeVisible()
        const [certificate] = council.certificates
        const row = await certificateRow(canvasElement, certificate.username)
        await expect(
            row.getByText(
                council.posts.find(({id}) => id === certificate.election_id)?.name as string
            )
        ).toBeVisible()
        await expect(row.getByText(`· ${council.titles[certificate.user_id]}`)).toBeVisible()
        // Its own checks.
        const switchState = (text: string) =>
            (canvas.getByLabelText(text) as HTMLInputElement).checked
        expect(switchState(label("checkRevocation"))).toBe(
            council.checks.revocation_check === RevocationCheck.Check
        )
        expect(switchState(label("onePost"))).toBe(
            council.checks.post_binding === CertificatePostBinding.OnePost
        )
        await expect(
            canvas.getByLabelText(label(`registration.${council.checks.registration}`))
        ).toBeChecked()
    },
}
