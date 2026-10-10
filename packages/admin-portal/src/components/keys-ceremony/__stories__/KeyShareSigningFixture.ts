// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The trustee's key step behind their own signature, for the stories of the
// keys ceremony check and the tally key restore: two organizations (names and
// tenant label overrides), the request the route answers and the fake
// signing service the signing panel talks to.
import {expect, userEvent, waitFor, within} from "storybook/test"
import {groupRoles, STORY_TRUSTEE} from "@/__stories__/storyAuth"
import {
    CERTIFICATE_FILES,
    CODE,
    EXPIRES_AT,
    JOSE,
    PASSWORD,
    REQUEST_ID,
    fakeApi,
    makePanel,
    signedInAs,
} from "@/components/signing/__stories__/fixtures"
import {hex} from "@/lib/signing/der"
import {SigningAction, SigningRequestStatus} from "@/lib/signing/types"
import {applyTenantTranslationOverrides} from "@/providers/TenantContextProvider"
import {IPermissions} from "@/types/keycloak"
import {EStoryPermissions} from "../../../../../ui-essentials/.storybook/globals"

export enum KeyShareRule {
    NotRequired = "not-required",
    Required = "required",
}

export enum KeyShareOrganization {
    Overseas = "overseas",
    StudentCouncil = "student-council",
}

export interface IKeyShareOrganization {
    ceremonyName: string
    trusteeName: string
    /** Tenant translation overrides (English), as stored in the tenant's settings. */
    overrides: Record<string, string>
}

export const KEY_SHARE_ORGANIZATIONS: Record<KeyShareOrganization, IKeyShareOrganization> = {
    [KeyShareOrganization.Overseas]: {
        ceremonyName: "Madrid PE keys",
        trusteeName: STORY_TRUSTEE,
        overrides: {},
    },
    [KeyShareOrganization.StudentCouncil]: {
        ceremonyName: "Student council keys",
        trusteeName: "returning-officer-1",
        overrides: {
            "adminPortal:signing.details.keys_ceremony_id": "Electoral commission ceremony",
            "adminPortal:signing.details.tally_session_id": "Count",
            "adminPortal:signing.details.trustee_id": "Returning officer",
            "adminPortal:signing.keyShare.signing":
                "Sign your share with your certificate to record it.",
        },
    },
}

/** The label the dialog shows for a subject field, from the organization's overrides. */
export const detailLabel = (organization: IKeyShareOrganization, key: string, fallback: string) =>
    organization.overrides[`adminPortal:signing.details.${key}`] ?? fallback

/** Applies the organization's overrides for one story; the returned cleanup removes them. */
export function applyKeyShareOverrides(organization: IKeyShareOrganization) {
    if (!Object.keys(organization.overrides).length) return undefined
    applyTenantTranslationOverrides({i18n: {en: organization.overrides}})
    return () => applyTenantTranslationOverrides(undefined)
}

/** Jose signs in as a trustee who may sign their key steps. */
export const TRUSTEE_ROLES = [
    ...groupRoles(EStoryPermissions.TRUSTEE),
    IPermissions.SIGN_KEY_CEREMONY,
    IPermissions.SIGN_TALLY_KEY,
]
/** Jose, signed in with the trustee account of the organization (a trustee's username is its name). */
export const trusteeAuth = (organization: IKeyShareOrganization) => ({
    ...signedInAs(JOSE),
    username: organization.trusteeName,
    trustee: organization.trusteeName,
})

export const TRUSTEE_ID = "88888888-8888-4888-8888-888888888888"

export const sha256 = async (text: string) =>
    hex(await crypto.subtle.digest("SHA-256", new TextEncoder().encode(text)))

/** What the route answers while the step waits for the trustee's signature. */
export const SIGNING_REQUEST = {id: REQUEST_ID, code: CODE, required: 1, expires_at: EXPIRES_AT}

