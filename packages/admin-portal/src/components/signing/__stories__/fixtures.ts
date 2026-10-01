// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Synthetic signing requests and a fake Harvest for the signing widget's
// stories. The certificates are the real fixtures of scripts/signing.

import {fn} from "storybook/test"
import {TENANT_ID, EVENT_ID} from "@/__stories__/AdminStoryProvider"
import {tenantRecord} from "@/__stories__/fixtures"
import type {Sequent_Backend_Tenant} from "@/gql/graphql"
import type {ISigningApi, ISigningPanelData} from "@/lib/signing/api"
import {hex} from "@/lib/signing/der"
import {canonicalJson} from "@/lib/signing/request"
import {multiKeyP12} from "@/lib/signing/__stories__/multiKeyP12"
import {
    CancelReason,
    CertificateCheckId,
    RequesterSigning,
    SigningAction,
    SigningRequestStatus,
    SigningRequirement,
    SigningSignerStatus,
    type ICertificateCheckResult,
    type ISigningSigner,
} from "@/lib/signing/types"
import {IPermissions} from "@/types/keycloak"
import mariaAes from "@/lib/signing/__fixtures__/maria-santos-rsa-aes.p12?url&inline"
import joseEc from "@/lib/signing/__fixtures__/jose-reyes-ec-aes.p12?url&inline"
import maria3des from "@/lib/signing/__fixtures__/maria-santos-rsa-3des.p12?url&inline"
import anaAes from "@/lib/signing/__fixtures__/ana-cruz-rsa-aes.p12?url&inline"
import rosaForeign from "@/lib/signing/__fixtures__/rosa-mendoza-foreign.p12?url&inline"
import fixtureManifest from "@/lib/signing/__fixtures__/fixtures.json"

export const REQUEST_ID = "55555555-5555-4555-8555-555555555555"
export const ELECTION_ID = "33333333-3333-4333-8333-333333333333"
export const AREA_ID = "44444444-4444-4444-8444-444444444444"
export const CODE = "7F3A-91C2"
export const POST = "Madrid PE"
export const COUNTRY = "Spain"
/** Timestamps of a request started 10:30 UTC that expires 11:30 UTC. */
export const CREATED_AT = "2028-05-12T10:30:00Z"
export const EXPIRES_AT = "2099-05-12T11:30:00Z"
/** The bytes "Open the document" downloads; the payload names their SHA-256. */
export const DOCUMENT_BYTES = new TextEncoder().encode("%PDF-1.7 synthetic election returns")
export const documentSha256 = async (bytes: Uint8Array = DOCUMENT_BYTES) =>
    hex(await crypto.subtle.digest("SHA-256", bytes as Uint8Array<ArrayBuffer>))
export const PASSWORD = fixtureManifest.files["maria-santos-rsa-aes.p12"].password

export interface IStoryPerson {
    userId: string
    username: string
    name: string
    title: string
    /** The fixture certificate's common name. */
    certificate: string
}

export const MARIA: IStoryPerson = {
    userId: "66666666-6666-4666-8666-000000000001",
    username: "sbei-madrid-1",
    name: "Maria L. Santos",
    title: "Chairperson",
    certificate: fixtureManifest.files["maria-santos-rsa-aes.p12"].commonName,
}
export const JOSE: IStoryPerson = {
    userId: "66666666-6666-4666-8666-000000000002",
    username: "sbei-madrid-2",
    name: "Jose P. Reyes",
    title: "Poll Clerk",
    certificate: fixtureManifest.files["jose-reyes-ec-aes.p12"].commonName,
}
export const ANA: IStoryPerson = {
    userId: "66666666-6666-4666-8666-000000000003",
    username: "sbei-madrid-3",
    name: "Ana M. Cruz",
    title: "Third Member",
    certificate: fixtureManifest.files["ana-cruz-rsa-aes.p12"].commonName,
}
export const MEMBERS = [MARIA, JOSE, ANA]

/** Two organizations: copy that names the organization must follow the tenant. */
export const ORGANIZATIONS: Record<"first" | "second" | "slugOnly", Sequent_Backend_Tenant> = {
    first: {
        ...tenantRecord,
        slug: "northland-board",
        settings: {...tenantRecord.settings, display_name: "Northland Election Board"},
    },
    second: {
        ...tenantRecord,
        slug: "student-council",
        settings: {...tenantRecord.settings, display_name: "Student Electoral Council"},
    },
    /** A tenant without a display name: its slug stands in. */
    slugOnly: {...tenantRecord, slug: "example-council"},
}

