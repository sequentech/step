// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

// Signing journeys: the people, their certificate files, and a fake Harvest
// that answers the signing actions the way the contract describes them
// (docs/docusaurus/docs/07-developers/14-signing/02-signing-api.md). The browser opens the files,
// checks them and signs for real; this server verifies what it receives with
// node's crypto, as Harvest does with OpenSSL.
import {createHash, verify, X509Certificate} from "node:crypto"
import {readFileSync} from "node:fs"
import {dirname, resolve} from "node:path"
import {fileURLToPath} from "node:url"
import forge from "node-forge"
import * as pkijs from "pkijs"
import {expect, type Page} from "@playwright/test"
import type {GraphQLCall, GraphQLReply} from "@sequentech/ui-test-kit/mocks/graphql"
import {FIXED_TIME, IDS} from "@sequentech/ui-test-kit/fixtures"
import type {ISigningPanelData} from "../../../src/lib/signing/api"
import type {
    ICertificateCheckResult,
    ISigningRequest,
    ISigningRuleCapacity,
    ISigningRuleRow,
    ISigningChecksRow,
    ISigningRequestListRow,
    IStaffCertificate,
    IStaffCrl,
    IStaffIssuer,
} from "../../../src/lib/signing/types"
import {TENANT_ID, type AdminPortal} from "../fixtures"
import {electionEvent, listOf, mockEvent, type Row} from "../events/data"

const ADMIN_PORTAL = resolve(dirname(fileURLToPath(import.meta.url)), "../../..")
const FIXTURES = resolve(ADMIN_PORTAL, "src/lib/signing/__fixtures__")
/** The signing organizations the scenario builder loads (PR 13). */
const ORGANIZATIONS = resolve(ADMIN_PORTAL, "../../scripts/dev/scenario/signing-organizations")

interface IFixtureFile {
    password: string
    algorithm: string
    commonName: string
    fingerprintSha256: string
}
const MANIFEST = JSON.parse(readFileSync(resolve(FIXTURES, "fixtures.json"), "utf8")) as {
    files: Record<string, IFixtureFile>
    revokedSerials: string[]
}
/** The serials the test PKI's revocation list revokes. */
export const REVOKED_SERIALS = MANIFEST.revokedSerials

export const EVENT_ID = IDS.event
export const EVENT_URL = `/sequent_backend_election_event/${EVENT_ID}`
export const ELECTION_ID = "a1000000-0000-4000-8000-000000000001"
export const SPAIN_ID = "a2000000-0000-4000-8000-000000000001"
export const PORTUGAL_ID = "a2000000-0000-4000-8000-000000000002"
export const POST = "Madrid PE"
export const SPAIN = "Spain"
export const PORTUGAL = "Portugal"

/** A file the person picks, from the synthetic test PKI (scripts/signing/make-test-p12.sh). */
export function certificateFile(file: string, name = file) {
    return {name, mimeType: "application/x-pkcs12", buffer: readFileSync(resolve(FIXTURES, file))}
}
export const fixture = (file: string): IFixtureFile => MANIFEST.files[file]

/** The trusted chain (root and intermediate) the event's Security Officer imported. */
export const TRUSTED_CHAIN_PEM = readFileSync(resolve(FIXTURES, "test-pnpki-chain.pem"), "utf8")
const pemBlocks = (text: string) =>
    text.match(/-----BEGIN CERTIFICATE-----[\s\S]+?-----END CERTIFICATE-----/g) ?? []
const TRUSTED = pemBlocks(TRUSTED_CHAIN_PEM).map((pem) => new X509Certificate(pem))
const TRUSTED_ROOT = TRUSTED.find((cert) => cert.subject === cert.issuer) as X509Certificate
const commonNameOf = (name: string) => /CN=([^\n,]+)/.exec(name)?.[1] ?? name
/** When the issuers' revocation lists were last downloaded. */
export const CRL_FETCHED_AT = "2026-01-15T11:00:00.000Z"

/** The leaf certificate of an RSA fixture as PEM, as a Security Officer would export it. */
export function leafPem(file: string): string {
    const der = readFileSync(resolve(FIXTURES, file)).toString("binary")
    const p12 = forge.pkcs12.pkcs12FromAsn1(forge.asn1.fromDer(der), fixture(file).password)
    const bags = p12.getBags({bagType: forge.pki.oids.certBag})[forge.pki.oids.certBag] ?? []
    const leaf = bags.find(
        (bag) => bag.cert?.subject.getField("CN")?.value === fixture(file).commonName
    )
    if (!leaf?.cert) throw new Error(`no leaf certificate in ${file}`)
    return forge.pki.certificateToPem(leaf.cert).replace(/\r\n/g, "\n")
}

export interface Person {
    userId: string
    username: string
    firstName: string
    lastName: string
    /** The `title` attribute, as the panel shows it. */
    title: string
    /** Their certificate file. */
    certificate: string
}
export const displayName = (person: Person) => `${person.firstName} ${person.lastName}`

