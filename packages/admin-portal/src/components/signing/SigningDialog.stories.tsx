// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {openedWindows} from "@/__stories__/storyNetwork"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {applyTenantTranslationOverrides} from "@/providers/TenantContextProvider"
import type {Sequent_Backend_Tenant} from "@/gql/graphql"
import type {ISigningApi, ISigningPanelData} from "@/lib/signing/api"
import {SigningApiError, SigningApiErrorKind} from "@/lib/signing/api"
import {
    CancelReason,
    CertificateCheckId,
    SigningAction,
    SigningRequestStatus,
    type ICertificateCheckResult,
} from "@/lib/signing/types"
import {SigningDialog} from "./SigningDialog"
import {shortHash} from "./format"
import {
    ANA,
    CERTIFICATE_FILES,
    CODE,
    COUNTRY,
    JOSE,
    MARIA,
    ORGANIZATIONS,
    PASSWORD,
    POST,
    SIGN_ROLES,
    everythingSent,
    fakeApi,
    makePanel,
    passingChecks,
    signedInAs,
    withFailure,
    type IPanelOptions,
    type IStoryPerson,
} from "./__stories__/fixtures"

interface Scenario {
    viewer: IStoryPerson
    organization: Sequent_Backend_Tenant
    panel: IPanelOptions
    /** What the server's dry run answers. */
    checks: ICertificateCheckResult[]
    onClose: () => void
}

let boundary: ReturnType<typeof graphqlBoundary>
let data: ISigningPanelData
let api: ReturnType<typeof fakeApi>

/** Opens Maria's certificate and waits until Sign may be pressed. */
async function readyToSign() {
    const view = await openCertificate(CERTIFICATE_FILES.maria())
    const sign = await view.findByRole("button", {name: "Sign"}, {timeout: 10000})
    await waitFor(() => expect(sign).toBeEnabled())
    return {view, sign}
}

const LOCAL_NOTE =
    /Signing happens in this browser\. Your certificate file, its private key and its password are never sent\./