/**
 * The trustee's request as the signing panel loads it: its subject is the
 * contract's (`target` = `keys_ceremony_id` or `tally_session_id`, the
 * trustee and the key share's SHA-256), the panel names the ceremony and
 * the trustee beside their ids, and Jose signs it.
 */
export async function keySharePanel(
    action: SigningAction,
    target: Record<string, string>,
    organization: IKeyShareOrganization,
    keyShare: string,
    status: SigningRequestStatus
) {
    const subject = {
        ...target,
        key_share_sha256: await sha256(keyShare),
        trustee_id: TRUSTEE_ID,
    }
    const done = status !== SigningRequestStatus.Waiting
    const panel = await makePanel({
        action,
        status,
        required: 1,
        requestedBy: JOSE,
        signed: done ? [JOSE] : [],
        subject,
        details: Object.keys(subject)
            .sort()
            .map((key) => ({key, value: String(subject[key as keyof typeof subject])})),
    })
    return {
        ...panel,
        request: {...panel.request, trustee_id: TRUSTEE_ID},
        ceremony_id: Object.values(target)[0],
        ceremony_name: organization.ceremonyName,
        trustee_name: organization.trusteeName,
    }
}

/** The fake signing service: the request waits (or was signed earlier) and Jose's signature completes it. */
export async function keyShareApi(
    action: SigningAction,
    target: Record<string, string>,
    organization: IKeyShareOrganization,
    keyShare: string,
    signedEarlier: boolean
) {
    const completed = await keySharePanel(
        action,
        target,
        organization,
        keyShare,
        SigningRequestStatus.Completed
    )
    return signedEarlier
        ? fakeApi(completed)
        : fakeApi(
              await keySharePanel(
                  action,
                  target,
                  organization,
                  keyShare,
                  SigningRequestStatus.Waiting
              ),
              {after: completed}
          )
}

/**
 * In the signing dialog that opened: checks the details table names the
 * ceremony, the trustee and the key share's hash, then signs with Jose's
 * certificate.
 */
export async function signKeyShare(
    organization: IKeyShareOrganization,
    keyShare: string,
    target: Record<string, string>
) {
    const dialog = await within(document.body).findByRole("dialog", {name: /^Sign /})
    await waitFor(() => expect(dialog).toBeVisible())
    const view = within(dialog)
    const row = (label: string) => view.getByRole("rowheader", {name: label}).closest("tr")
    const [[field, id]] = Object.entries(target)
    const fieldLabel = field === "keys_ceremony_id" ? "Ceremony" : "Tally session"
    // The ceremony's name beside the signed id it names.
    const ceremonyRow = row(detailLabel(organization, field, fieldLabel))
    await expect(ceremonyRow).toHaveTextContent(organization.ceremonyName)
    await expect(ceremonyRow).toHaveTextContent(id)
    const trusteeRow = row(detailLabel(organization, "trustee_id", "Trustee"))
    await expect(trusteeRow).toHaveTextContent(organization.trusteeName)
    await expect(trusteeRow).toHaveTextContent(TRUSTEE_ID)
    await expect(row("Key share SHA-256")).toHaveTextContent(await sha256(keyShare))
    const continueButton = view.getByRole("button", {name: "Continue"})
    await waitFor(() => expect(continueButton).toBeEnabled())
    await userEvent.click(continueButton)
    await userEvent.upload(view.getByLabelText("Certificate file"), CERTIFICATE_FILES.jose())
    await userEvent.type(view.getByLabelText("Certificate password"), PASSWORD)
    await userEvent.click(view.getByRole("button", {name: "Open certificate"}))
    const sign = await view.findByRole("button", {name: "Sign"}, {timeout: 10000})
    await waitFor(() => expect(sign).toBeEnabled())
    await userEvent.click(sign)
}

/** What the step says while it waits for the signature, in the organization's words. */
export const signingNote = (organization: IKeyShareOrganization) =>
    organization.overrides["adminPortal:signing.keyShare.signing"] ??
    "Sign your key share in the signing panel. It is recorded once you have signed."