export const MARIA: Person = {
    userId: "66666666-6666-4666-8666-000000000001",
    username: "sbei-madrid-1",
    firstName: "Maria L.",
    lastName: "Santos",
    title: "Chairperson",
    certificate: "maria-santos-rsa-aes.p12",
}
export const JOSE: Person = {
    userId: "66666666-6666-4666-8666-000000000002",
    username: "sbei-madrid-2",
    firstName: "Jose P.",
    lastName: "Reyes",
    title: "Poll Clerk",
    certificate: "jose-reyes-ec-aes.p12",
}
export const ANA: Person = {
    userId: "66666666-6666-4666-8666-000000000003",
    username: "sbei-madrid-3",
    firstName: "Ana M.",
    lastName: "Cruz",
    title: "Third Member",
    certificate: "ana-cruz-rsa-aes.p12",
}
export const SBEIS = [MARIA, JOSE, ANA]

/** The identity provider signs `person` in next. */
export function signInAs(portal: AdminPortal, person: Person) {
    portal.user = {
        id: person.userId,
        username: person.username,
        firstName: person.firstName,
        lastName: person.lastName,
    }
}

/** The roles of an SBEI who signs `signPermissions` (no admin-user, as in the presets). */
export const sbeiRoles = (...signPermissions: string[]) => [
    "election-event-read",
    "election-read",
    ...signPermissions,
]

// --- The contract's wire values ----------------------------------------------

/** Canonical JSON (design §2): sorted keys, no whitespace. */
export function canonicalJson(value: unknown): string {
    if (Array.isArray(value)) return `[${value.map(canonicalJson).join(",")}]`
    if (value && typeof value === "object") {
        const record = value as Record<string, unknown>
        return `{${Object.keys(record)
            .sort()
            .map((key) => `${JSON.stringify(key)}:${canonicalJson(record[key])}`)
            .join(",")}}`
    }
    return JSON.stringify(value)
}
export const sha256 = (bytes: Uint8Array | string) =>
    createHash("sha256").update(bytes).digest("hex")
const iso = (ms: number) => new Date(ms).toISOString()

/** Harvest's error body, as Hasura forwards a failed action. */
export function actionError(
    status: number,
    code: string,
    message: string,
    extra: Record<string, unknown> = {}
): GraphQLReply {
    return {
        errors: [
            {
                message,
                extensions: {
                    code,
                    ...extra,
                    internal: {
                        response: {
                            status,
                            body: JSON.stringify({message, extensions: {code, ...extra}}),
                        },
                    },
                },
            },
        ],
    }
}

/** What a verified approval proves, kept for the test's own assertions. */
export interface IReceivedApproval {
    person: Person
    algorithm: string
    /** node's crypto verified the payload signature with the leaf's public key. */
    payloadVerified: boolean
    /** PDF: the CMS's messageDigest is the prepared digest and its signature verifies. */
    cmsVerified: boolean | null
    revision: number | null
    fingerprint: string
}

export interface IRequestSpec {
    id: string
    action: string
    code: string
    required: number
    requestedBy: Person
    signers: Person[]
    subject: Record<string, unknown>
    areaId?: string | null
    areaName?: string | null
    /** The Post's name; Madrid PE by default. */
    postName?: string
    /** PDF actions: the base document the payload names. */
    document?: {name: string; bytes: Uint8Array}
    /** Signed before the journey starts, in order. */
    signed?: Person[]
    /** What the action keeps as its result once the last signature arrives. */
    execute?: (state: SigningRequestState) => Record<string, unknown>
}

interface IApproval {
    person: Person
    signedAt: string
    commonName: string
    subject: string
    fingerprint: string
}

export class SigningRequestState {
    readonly approvals: IApproval[] = []
    readonly received: IReceivedApproval[] = []
    status = "waiting"
    cancelReason: string | null = null
    completedAt: string | null = null
    executionResult: Record<string, unknown> | null = null
    /** PDF: signed revisions after the base. */
    revisions = 0
    prepared: {revision: number; digest: Buffer; signer: string} | null = null
    /** Every digest `signingPdfPrepare` answered, in order. */
    readonly preparedDigests: Buffer[] = []
    readonly canonicalPayload: string
    readonly documentSha256: string | null
    documentUrl: string | null = null
    readonly createdAt: string
    readonly expiresAt: string

    constructor(
        readonly spec: IRequestSpec,
        now: number
    ) {
        this.createdAt = iso(now)
        this.expiresAt = iso(now + 60 * 60 * 1000)
        this.documentSha256 = spec.document ? sha256(spec.document.bytes) : null
        this.canonicalPayload = canonicalJson({
            action: spec.action,
            area_id: spec.areaId ?? null,
            code: spec.code,
            created_at: this.createdAt,
            domain: "step-signing/v1",
            election_event_id: EVENT_ID,
            election_id: ELECTION_ID,
            expires_at: this.expiresAt,
            request_id: spec.id,
            requested_by: spec.requestedBy.userId,
            subject: spec.subject,
            tenant_id: TENANT_ID,
        })
    }