const meta = {
    title: "Admin/Signing/SigningDialog",
    component: SigningDialog,
    args: {
        viewer: MARIA,
        organization: ORGANIZATIONS.first,
        panel: {},
        checks: passingChecks(),
        onClose: fn(),
    },
    beforeEach: async ({args}) => {
        boundary = graphqlBoundary({})
        data = await makePanel(args.panel)
        api = fakeApi(data, {checks: args.checks})
    },
    render: ({viewer, organization, onClose}) => (
        <AdminStoryProvider
            boundary={boundary}
            roles={SIGN_ROLES}
            auth={signedInAs(viewer)}
            tenantRecord={organization}
        >
            <SigningDialog open data={data} api={api as ISigningApi} onClose={onClose} />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const dialog = async () => {
    const found = await within(document.body).findByRole("dialog", {
        name: /^Sign the /,
    })
    await waitFor(() => expect(found).toBeVisible())
    return within(found)
}

/** From the check step to an opened certificate. */
async function openCertificate(file: File, password = PASSWORD) {
    const view = await dialog()
    const confirm = view.queryByRole("checkbox")
    if (confirm) await userEvent.click(confirm)
    const continueButton = view.getByRole("button", {name: "Continue"})
    // Enabled once the payload is checked against what is shown.
    await waitFor(() => expect(continueButton).toBeEnabled())
    await userEvent.click(continueButton)
    await userEvent.upload(view.getByLabelText("Certificate file"), file)
    await expect(view.getByText(file.name)).toBeVisible()
    await userEvent.type(view.getByLabelText("Certificate password"), password)
    await userEvent.click(view.getByRole("button", {name: "Open certificate"}))
    return view
}

const checkLine = (view: ReturnType<typeof within>, id: CertificateCheckId) =>
    view.getByRole("list", {name: "Certificate checks"}).querySelector(`[data-check="${id}"]`)

export const CheckWithDocument: Story = {
    parameters: {widgets: ["CheckStep", "LocalSigningNote"]},
    play: async ({args}) => {
        const view = await dialog()
        await expect(view.getByRole("heading", {name: "Sign the election returns"})).toBeVisible()
        await expect(
            view.getByText(`You are signing as ${args.viewer.name} · ${args.viewer.title}, ${POST}`)
        ).toBeVisible()
        await expect(view.getByText(`Election returns · ${POST} · ${COUNTRY}`)).toBeVisible()
        await expect(view.getByText(`Election returns, ${POST}, ${COUNTRY}.pdf`)).toBeVisible()
        const hash = data.request.document_sha256!
        await expect(view.getByText(`PDF · 3 pages · SHA-256 ${shortHash(hash)}`)).toHaveAttribute(
            "title",
            hash
        )
        // The document opens from the verified bytes, never from the server's URL.
        await userEvent.click(view.getByRole("button", {name: "Open the document"}))
        await waitFor(() => expect(openedWindows()).toHaveLength(1))
        await expect(openedWindows()[0]).toMatch(/^blob:/)
        await expect(api.fetchDocument).toHaveBeenCalledWith(data.document_url)
        await expect(view.getByTestId("signing-code")).toHaveTextContent(CODE)
        await expect(view.getByText("Everyone who signs sees the same code.")).toBeVisible()
        await expect(view.getByText(LOCAL_NOTE)).toBeVisible()

        const continueButton = view.getByRole("button", {name: "Continue"})
        await expect(continueButton).toBeDisabled()
        await userEvent.click(
            view.getByRole("checkbox", {name: "I have checked the election returns"})
        )
        await waitFor(() => expect(continueButton).toBeEnabled())
        await userEvent.click(view.getByRole("button", {name: "Cancel"}))
        await expect(args.onClose).toHaveBeenCalledTimes(1)
    },
}

export const CheckWithDetails: Story = {
    args: {
        viewer: JOSE,
        panel: {
            action: SigningAction.ApproveVoter,
            required: 1,
            subject: {
                application_id: "APP-2028-118204",
                applicant_registry_id: "R-9",
                decision: "approve",
            },
            details: [
                {key: "application_id", label: "Application", value: "APP-2028-118204"},
                {key: "applicant_registry_id", label: "Registry record", value: "R-9"},
            ],
        },
    },
    parameters: {widgets: ["CheckStep", "LocalSigningNote"]},
    play: async () => {
        const view = await dialog()
        await expect(view.getByRole("heading", {name: "Sign the voter approval"})).toBeVisible()
        const table = view.getByRole("table", {name: "Details"})
        // The rows are the payload's subject, labelled by the server.
        await expect(within(table).getByText("APP-2028-118204")).toBeVisible()
        await expect(within(table).getByText("Application")).toBeVisible()
        await expect(within(table).getByText("Registry record")).toBeVisible()
        await expect(within(table).getByText("decision")).toBeVisible()
        // Nothing to read through: no "I have checked" box.
        await expect(view.queryByRole("checkbox")).toBeNull()
        await waitFor(() => expect(view.getByRole("button", {name: "Continue"})).toBeEnabled())
        await expect(view.getByText(LOCAL_NOTE)).toBeVisible()
    },
}

export const CertificateChosen: Story = {
    parameters: {widgets: ["CheckStep", "CertificateStep", "LocalSigningNote"]},
    play: async () => {
        const view = await dialog()
        await userEvent.click(view.getByRole("checkbox"))
        const continueButton = view.getByRole("button", {name: "Continue"})
        await waitFor(() => expect(continueButton).toBeEnabled())
        await userEvent.click(continueButton)
        await expect(
            view.getByText("Insert your security token and choose your certificate file.")
        ).toBeVisible()
        await expect(view.getByRole("button", {name: "Open certificate"})).toBeDisabled()

        await userEvent.upload(view.getByLabelText("Certificate file"), CERTIFICATE_FILES.maria())
        await expect(view.getByText("maria-santos.p12")).toBeVisible()
        const password = view.getByLabelText("Certificate password")
        await userEvent.type(password, PASSWORD)
        // Masked, but not a password field the browser would offer to save.
        await expect(password).toHaveAttribute("type", "text")
        await expect(password).toHaveAttribute("autocomplete", "one-time-code")
        await expect(password.closest("form")).toBeNull()
        const masking = () => getComputedStyle(password).getPropertyValue("-webkit-text-security")
        await expect(masking()).toBe("disc")
        await userEvent.click(view.getByRole("button", {name: "Show password"}))
        await expect(masking()).toBe("none")
        await expect(
            view.getByText(
                "The file and its password stay on this computer. Only your signature and the public certificate are sent."
            )
        ).toBeVisible()
        await expect(view.getByText(LOCAL_NOTE)).toBeVisible()
        await userEvent.click(view.getByRole("button", {name: "Back"}))
        await expect(view.getByRole("checkbox")).toBeChecked()
    },
}

export const CertificateCheckedOk: Story = {
    parameters: {
        widgets: [
            "CheckStep",
            "CertificateStep",
            "LocalSigningNote",
            "CertificateCard",
            "CheckList",
        ],
    },
    play: async () => {
        const view = await openCertificate(CERTIFICATE_FILES.maria())
        const card = await view.findByTestId("signing-certificate", {}, {timeout: 10000})
        await expect(within(card).getByText(MARIA.certificate)).toBeVisible()
        await expect(
            within(card).getByText(
                /^Issued by Test PNPKI Individual CA · valid until Dec 31, 2029 · RSA$/
            )
        ).toBeVisible()
        await expect(within(card).getByText(/^SHA-256 [0-9A-F:]+…[0-9A-F:]+$/)).toBeVisible()
        await view.findByText("Issued by a trusted issuer (Test PNPKI Root CA)")
        await expect(view.getByText("Valid today")).toBeVisible()
        await expect(view.getByText("Made for signing")).toBeVisible()
        await expect(
            view.getByText(/^Not revoked \(lists updated ([01]\d|2[0-3]):[0-5]\d UTC\)$/)
        ).toBeVisible()
        await expect(view.getByText("Registered to you on Apr 8, 2028")).toBeVisible()
        await expect(view.getByRole("button", {name: "Sign"})).toBeEnabled()
        await expect(view.getByText(LOCAL_NOTE)).toBeVisible()

        // The server gets the public chain only: leaf, Individual CA, root.
        await expect(api.checkCertificate).toHaveBeenCalledTimes(1)
        const [requestId, input] = api.checkCertificate.mock.calls[0]
        await expect(requestId).toBe(data.request.id)
        await expect(Object.keys(input)).toEqual(["chain_pem"])
        await expect(input.chain_pem).toHaveLength(3)
        for (const pem of input.chain_pem) {
            await expect(pem).toMatch(
                /^-----BEGIN CERTIFICATE-----\n[A-Za-z0-9+/=\n]+-----END CERTIFICATE-----\n$/
            )
        }
        await expect(JSON.stringify(input)).not.toContain(PASSWORD)
        await expect(api.reportOpenFailure).not.toHaveBeenCalled()
        // The password is forgotten once the key is open.
        await expect(view.queryByLabelText("Certificate password")).toBeNull()
    },
}

export const FirstUse: Story = {
    args: {viewer: JOSE, panel: {signed: [MARIA]}, checks: passingChecks(null)},
    parameters: {
        widgets: [
            "CheckStep",
            "CertificateStep",
            "LocalSigningNote",
            "CertificateCard",
            "CheckList",
        ],
    },
    play: async () => {
        const view = await openCertificate(CERTIFICATE_FILES.jose())
        await view.findByText("First use: it will be registered to you", {}, {timeout: 10000})
        await expect(view.getByText(/· EC P-256$/)).toBeVisible()
        await expect(view.getByText(JOSE.certificate)).toBeVisible()
        await expect(view.getByRole("button", {name: "Sign"})).toBeEnabled()
    },
}

export const WrongPassword: Story = {
    parameters: {widgets: ["CheckStep", "CertificateStep", "LocalSigningNote"]},
    play: async () => {
        const view = await openCertificate(CERTIFICATE_FILES.maria(), "not-the-password")
        await view.findByText("Wrong password. Check it and try again.", {}, {timeout: 10000})
        await expect(view.getByLabelText("Certificate password")).toHaveAttribute(
            "aria-invalid",
            "true"
        )
        await expect(view.queryByRole("button", {name: "Sign"})).toBeNull()
        // The log learns the file name and the reason only.
        await expect(api.reportOpenFailure).toHaveBeenCalledWith(data.request.id, {
            file_name: "maria-santos.p12",
            reason: "wrong-password",
        })
        await expect(api.checkCertificate).not.toHaveBeenCalled()
        await expect(everythingSent(api)).not.toContain("not-the-password")
        await expect(view.getByText(LOCAL_NOTE)).toBeVisible()
    },
}

export const UnreadableFile: Story = {
    parameters: {widgets: ["CheckStep", "CertificateStep", "LocalSigningNote"]},
    play: async () => {
        const notACertificate = new File(["-----BEGIN CERTIFICATE-----\n"], "certificate.p12")
        const view = await openCertificate(notACertificate)
        await view.findByText(
            "This file is not a certificate file (.p12 or .pfx), or it is damaged.",
            {},
            {timeout: 10000}
        )
        await expect(api.reportOpenFailure).toHaveBeenCalledWith(data.request.id, {
            file_name: "certificate.p12",
            reason: "unreadable",
        })
        await expect(api.checkCertificate).not.toHaveBeenCalled()
    },
}

const untrusted = withFailure(passingChecks(null), {
    id: CertificateCheckId.TrustedIssuer,
    ok: false,
    detail: null,
})

/** The name the tenant configured: its display name, else its slug. */
const displayName = (organization: Sequent_Backend_Tenant): string =>
    (organization.settings as {display_name?: string}).display_name ?? organization.slug

const expectIssuerRefused = async (organization: Sequent_Backend_Tenant) => {
    const view = await openCertificate(CERTIFICATE_FILES.rosa())
    await view.findByText(
        "Example Commercial CA is not a trusted issuer for this election event",
        {},
        {timeout: 10000}
    )
    await expect(checkLine(view, CertificateCheckId.TrustedIssuer)).toHaveAttribute(
        "data-ok",
        "false"
    )
    await expect(
        view.getByText(
            `Use the certificate ${displayName(organization)} registered for you. Certificates from other issuers are not accepted.`
        )
    ).toBeVisible()
    await expect(view.getByRole("button", {name: "Sign"})).toBeDisabled()
}

export const UntrustedIssuer: Story = {
    args: {viewer: ANA, panel: {signed: [MARIA, JOSE]}, checks: untrusted},
    parameters: {
        widgets: [
            "CheckStep",
            "CertificateStep",
            "LocalSigningNote",
            "CertificateCard",
            "CheckList",
        ],
    },
    play: async ({args}) => {
        await expectIssuerRefused(args.organization)
        await expect(
            within(document.body).queryByText(new RegExp(args.organization.slug))
        ).toBeNull()
    },
}

/** The same refusal names the second organization: nothing is hard-coded. */
export const UntrustedIssuerSecondOrganization: Story = {
    args: {...UntrustedIssuer.args, organization: ORGANIZATIONS.second},
    parameters: {
        widgets: [
            "CheckStep",
            "CertificateStep",
            "LocalSigningNote",
            "CertificateCard",
            "CheckList",
        ],
    },
    play: async ({args}) => {
        await expect(displayName(args.organization)).not.toBe(displayName(ORGANIZATIONS.first))
        await expectIssuerRefused(args.organization)
        // The display name, not the slug.
        await expect(
            within(document.body).queryByText(new RegExp(args.organization.slug))
        ).toBeNull()
    },
}

/** Without a display name the slug names the organization. */
export const UntrustedIssuerSlugOnly: Story = {
    args: {...UntrustedIssuer.args, organization: ORGANIZATIONS.slugOnly},
    parameters: {
        widgets: [
            "CheckStep",
            "CertificateStep",
            "LocalSigningNote",
            "CertificateCard",
            "CheckList",
        ],
    },
    play: async ({args}) => {
        await expect(displayName(args.organization)).toBe("example-council")
        await expectIssuerRefused(args.organization)
    },
}

export const RegisteredToSomeoneElse: Story = {
    args: {
        viewer: JOSE,
        panel: {signed: [MARIA]},
        checks: withFailure(passingChecks(), {
            id: CertificateCheckId.RegisteredToOther,
            ok: false,
            detail: MARIA.name,
        }).filter((check) => check.id !== CertificateCheckId.Registered),
    },
    parameters: {
        widgets: [
            "CheckStep",
            "CertificateStep",
            "LocalSigningNote",
            "CertificateCard",
            "CheckList",
        ],
    },
    play: async () => {
        const view = await openCertificate(CERTIFICATE_FILES.maria())
        await view.findByText(`Registered to ${MARIA.name}`, {}, {timeout: 10000})
        await expect(
            view.getByText(
                "This certificate can't sign for you. Use the certificate on your own security token."
            )
        ).toBeVisible()
        await expect(view.getByRole("button", {name: "Sign"})).toBeDisabled()
        await expect(api.approve).not.toHaveBeenCalled()
    },
}

export const SignedOneOfThree: Story = {
    parameters: {
        widgets: [
            "CheckStep",
            "CertificateStep",
            "LocalSigningNote",
            "CertificateCard",
            "CheckList",
            "SignedStep",
        ],
    },
    play: async () => {
        const view = await openCertificate(CERTIFICATE_FILES.maria())
        const sign = await view.findByRole("button", {name: "Sign"}, {timeout: 10000})
        await waitFor(() => expect(sign).toBeEnabled())
        await userEvent.click(sign)
        await view.findByText("1 of 3 signatures.", {}, {timeout: 10000})
        await expect(view.getByText(`Next: ${JOSE.name} and ${ANA.name} sign.`)).toBeVisible()
        await expect(
            view.getByText(new RegExp(`with the certificate of ${MARIA.certificate}$`))
        ).toBeVisible()
        await expect(view.getByText(LOCAL_NOTE)).toBeVisible()

        // PDF mode: the prepared revision's digest is signed and sent with its revision.
        await expect(api.pdfPrepare).toHaveBeenCalledTimes(1)
        await expect(api.approve).toHaveBeenCalledTimes(1)
        const [requestId, approval] = api.approve.mock.calls[0]
        await expect(requestId).toBe(data.request.id)
        await expect(approval).toMatchObject({algorithm: "rsa-pkcs1-sha256", revision: 1})
        await expect(approval.chain_pem).toHaveLength(3)
        await expect(approval.payload_signature_b64).toMatch(/^[A-Za-z0-9+/]+={0,2}$/)
        await expect(approval.pdf_cms_b64).toMatch(/^[A-Za-z0-9+/]+={0,2}$/)
        // Checked again right before signing.
        await expect(api.getRequest).toHaveBeenCalledTimes(1)
        await expect(everythingSent(api)).not.toContain(PASSWORD)
        await userEvent.click(view.getByRole("button", {name: "Done"}))
    },
}

/** A tenant renames the action in its translations; the dialog follows. */
export const RenamedByTheOrganization: Story = {
    args: {organization: ORGANIZATIONS.second},
    parameters: {widgets: ["CheckStep", "LocalSigningNote"]},
    beforeEach: () => {
        applyTenantTranslationOverrides({
            i18n: {
                en: {
                    "adminPortal:signing.actions.generate-election-returns.short":
                        "Faculty results",
                    "adminPortal:signing.actions.generate-election-returns.object":
                        "faculty results",
                },
            },
        })
        return () => applyTenantTranslationOverrides({})
    },
    play: async () => {
        const view = await dialog()
        await expect(view.getByRole("heading", {name: "Sign the faculty results"})).toBeVisible()
        await expect(view.getByText(`Faculty results · ${POST} · ${COUNTRY}`)).toBeVisible()
        await expect(view.getByText("I have checked the faculty results")).toBeVisible()
        await expect(
            view.getByRole("checkbox", {name: "I have checked the faculty results"})
        ).not.toBeChecked()
    },
}

/** A details row the payload doesn't sign: the dialog refuses to go on. */
export const PayloadMismatchRefused: Story = {
    args: {
        viewer: JOSE,
        panel: {
            action: SigningAction.ApproveVoter,
            required: 1,
            subject: {application_id: "APP-2028-118204", decision: "approve"},
            details: [{key: "application_id", label: "Application", value: "APP-2028-999999"}],
        },
    },
    parameters: {widgets: ["CheckStep", "LocalSigningNote"]},
    play: async () => {
        const view = await dialog()
        await expect(await view.findByTestId("signing-mismatch")).toHaveTextContent(
            "What would be signed does not match this request."
        )
        await expect(view.queryByText("APP-2028-999999")).toBeNull()
        await expect(view.getByRole("button", {name: "Continue"})).toBeDisabled()
    },
}

/** The request was cancelled while the dialog was open: nothing is signed. */
export const CancelledWhileOpen: Story = {
    parameters: {
        widgets: [
            "CheckStep",
            "CertificateStep",
            "LocalSigningNote",
            "CertificateCard",
            "CheckList",
        ],
    },
    play: async () => {
        const {view, sign} = await readyToSign()
        api.getRequest.mockResolvedValueOnce(
            await makePanel({
                status: SigningRequestStatus.Cancelled,
                cancelReason: CancelReason.PayloadChanged,
            })
        )
        await userEvent.click(sign)
        await expect(await view.findByTestId("signing-closed")).toHaveTextContent(
            "This request was cancelled: What it signs changed."
        )
        await expect(view.getByRole("button", {name: "Sign"})).toBeDisabled()
        await expect(api.pdfPrepare).not.toHaveBeenCalled()
        await expect(api.approve).not.toHaveBeenCalled()
    },
}

/** A refusal that isn't a stale revision is not prepared again. */
export const AlreadySignedElsewhere: Story = {
    parameters: {
        widgets: [
            "CheckStep",
            "CertificateStep",
            "LocalSigningNote",
            "CertificateCard",
            "CheckList",
        ],
    },
    play: async () => {
        const {view, sign} = await readyToSign()
        api.approve.mockRejectedValueOnce(
            new SigningApiError(SigningApiErrorKind.AlreadySigned, "already signed", {status: 409})
        )
        await userEvent.click(sign)
        await expect(await view.findByTestId("signing-closed")).toHaveTextContent(
            "You have already signed this request."
        )
        await expect(api.pdfPrepare).toHaveBeenCalledTimes(1)
        await expect(view.getByRole("button", {name: "Sign"})).toBeDisabled()
    },
}

/** Two quick clicks send one approval. */
export const DoubleSubmit: Story = {
    parameters: {
        widgets: [
            "CheckStep",
            "CertificateStep",
            "LocalSigningNote",
            "CertificateCard",
            "CheckList",
            "SignedStep",
        ],
    },
    play: async () => {
        const {view, sign} = await readyToSign()
        await userEvent.dblClick(sign)
        await view.findByText("1 of 3 signatures.", {}, {timeout: 10000})
        await expect(api.approve).toHaveBeenCalledTimes(1)
    },
}

/** A token file with two equally good certificates: the person picks one. */
export const SeveralKeysInTheFile: Story = {
    parameters: {
        widgets: [
            "CheckStep",
            "CertificateStep",
            "LocalSigningNote",
            "CertificateCard",
            "CheckList",
        ],
    },
    play: async () => {
        const view = await openCertificate(CERTIFICATE_FILES.twoKeys())
        const choice = await view.findByLabelText("Certificate to sign with", {}, {timeout: 10000})
        await waitFor(() => expect(api.checkCertificate).toHaveBeenCalledTimes(1))
        const options = within(choice).getAllByRole("option")
        await expect(options).toHaveLength(2)
        const other = options.find((option) => !(option as HTMLOptionElement).selected)!
        await userEvent.selectOptions(choice, other)
        await waitFor(() => expect(api.checkCertificate).toHaveBeenCalledTimes(2))
        const [, first] = api.checkCertificate.mock.calls[0]
        const [, second] = api.checkCertificate.mock.calls[1]
        await expect(second.chain_pem[0]).not.toBe(first.chain_pem[0])
        await expect(
            within(view.getByTestId("signing-certificate")).getByText(
                other.textContent!.split(" · ")[0]
            )
        ).toBeVisible()
    },
}