/** The signed-in member, as AuthContextProvider describes them. */
export const signedInAs = (person: IStoryPerson, logout = fn()) => ({
    isAuthenticated: true,
    userId: person.userId,
    username: person.username,
    firstName: person.name.split(" ")[0],
    tenantId: TENANT_ID,
    logout,
})

export const SIGN_ROLES = [
    IPermissions.SIGN_GENERATE_ELECTION_RETURNS,
    IPermissions.SIGN_APPROVE_VOTER,
]

export interface IPanelOptions {
    action?: SigningAction
    status?: SigningRequestStatus
    required?: number
    /** Who has signed, in order. */
    signed?: IStoryPerson[]
    requestedBy?: IStoryPerson
    cancelReason?: CancelReason | null
    subject?: Record<string, unknown>
    details?: ISigningPanelData["details"]
    /** The election event's time zone. */
    timeZone?: string | null
}

const signedAt = (index: number) => `2028-05-12T10:${String(41 + 2 * index).padStart(2, "0")}:00Z`

/** A request as `GET /signing-requests/<id>` answers it, with a consistent canonical payload. */
export async function makePanel({
    action = SigningAction.GenerateElectionReturns,
    status = SigningRequestStatus.Waiting,
    required = 3,
    signed = [],
    requestedBy = MARIA,
    cancelReason = null,
    subject,
    details = null,
    timeZone = null,
}: IPanelOptions = {}): Promise<ISigningPanelData> {
    const withDocument =
        action === SigningAction.GenerateElectionReturns || action === SigningAction.GenerateReports
    const documentHash = await documentSha256()
    subject ??= {
        report_type: "election-returns",
        document_sha256: documentHash,
        template_id: null,
    }
    // Sorted keys, no whitespace (design §2).
    const canonical = canonicalJson({
        action,
        area_id: AREA_ID,
        code: CODE,
        created_at: CREATED_AT,
        domain: "step-signing/v1",
        election_event_id: EVENT_ID,
        election_id: ELECTION_ID,
        expires_at: EXPIRES_AT,
        request_id: REQUEST_ID,
        requested_by: requestedBy.userId,
        subject,
        tenant_id: TENANT_ID,
    })
    const digest = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(canonical))
    const signers: ISigningSigner[] = MEMBERS.map((person) => {
        const index = signed.indexOf(person)
        return {
            user_id: person.userId,
            username: person.username,
            display_name: person.name,
            title: person.title,
            signed_at: index >= 0 ? signedAt(index) : null,
            certificate_subject:
                index >= 0 ? `C=PH, O=Test PNPKI, OU=Individual, CN=${person.certificate}` : null,
            status: index >= 0 ? SigningSignerStatus.Signed : SigningSignerStatus.NotSigned,
            certificate_cn: index >= 0 ? person.certificate : null,
        }
    })
    const done =
        status === SigningRequestStatus.Completed || status === SigningRequestStatus.Executed
    return {
        request: {
            id: REQUEST_ID,
            tenant_id: TENANT_ID,
            election_event_id: EVENT_ID,
            action,
            election_id: ELECTION_ID,
            area_id: AREA_ID,
            trustee_id: null,
            subject,
            canonical_payload: canonical,
            payload_sha256: hex(digest),
            document_id: withDocument ? "77777777-7777-4777-8777-777777777777" : null,
            document_sha256: withDocument ? documentHash : null,
            code: CODE,
            config_revision: null,
            rule_revision: 2,
            required,
            status,
            cancel_reason: cancelReason,
            cancelled_by: cancelReason ? MARIA.userId : null,
            requested_by: requestedBy.userId,
            requested_by_username: requestedBy.username,
            created_at: CREATED_AT,
            expires_at: EXPIRES_AT,
            completed_at: done ? signedAt(signed.length - 1) : null,
            executed_at:
                status === SigningRequestStatus.Executed ? signedAt(signed.length - 1) : null,
            execution_result: null,
        },
        rule: {
            action,
            requirement: SigningRequirement.Required,
            signatures: required,
            requester_signing: RequesterSigning.Allowed,
            expires_minutes: 60,
            revision: 2,
        },
        count: signed.length,
        signers,
        document_url: withDocument ? "https://documents.admin-story.invalid/er.pdf" : null,
        election_name: POST,
        area_name: action === SigningAction.ApproveVoter ? null : COUNTRY,
        document_name: withDocument ? `Election returns, ${POST}, ${COUNTRY}.pdf` : null,
        document_pages: withDocument ? 3 : null,
        details,
        time_zone: timeZone,
    }
}