    get request(): ISigningRequest {
        const {spec} = this
        return {
            id: spec.id,
            tenant_id: TENANT_ID,
            election_event_id: EVENT_ID,
            action: spec.action as ISigningRequest["action"],
            election_id: ELECTION_ID,
            area_id: spec.areaId ?? null,
            trustee_id: null,
            subject: spec.subject,
            canonical_payload: this.canonicalPayload,
            payload_sha256: sha256(this.canonicalPayload),
            document_id: spec.document ? `${spec.id}-document` : null,
            document_sha256: this.documentSha256,
            code: spec.code,
            config_revision: "2",
            rule_revision: 3,
            required: spec.required,
            status: this.status as ISigningRequest["status"],
            cancel_reason: this.cancelReason as ISigningRequest["cancel_reason"],
            cancelled_by: null,
            requested_by: spec.requestedBy.userId,
            requested_by_username: spec.requestedBy.username,
            requested_by_name: displayName(spec.requestedBy),
            created_at: this.createdAt,
            expires_at: this.expiresAt,
            completed_at: this.completedAt,
            executed_at: this.status === "executed" ? this.completedAt : null,
            execution_result: this.executionResult,
        }
    }

    /** `signingGetRequest`'s panel for `viewer` (the contract's Panel). */
    panel(viewer: string | undefined): ISigningPanelData & Record<string, unknown> {
        const {spec} = this
        return {
            request: this.request,
            rule: {
                action: spec.action as ISigningRequest["action"],
                requirement: "required" as never,
                signatures: spec.required,
                requester_signing: "allowed" as never,
                expires_minutes: 60,
                revision: 3,
            },
            count: this.approvals.length,
            required: spec.required,
            signers: spec.signers.map((person) => {
                const approval = this.approvals.find((a) => a.person === person)
                return {
                    user_id: person.userId,
                    username: person.username,
                    display_name: displayName(person),
                    title: person.title,
                    is_you: person.userId === viewer,
                    status: (approval ? "signed" : "not-signed") as never,
                    signed_at: approval?.signedAt ?? null,
                    certificate_subject: approval?.subject ?? null,
                    certificate_cn: approval?.commonName ?? null,
                }
            }),
            document_url: this.documentUrl,
            document_name: spec.document?.name ?? null,
            document_pages: null,
            document_revision: spec.document
                ? {
                      revision: this.revisions,
                      sha256: this.documentSha256,
                      signed_count: this.revisions,
                      url: this.documentUrl,
                  }
                : null,
            election_name: spec.postName ?? POST,
            area_name: spec.areaName ?? null,
            details: null,
            time_zone: null,
        }
    }

    /** The Requests sub-tab's row. */
    get row(): ISigningRequestListRow {
        const {request} = this
        return {
            id: request.id,
            action: request.action,
            election_id: request.election_id,
            area_id: request.area_id,
            code: request.code,
            required: request.required,
            status: request.status,
            cancel_reason: request.cancel_reason,
            requested_by_username: request.requested_by_username,
            requested_by_name: request.requested_by_name,
            created_at: request.created_at,
            expires_at: request.expires_at,
            approvals: this.approvals.map((approval, index) => ({
                id: `${request.id}-approval-${index}`,
                username: approval.person.username,
                display_name: displayName(approval.person),
                signed_at: approval.signedAt,
            })),
        }
    }
}

const CMS_MESSAGE_DIGEST = "1.2.840.113549.1.9.4"

/** Verifies a signature the way Harvest does: RSA PKCS#1 v1.5 or ECDSA (DER), SHA-256. */
function verifySignature(cert: X509Certificate, data: Uint8Array, signature: Uint8Array) {
    const key = cert.publicKey
    return key.asymmetricKeyType === "ec"
        ? verify("sha256", data, {key, dsaEncoding: "der"}, signature)
        : verify("sha256", data, key, signature)
}

/** The detached CMS signs the prepared digest: its messageDigest and its signature over the signed attributes. */
export function verifyDetachedCms(cmsDer: Uint8Array, digest: Uint8Array, cert: X509Certificate) {
    const info = pkijs.ContentInfo.fromBER(new Uint8Array(cmsDer))
    const signed = new pkijs.SignedData({schema: info.content})
    const signer = signed.signerInfos[0]
    const attributes = signer.signedAttrs
    if (!attributes) return false
    const messageDigest = attributes.attributes.find((a) => a.type === CMS_MESSAGE_DIGEST)
    const value = messageDigest?.values[0] as {valueBlock: {valueHexView: Uint8Array}} | undefined
    if (!value || !Buffer.from(value.valueBlock.valueHexView).equals(Buffer.from(digest))) {
        return false
    }
    // The signature covers DER(SET OF Attribute): the [0] IMPLICIT tag becomes a SET.
    const signedBytes = new Uint8Array(attributes.encodedValue.slice(0))
    signedBytes[0] = 0x31
    return verifySignature(cert, signedBytes, signer.signature.valueBlock.valueHexView)
}

const fingerprintOf = (cert: X509Certificate) => cert.fingerprint256.replace(/:/g, "").toLowerCase()

/**
 * Checks an approval the page sent (`signingApprove`'s variables) with node's
 * crypto: its payload signature over `payload`, made by its chain's leaf.
 */
export function verifyApproval(variables: Record<string, unknown>, payload: string): boolean {
    const leaf = new X509Certificate((variables.chain_pem as string[])[0])
    return verifySignature(
        leaf,
        Buffer.from(payload, "utf8"),
        Buffer.from(String(variables.payload_signature_b64), "base64")
    )
}

/** The leaf certificate of an approval's chain. */
export const approvalLeaf = (variables: Record<string, unknown>) =>
    new X509Certificate((variables.chain_pem as string[])[0])

/**
 * Harvest's signing routes, behind the Hasura actions the widget calls. One
 * person signs once; the request runs its action with the last signature.
 */
export class SigningServer {
    readonly requests = new Map<string, SigningRequestState>()
    /** Certificates registered to a person, by fingerprint (first use registers). */
    readonly registrations = new Map<string, {person: Person; at: string}>()
    /** Revoked serials the server's revocation list knows; refreshed by tests. */
    revokedSerials = new Set<string>()
    /** One-shot answers for the next call of an operation, before the normal handling. */
    private readonly scripted = new Map<
        string,
        Array<(call: GraphQLCall) => GraphQLReply | undefined>
    >()

    constructor(
        private readonly portal: AdminPortal,
        /** Who may sign in. */
        private readonly people: Person[] = SBEIS
    ) {
        const on = (operation: string, handle: (call: GraphQLCall) => GraphQLReply) =>
            portal.graphql.on(operation, (call) => {
                const next = this.scripted.get(operation)?.shift()
                return next?.(call) ?? handle(call)
            })
        on("GetWaitingSigningRequests", (call) => ({
            data: {
                sequent_backend_signing_request: Array.from(this.requests.values())
                    .filter(
                        (state) =>
                            state.status === "waiting" &&
                            `sign-${state.spec.action}` === call.headers["x-hasura-role"]
                    )
                    .map((state) => ({
                        ...state.row,
                        approvals: state.approvals.map((approval, index) => ({
                            id: `${state.spec.id.slice(0, -3)}${String(index + 1).padStart(3, "0")}`,
                            user_id: approval.person.userId,
                            signed_at: approval.signedAt,
                        })),
                    })),
            },
        }))
        on("SigningEventInfo", () => ({
            data: {
                signingEventInfo: {
                    time_zone: "Asia/Manila",
                    titles: Object.fromEntries(
                        this.people.map((person) => [person.userId, person.title])
                    ),
                },
            },
        }))
        on("GetHeldReportRequests", () => ({
            data: {
                signingHeldReportRequests: {
                    requests: Array.from(this.requests.values())
                        .filter((state) =>
                            ["generate-election-returns", "generate-reports"].includes(
                                state.spec.action
                            )
                        )
                        .map((state) => ({
                            request_id: state.spec.id,
                            code: state.spec.code,
                            report_type:
                                state.spec.action === "generate-election-returns"
                                    ? "ELECTORAL_RESULTS"
                                    : "PARTICIPATION_REPORT",
                            election_id: ELECTION_ID,
                            area_id: SPAIN_ID,
                            report_id: null,
                            results_event_id: null,
                            tally_session_id: null,
                            status: state.status,
                        })),
                },
            },
        }))
        on("SigningGetRequest", (call) => {
            const state = this.state(call)
            return {data: {signingGetRequest: {panel: state.panel(this.caller(call)?.userId)}}}
        })
        on("SigningCheckCertificate", (call) => {
            const state = this.state(call)
            const chain = call.variables.chain_pem as string[]
            const leaf = new X509Certificate(chain[0])
            return {
                data: {
                    signingCheckCertificate: {
                        checks: this.checks(state, this.caller(call), chain),
                        certificate: {
                            common_name: commonNameOf(leaf.subject),
                            fingerprint_sha256: fingerprintOf(leaf),
                        },
                        registration: this.registrations.get(fingerprintOf(leaf))
                            ? "registered"
                            : "first-use",
                        revocation_status: "checked",
                    },
                },
            }
        })
        on("SigningPdfPrepare", (call) => {
            const state = this.state(call)
            const leaf = new X509Certificate((call.variables.chain_pem as string[])[0])
            const revision = state.revisions + 1
            // The SHA-256 of the next revision's ByteRange: its own bytes in the real PDF.
            const digest = createHash("sha256")
                .update(`${state.spec.id}:${revision}:${fingerprintOf(leaf)}`)
                .digest()
            state.prepared = {revision, digest, signer: fingerprintOf(leaf)}
            state.preparedDigests.push(digest)
            return {
                data: {
                    signingPdfPrepare: {
                        revision,
                        digest_b64: digest.toString("base64"),
                        signing_time: iso(this.portal.now),
                    },
                },
            }
        })
        on("SigningApprove", (call) => this.approve(call))
        on("SigningOpenFailure", (call) => ({
            data: {signingOpenFailure: {request_id: call.variables.request_id}},
        }))
        on("SigningHandover", (call) => ({
            data: {signingHandover: {request_id: call.variables.request_id}},
        }))
        on("SigningCancel", (call) => {
            const state = this.state(call)
            state.status = "cancelled"
            state.cancelReason = "by-operator"
            return {data: {signingCancel: {request_id: state.spec.id}}}
        })
    }