/** Every check passes; `registeredOn` null is a first use. */
export const passingChecks = (
    registeredOn: string | null = "2028-04-08T09:00:00Z"
): ICertificateCheckResult[] => [
    {id: CertificateCheckId.TrustedIssuer, ok: true, detail: "Test PNPKI Root CA"},
    {id: CertificateCheckId.ValidNow, ok: true, detail: null},
    {id: CertificateCheckId.SigningKeyUsage, ok: true, detail: null},
    {id: CertificateCheckId.NotRevoked, ok: true, detail: "2028-05-12T10:00:00Z"},
    {id: CertificateCheckId.Registered, ok: true, detail: registeredOn},
]

export const withFailure = (
    checks: ICertificateCheckResult[],
    failure: ICertificateCheckResult
): ICertificateCheckResult[] =>
    checks.some((check) => check.id === failure.id)
        ? checks.map((check) => (check.id === failure.id ? failure : check))
        : [...checks, failure]

/** A fake Harvest: answers `data`, dry-runs `checks`, and counts each approval. */
export function fakeApi(
    data: ISigningPanelData,
    {
        checks = passingChecks(),
        after,
    }: {checks?: ICertificateCheckResult[]; after?: ISigningPanelData} = {}
): ISigningApi & {[K in keyof ISigningApi]: ReturnType<typeof fn>} {
    let current = data
    return {
        getRequest: fn(async () => current),
        checkCertificate: fn(async () => ({checks})),
        pdfPrepare: fn(async () => ({
            revision: current.count + 1,
            // SHA-256 of the prepared revision's ByteRange bytes.
            digest_b64: btoa(
                String.fromCharCode.apply(null, Array.from(new Uint8Array(32).fill(7)))
            ),
            signing_time: "2028-05-12T11:00:00Z",
        })),
        approve: fn(async () => {
            if (after) current = after
            return {
                status: current.request.status,
                count: data.count + 1,
                required: data.request.required,
            }
        }),
        reportOpenFailure: fn(async () => undefined),
        handover: fn(async () => undefined),
        cancel: fn(async () => {
            current = {
                ...current,
                request: {
                    ...current.request,
                    status: SigningRequestStatus.Cancelled,
                    cancel_reason: CancelReason.ByRequester,
                },
            }
        }),
        fetchDocument: fn(async () => new Uint8Array(DOCUMENT_BYTES)),
    } as unknown as ISigningApi & {[K in keyof ISigningApi]: ReturnType<typeof fn>}
}

const dataUrlBytes = (dataUrl: string): Uint8Array<ArrayBuffer> =>
    Uint8Array.from(atob(dataUrl.slice(dataUrl.indexOf(",") + 1)), (c) => c.charCodeAt(0))

/** A fixture .p12 as the file a person picks. */
export function p12File(dataUrl: string, name: string): File {
    return new File([dataUrlBytes(dataUrl)], name, {type: "application/x-pkcs12"})
}

export const CERTIFICATE_FILES = {
    maria: () => p12File(mariaAes, "maria-santos.p12"),
    jose: () => p12File(joseEc, "jose-reyes.p12"),
    rosa: () => p12File(rosaForeign, "rosa-mendoza-personal.p12"),
    /** Maria's and Ana's keys in one file: equally good, so the person chooses. */
    twoKeys: () =>
        new File(
            [
                multiKeyP12(
                    [
                        {bytes: dataUrlBytes(maria3des), password: PASSWORD, friendlyName: "Maria"},
                        {bytes: dataUrlBytes(anaAes), password: PASSWORD, friendlyName: "Ana"},
                    ],
                    PASSWORD
                ),
            ],
            "shared-token.p12"
        ),
}

/** Session storage stand-in, so stories never touch the real one. */
export function memoryStorage(initial: Record<string, string> = {}): Storage {
    const items = new Map(Object.entries(initial))
    return {
        get length() {
            return items.size
        },
        clear: () => items.clear(),
        getItem: (key) => items.get(key) ?? null,
        key: (index) => Array.from(items.keys())[index] ?? null,
        removeItem: (key) => void items.delete(key),
        setItem: (key, value) => void items.set(key, value),
    }
}

/** Everything the fake Harvest received, to prove what never leaves the browser. */
export const everythingSent = (api: ISigningApi): string =>
    JSON.stringify(
        Object.values(api).map((call) => (call as ReturnType<typeof fn>).mock.calls),
        (_key, value: unknown) => (value instanceof Uint8Array ? Array.from(value) : value)
    )