    /** A request as the guarded route creates it; earlier signatures are replayed. */
    add(spec: IRequestSpec): SigningRequestState {
        const state = new SigningRequestState(spec, this.portal.now)
        if (spec.document) {
            const key = `${TENANT_ID}/${EVENT_ID}/signing/${spec.id}/base.pdf`
            this.portal.s3.putBytes("private", key, spec.document.bytes, "application/pdf")
            state.documentUrl = this.portal.s3.presign(key, `base-${spec.id}`)
        }
        for (const person of spec.signed ?? []) this.record(state, person, this.leafOf(person))
        this.requests.set(spec.id, state)
        return state
    }

    /** Answers the next call of `operation` with `reply` instead. */
    once(operation: string, reply: (call: GraphQLCall) => GraphQLReply | undefined) {
        const queue = this.scripted.get(operation) ?? []
        queue.push(reply)
        this.scripted.set(operation, queue)
    }

    /** `person` signs the request on another laptop (a PDF gains its revision). */
    signElsewhere(requestId: string, person: Person) {
        const state = this.requests.get(requestId) as SigningRequestState
        if (state.spec.document) state.revisions += 1
        this.record(state, person, this.leafOf(person))
    }

    register(person: Person, file = person.certificate) {
        this.registrations.set(fixture(file).fingerprintSha256, {
            person,
            at: "2026-01-10T09:00:00.000Z",
        })
    }

    /** Every approval the server verified for a request. */
    received(requestId: string) {
        return this.requests.get(requestId)?.received ?? []
    }

    private leafOf(person: Person) {
        // A replayed signer's certificate: only its names and fingerprint matter.
        const info = fixture(person.certificate)
        return {
            commonName: info.commonName,
            subject: `C=PH, O=Test PNPKI, OU=Individual, CN=${info.commonName}`,
            fingerprint: info.fingerprintSha256,
        }
    }

    private record(
        state: SigningRequestState,
        person: Person,
        leaf: {commonName: string; subject: string; fingerprint: string}
    ) {
        state.approvals.push({
            person,
            signedAt: iso(this.portal.now + state.approvals.length * 60_000),
            ...leaf,
        })
        if (!this.registrations.has(leaf.fingerprint)) {
            this.registrations.set(leaf.fingerprint, {person, at: iso(this.portal.now)})
        }
        if (state.approvals.length >= state.spec.required) {
            state.status = "executed"
            state.completedAt = state.approvals.at(-1)?.signedAt ?? null
            state.executionResult = state.spec.execute?.(state) ?? null
        }
    }

    private state(call: GraphQLCall): SigningRequestState {
        const state = this.requests.get(String(call.variables.request_id))
        if (!state) throw new Error(`unknown signing request ${String(call.variables.request_id)}`)
        return state
    }

    /** The signed-in person the call's bearer token names. */
    caller(call: GraphQLCall): Person | undefined {
        const token = call.headers.authorization?.replace(/^Bearer /, "") ?? ""
        const sub = this.portal.oidc.verifyAccessToken(token)?.sub
        return this.people.find((person) => person.userId === sub)
    }

    /** The certificate checks of design §7, in CertificateCheckId order. */
    checks(state: SigningRequestState, caller: Person | undefined, chainPem: string[]) {
        const chain = chainPem.map((pem) => new X509Certificate(pem))
        const leaf = chain[0]
        const linked = chain.every((cert, index) =>
            index + 1 < chain.length ? cert.verify(chain[index + 1].publicKey) : true
        )
        const top = chain[chain.length - 1]
        const trusted =
            linked &&
            (top.fingerprint256 === TRUSTED_ROOT.fingerprint256 ||
                top.verify(TRUSTED_ROOT.publicKey))
        const now = this.portal.now
        const valid = Date.parse(leaf.validFrom) <= now && now <= Date.parse(leaf.validTo)
        const fingerprint = fingerprintOf(leaf)
        const holder = this.registrations.get(fingerprint)
        const other = holder && holder.person.userId !== caller?.userId ? holder.person : null
        const checks: ICertificateCheckResult[] = [
            {
                id: "trusted-issuer" as never,
                ok: trusted,
                detail: trusted ? commonNameOf(TRUSTED_ROOT.subject) : null,
            },
            {id: "valid-now" as never, ok: valid, detail: null},
            {id: "signing-key-usage" as never, ok: true, detail: null},
            {
                id: "not-revoked" as never,
                ok: !this.revokedSerials.has(leaf.serialNumber),
                detail: CRL_FETCHED_AT,
            },
            {id: "registered" as never, ok: !other, detail: holder && !other ? holder.at : null},
        ]
        if (other) {
            checks.push({id: "registered-to-other" as never, ok: false, detail: displayName(other)})
        }
        if (state.approvals.some((approval) => approval.fingerprint === fingerprint)) {
            checks.push({id: "already-signed" as never, ok: false, detail: null})
        }
        return checks
    }

    private approve(call: GraphQLCall): GraphQLReply {
        const state = this.state(call)
        const caller = this.caller(call)
        if (!caller) return actionError(403, "forbidden", "not a signer")
        if (state.status !== "waiting") {
            return actionError(409, "request-closed", "the request is not waiting", {
                status: state.status,
            })
        }
        if (state.approvals.some((approval) => approval.person === caller)) {
            return actionError(422, "already-signed", "already signed")
        }
        const {variables} = call
        const chainPem = variables.chain_pem as string[]
        const leaf = new X509Certificate(chainPem[0])
        const payloadVerified = verifySignature(
            leaf,
            Buffer.from(state.canonicalPayload, "utf8"),
            Buffer.from(String(variables.payload_signature_b64), "base64")
        )
        const revision = typeof variables.revision === "number" ? variables.revision : null
        let cmsVerified: boolean | null = null
        if (state.spec.document) {
            if (!state.prepared || revision !== state.prepared.revision) {
                return actionError(409, "stale-revision", "the prepared revision is stale")
            }
            cmsVerified = verifyDetachedCms(
                Buffer.from(String(variables.pdf_cms_b64), "base64"),
                state.prepared.digest,
                leaf
            )
        }
        state.received.push({
            person: caller,
            algorithm: String(variables.algorithm),
            payloadVerified,
            cmsVerified,
            revision,
            fingerprint: fingerprintOf(leaf),
        })
        const failed = this.checks(state, caller, chainPem).find((check) => !check.ok)
        if (!payloadVerified || cmsVerified === false || failed) {
            return actionError(422, "signing-refused", "the signature was refused", {
                check: failed?.id ?? "signature",
            })
        }
        if (state.spec.document) {
            state.revisions += 1
            state.prepared = null
        }
        this.record(state, caller, {
            commonName: commonNameOf(leaf.subject),
            subject: leaf.subject.replace(/\n/g, ", "),
            fingerprint: fingerprintOf(leaf),
        })
        return {
            data: {
                signingApprove: {
                    status: state.status,
                    count: state.approvals.length,
                    required: state.spec.required,
                },
            },
        }
    }
}

/** The seal record closing voting keeps (PR 10's SealRecord). */
export function sealRecord(
    state: SigningRequestState,
    seals: Array<Record<string, unknown>> = []
): Record<string, unknown> {
    return {
        closed_at: state.approvals.at(-1)?.signedAt,
        election_id: ELECTION_ID,
        channels: ["ONLINE"],
        from: ["ONLINE=OPEN"],
        code: state.spec.code,
        payload_sha256: sha256(state.canonicalPayload),
        signatures: state.approvals.map((approval) => ({
            user_id: approval.person.userId,
            username: approval.person.username,
            display_name: displayName(approval.person),
            signed_at: approval.signedAt,
            certificate_fingerprint: approval.fingerprint,
            certificate_subject: approval.subject,
            certificate_cn: approval.commonName,
        })),
        seals,
    }
}

// --- The pages ------------------------------------------------------------------

export function election(status: Row = {}, post = POST): Row {
    return {
        id: ELECTION_ID,
        tenant_id: TENANT_ID,
        election_event_id: EVENT_ID,
        name: post,
        alias: null,
        description: null,
        presentation: {i18n: {en: {name: post}}, initialization_report_policy: "not-required"},
        status,
        contests: [],
        contests_aggregate: {aggregate: {count: 0}, nodes: []},
        voting_channels: {online: true, kiosk: false, early_voting: false, telephone: false},
        created_at: FIXED_TIME,
        last_updated_at: FIXED_TIME,
    }
}

export const AREAS = [
    {id: SPAIN_ID, name: SPAIN, election_event_id: EVENT_ID, tenant_id: TENANT_ID},
    {id: PORTUGAL_ID, name: PORTUGAL, election_event_id: EVENT_ID, tenant_id: TENANT_ID},
]

/** The event with its Post and countries. */
export function mockSigningEvent(portal: AdminPortal, overrides: Row = {}, post = POST) {
    mockEvent(portal, electionEvent(overrides))
    portal.graphql.on(
        "sequent_backend_election",
        listOf("sequent_backend_election", [election({}, post)])
    )
    portal.graphql.on("sequent_backend_area", listOf("sequent_backend_area", AREAS))
}

/** A signing permission held as the call's Hasura role. */
export function expectRole(portal: AdminPortal, operation: string, role: string) {
    const calls = portal.graphql.callsTo(operation)
    expect(calls.length, `${operation} was called`).toBeGreaterThan(0)
    expect(
        calls.map((call) => call.headers["x-hasura-role"]),
        `${operation} x-hasura-role`
    ).toEqual(calls.map(() => role))
}

/** The signing dialog; its local-signing note is on every step (R2). */
export function signingDialog(page: Page) {
    return page.getByRole("dialog", {name: /^Sign the /})
}
export const LOCAL_NOTE =
    "Signing happens in this browser. Your certificate file, its private key and its password are never sent. Only your signature and your public certificate go to the server."

/** The Certificate step: picks the file, types its password and opens it. */
export async function openCertificate(page: Page, file: string, password = fixture(file).password) {
    const dialog = signingDialog(page)
    await dialog.getByLabel("Certificate file").setInputFiles(certificateFile(file))
    await dialog.getByTestId("certificate-password").fill(password)
    await dialog.getByRole("button", {name: "Open certificate", exact: true}).click()
}

// --- Election Event > Signatures -------------------------------------------------

export const SBEI_GROUP = {id: "b1000000-0000-4000-8000-000000000001", name: "sbei", path: "/sbei"}
export const OFOV_GROUP = {id: "b1000000-0000-4000-8000-000000000002", name: "ofov", path: "/ofov"}

export function capacity(overrides: Partial<ISigningRuleCapacity> = {}): ISigningRuleCapacity {
    return {
        max: 3,
        posts: [{election_id: ELECTION_ID, name: POST, count: 3}],
        posts_short: [],
        posts_short_without_requester: [],
        roles: [SBEI_GROUP],
        waiting: 0,
        config_version: 2,
        ...overrides,
    }
}

export function rule(action: string, overrides: Partial<ISigningRuleRow> = {}): ISigningRuleRow {
    return {
        action: action as ISigningRuleRow["action"],
        requirement: "required" as never,
        signatures: 3,
        requester_signing: "allowed" as never,
        expires_minutes: 60,
        revision: 4,
        updated_by: "c1000000-0000-4000-8000-000000000001",
        updated_by_name: "Carlos Mendez",
        updated_at: "2026-01-14T10:00:00.000Z",
        ...overrides,
    }
}

export interface ISignaturesTabState {
    rules: ISigningRuleRow[]
    capacities: Partial<Record<string, ISigningRuleCapacity>>
    checks: ISigningChecksRow[]
    issuers: IStaffIssuer[]
    certificates: IStaffCertificate[]
    crls: IStaffCrl[]
    requests: () => ISigningRequestListRow[]
}

/** The tab's table selects and capacity reads, answered from `state` at each call. */
export function mockSignaturesTab(portal: AdminPortal, state: ISignaturesTabState) {
    portal.graphql.on("GetSigningRules", () => ({
        data: {sequent_backend_signing_rule: state.rules},
    }))
    // The selections are aliased, so the root fields resolve by their arguments, as Hasura's do.
    portal.graphql.on("GetSigningRuleCapacities", () => ({
        data: {
            signingRuleCapacity: ({action}: {action: string}) =>
                state.capacities[action] ?? capacity({roles: []}),
        },
    }))
    portal.graphql.on("GetSigningCertificates", () => ({
        data: {
            sequent_backend_signing_checks: () => state.checks,
            sequent_backend_certificate_authority: () => state.issuers,
            sequent_backend_staff_certificate: () => state.certificates,
            sequent_backend_staff_crl: () => state.crls,
        },
    }))
    portal.graphql.on("GetSigningRequests", () => ({
        data: {sequent_backend_signing_request: state.requests()},
    }))
}

export function staffCertificate(
    person: Person,
    overrides: Partial<IStaffCertificate> = {}
): IStaffCertificate {
    const info = fixture(person.certificate)
    return {
        id: `d1000000-0000-4000-8000-${person.userId.slice(-12)}`,
        user_id: person.userId,
        username: person.username,
        election_id: ELECTION_ID,
        fingerprint_sha256: info.fingerprintSha256,
        spki_sha256: "00".repeat(32),
        holder_sha256: "11".repeat(32),
        serial: "1001",
        subject: `C=PH, O=Test PNPKI, OU=Individual, CN=${info.commonName}`,
        issuer: "C=PH, O=Test PNPKI, CN=Test PNPKI Individual CA",
        not_before: "2025-01-01T00:00:00.000Z",
        not_after: "2029-12-31T23:59:59.000Z",
        status: "active" as never,
        registration: "first-use" as never,
        // On first use, the holder registered it.
        registered_by: person.userId,
        registered_at: "2026-01-10T09:00:00.000Z",
        revoked_by: null,
        revoked_at: null,
        revoke_reason: null,
        user_display_name: displayName(person),
        registered_by_name: null,
        revoked_by_name: null,
        linked_to: null,
        ...overrides,
    }
}

/** The trusted issuers of a chain PEM, as the certificate_authority select returns them. */
export function issuersOf(chainPem: string): IStaffIssuer[] {
    return pemBlocks(chainPem).map((pem, index) => {
        const cert = new X509Certificate(pem)
        return {
            id: `e1000000-0000-4000-8000-00000000000${index + 1}`,
            common_name: commonNameOf(cert.subject),
            subject: cert.subject.replace(/\n/g, ", "),
            issuer: cert.issuer.replace(/\n/g, ", "),
            issuer_common_name: commonNameOf(cert.issuer),
            not_after: new Date(cert.validTo).toISOString(),
            fingerprint_sha256: fingerprintOf(cert),
        }
    })
}

/** A signing organization's fixture (PR 13): its tenant settings, groups, Posts and rules. */
export interface IOrganization {
    name: string
    tenant: {display_name: string; i18n: Record<string, Record<string, string>>}
    groups: Array<{name: string; permissions: string[]}>
    posts: Array<{key: string; name: string}>
    signers: Array<{group: string; per_post?: string[]; event_wide?: string[]}>
    signing_rules?: Array<{action: string; requirement: string; signatures: number}>
    preset?: string
}
export function organization(name: string): IOrganization {
    return JSON.parse(readFileSync(resolve(ORGANIZATIONS, `${name}.json`), "utf8")) as IOrganization
}

/** An organization's signing rules: its own, or those of the preset it loads. */
export function organizationRules(org: IOrganization) {
    if (org.signing_rules) return org.signing_rules
    const preset = JSON.parse(
        readFileSync(resolve(ADMIN_PORTAL, "../..", String(org.preset)), "utf8")
    ) as {signing_rules: NonNullable<IOrganization["signing_rules"]>}
    return preset.signing_rules
}

/** The tenant's English override of an admin portal key, if the organization has one. */
export const overrideOf = (org: IOrganization, key: string): string | undefined =>
    org.tenant.i18n.en?.[`adminPortal:${key}`]

// --- Election returns (a PDF action) ------------------------------------------

export const ER_REQUEST_ID = "f3000000-0000-4000-8000-000000000001"
export const ER_CODE = "4B2D-08E1"
/** The election returns with their empty signature page: the base the payload names. */
export const ER_BASE = new TextEncoder().encode(
    "%PDF-1.7\n% Synthetic election returns, Madrid PE, Spain, with three signature fields\n%%EOF\n"
)
export const ER_ROLES = sbeiRoles(
    "election-event-signatures-tab",
    "signing-requests-read",
    "sign-generate-election-returns",
    "miru-create",
    "miru-send"
)

/** The tally's election returns request for Madrid PE · Spain, signed so far by `signed`. */
export function electionReturns(portal: AdminPortal, signed = [MARIA, JOSE]) {
    const server = new SigningServer(portal)
    for (const person of signed) server.register(person)
    const state = server.add({
        id: ER_REQUEST_ID,
        action: "generate-election-returns",
        code: ER_CODE,
        required: 3,
        requestedBy: MARIA,
        signers: [MARIA, JOSE, ANA],
        areaId: SPAIN_ID,
        areaName: SPAIN,
        subject: {
            document_sha256: sha256(ER_BASE),
            report_type: "ELECTORAL_RESULTS",
            template_id: null,
        },
        document: {name: `Election returns, ${POST}, ${SPAIN}.pdf`, bytes: ER_BASE},
        signed,
        execute: (done) => ({
            document_id: `${ER_REQUEST_ID}-signed`,
            sha256: sha256(`signed revision ${done.revisions}`),
            revision: done.revisions,
            mailed: false,
        }),
    })
    // The tally made two signed revisions before this journey.
    state.revisions = signed.length
    mockSigningEvent(portal)
    mockSignaturesTab(portal, {
        rules: [],
        capacities: {},
        checks: [],
        issuers: [],
        certificates: [],
        crls: [],
        requests: () => [state.row],
    })
    return {server, state}
}

/** Election Event > Signatures > Requests, then the request's panel. */
export async function openRequest(
    page: Page,
    portal: AdminPortal,
    title = `Election returns · ${POST} · ${SPAIN}`
) {
    await page.goto(`${portal.origin}${EVENT_URL}?lang=en`)
    await page.getByRole("tab", {name: "Signatures", exact: true}).click()
    await page.getByRole("button", {name: title, exact: true}).click()
    const panel = page.locator(".MuiDrawer-paper").filter({has: page.getByTestId("signing-status")})
    await expect(panel.getByTestId("signing-status")).toBeVisible()
    return panel
}
