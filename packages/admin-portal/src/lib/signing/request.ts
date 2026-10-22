// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Pure rules of the signing widget: who may sign, who is left, what exactly
// is signed, and the handover note kept across a sign-out.

import {hex, sha256} from "./der"
import {
    DocumentKind,
    RequesterSigning,
    SIGNING_ACTIONS,
    SigningRequestStatus,
    type ISigningRequest,
    type ISigningRequestPanel,
    type ISigningSigner,
} from "./types"
import type {ISigningPanelData} from "./api"

export const isOpenForSigning = (request: ISigningRequest, now: Date = new Date()): boolean =>
    request.status === SigningRequestStatus.Waiting &&
    (request.expires_at === null || new Date(request.expires_at).getTime() > now.getTime())

export const documentKindOf = (request: ISigningRequest): DocumentKind =>
    SIGNING_ACTIONS[request.action]?.document ?? DocumentKind.NoDocument

/** The viewer among the signers: the server's `is_you`, or the same user id. */
export const signerOf = (panel: ISigningRequestPanel, userId: string): ISigningSigner | undefined =>
    panel.signers.find((signer) => signer.is_you === true || signer.user_id === userId)

export const isViewer = (signer: ISigningSigner, userId: string): boolean =>
    signer.is_you === true || signer.user_id === userId

export enum SignBlock {
    None = "none",
    Closed = "closed",
    NoPermission = "no-permission",
    NotASigner = "not-a-signer",
    AlreadySigned = "already-signed",
    Requester = "requester",
}

/** Why the viewer can't sign the request now, or `SignBlock.None`. */
export const signBlock = (
    panel: ISigningRequestPanel,
    userId: string,
    hasSignPermission: boolean,
    now: Date = new Date()
): SignBlock => {
    if (!isOpenForSigning(panel.request, now)) {
        return SignBlock.Closed
    }
    if (!hasSignPermission) {
        return SignBlock.NoPermission
    }
    const me = signerOf(panel, userId)
    if (me?.signed_at) {
        return SignBlock.AlreadySigned
    }
    if (
        panel.request.requested_by === userId &&
        panel.rule.requester_signing === RequesterSigning.NotAllowed
    ) {
        return SignBlock.Requester
    }
    return me ? SignBlock.None : SignBlock.NotASigner
}

/** The eligible people who haven't signed yet, without `exceptUserId`. */
export const pendingSigners = (
    panel: ISigningRequestPanel,
    exceptUserId?: string
): ISigningSigner[] =>
    panel.signers.filter(
        (signer) =>
            !signer.signed_at && (exceptUserId === undefined || !isViewer(signer, exceptUserId))
    )

export const signerName = (signer: ISigningSigner): string => signer.display_name || signer.username

/** The CN of the certificate a signer signed with. */
export const signerCertificate = (signer: ISigningSigner): string | null =>
    signer.certificate_cn ||
    (signer.certificate_subject ? commonNameOf(signer.certificate_subject) : null)

/** "MARIA L. SANTOS" from "C=PH, …, CN=MARIA L. SANTOS, …"; other text as is. */
export const commonNameOf = (subject: string): string => {
    const match = /(?:^|[,/]\s*)CN=([^,/]+)/.exec(subject)
    return match ? match[1].trim() : subject
}

export enum PayloadProblem {
    Unparseable = "unparseable",
    /** Not in canonical form (sorted keys, no whitespace), or not of this domain. */
    NotCanonical = "not-canonical",
    /** The payload describes another request than the one shown. */
    Mismatch = "mismatch",
    Digest = "digest",
    /** The document isn't the one the signed payload names. */
    Document = "document",
}

export class PayloadMismatchError extends Error {
    readonly problem: PayloadProblem

    constructor(problem: PayloadProblem, message: string) {
        super(message)
        // The portal compiles to ES5, where an Error subclass loses its prototype: restore it
        // so `instanceof PayloadMismatchError` holds in the production build.
        Object.setPrototypeOf(this, PayloadMismatchError.prototype)
        this.name = "PayloadMismatchError"
        this.problem = problem
    }
}

/** The payload's `domain` (design §2). */
export const SIGNING_DOMAIN = "step-signing/v1"

const isRecord = (value: unknown): value is Record<string, unknown> =>
    !!value && typeof value === "object" && !Array.isArray(value)

/** Design §2's canonical JSON: sorted keys, no whitespace, no floats. */
export const canonicalJson = (value: unknown): string => {
    if (Array.isArray(value)) {
        return `[${value.map(canonicalJson).join(",")}]`
    }
    if (isRecord(value)) {
        return `{${Object.keys(value)
            .sort()
            .map((key) => `${JSON.stringify(key)}:${canonicalJson(value[key])}`)
            .join(",")}}`
    }
    if (typeof value === "number" && !Number.isInteger(value)) {
        throw new PayloadMismatchError(PayloadProblem.NotCanonical, "a float in the payload")
    }
    return JSON.stringify(value)
}

/** How a signed value reads in the details table. */
export const displayValue = (value: unknown): string => {
    if (value === null || value === undefined) return ""
    if (Array.isArray(value)) return value.map(displayValue).join(", ")
    if (isRecord(value)) return canonicalJson(value)
    return String(value)
}

const lowerString = (value: unknown): string | null =>
    typeof value === "string" && value ? value.toLowerCase() : null

export interface ISignedRow {
    key: string
    /** The server's label, used when `signing.details.<key>` has no translation. */
    label: string | null
    value: string
    /** The name of what the signed id names (a server label, not signed), when known. */
    name?: string | null
}

/** Subject fields of a trustee's request whose ids the panel names. */
const CEREMONY_FIELDS = ["keys_ceremony_id", "tally_session_id"]
const TRUSTEE_FIELD = "trustee_id"

/** The panel's name for a signed id, only when the panel names that very id. */
const nameOf = (data: ISigningPanelData, key: string, value: unknown): string | null => {
    if (CEREMONY_FIELDS.includes(key) && data.ceremony_id && data.ceremony_id === value) {
        return data.ceremony_name ?? null
    }
    if (key === TRUSTEE_FIELD && data.request.trustee_id && data.request.trustee_id === value) {
        return data.trustee_name ?? null
    }
    return null
}

/** What the approval signs, read from the canonical payload itself. */
export interface ISignedView {
    payload: Record<string, unknown>
    subject: Record<string, unknown>
    /** The document's SHA-256 the payload names (PDF: document_sha256, EML: eml_sha256). */
    documentSha256: string | null
    /** The subject as the details table shows it. */
    rows: ISignedRow[]
}

const mismatch = (message: string) => new PayloadMismatchError(PayloadProblem.Mismatch, message)

/**
 * What you see is what you sign: the dialog shows the canonical payload the
 * approval signs, not fields the server sent beside it. It must be in
 * canonical form, of this domain and about the request shown (id, code,
 * action, tenant, event, Post and country); the document hash on the card is
 * the one it names; and server labels only name values it holds.
 * Throws `PayloadMismatchError`.
 */
export const signedView = (data: ISigningPanelData): ISignedView => {
    const {request} = data
    let payload: unknown
    try {
        payload = JSON.parse(request.canonical_payload)
    } catch {
        payload = undefined
    }
    if (!isRecord(payload)) {
        throw new PayloadMismatchError(PayloadProblem.Unparseable, "the payload is not an object")
    }
    if (payload.domain !== SIGNING_DOMAIN || canonicalJson(payload) !== request.canonical_payload) {
        throw new PayloadMismatchError(
            PayloadProblem.NotCanonical,
            "the payload is not in canonical form"
        )
    }
    const expected: Record<string, unknown> = {
        request_id: request.id,
        code: request.code,
        action: request.action,
        tenant_id: request.tenant_id,
        election_event_id: request.election_event_id,
        election_id: request.election_id ?? null,
        area_id: request.area_id ?? null,
    }
    for (const [field, value] of Object.entries(expected)) {
        if ((payload[field] ?? null) !== value) {
            throw mismatch(`the payload's ${field} is not the request's`)
        }
    }
    const subject = payload.subject
    if (!isRecord(subject)) {
        throw mismatch("the payload has no subject")
    }

    let documentSha256: string | null = null
    const kind = documentKindOf(request)
    if (kind !== DocumentKind.NoDocument) {
        documentSha256 = lowerString(
            kind === DocumentKind.Pdf ? subject.document_sha256 : subject.eml_sha256
        )
        if (!documentSha256) {
            throw new PayloadMismatchError(PayloadProblem.Document, "the payload names no document")
        }
        const shown = lowerString(request.document_sha256)
        if (shown && shown !== documentSha256) {
            throw new PayloadMismatchError(
                PayloadProblem.Document,
                "the payload names another document than the request"
            )
        }
    }

    const labelled: ISignedRow[] = (data.details ?? []).map((detail) => {
        if (!(detail.key in subject) || displayValue(subject[detail.key]) !== detail.value) {
            throw mismatch(`the details row ${detail.key} is not what the payload signs`)
        }
        return {key: detail.key, label: detail.label ?? null, value: detail.value}
    })
    const rest = Object.keys(subject)
        .filter((key) => !labelled.some((row) => row.key === key))
        .map((key) => ({key, label: null, value: displayValue(subject[key])}))
    const rows = [...labelled, ...rest].map((row) => {
        const name = nameOf(data, row.key, subject[row.key])
        return name ? {...row, name} : row
    })
    return {payload, subject, documentSha256, rows}
}

/**
 * The bytes the approval signs: the canonical payload exactly as the server
 * sent it, UTF-8 encoded, once `signedView` accepts it and it hashes to
 * `payload_sha256`. Throws `PayloadMismatchError`.
 */
export const payloadToSign = async (
    data: ISigningPanelData
): Promise<{bytes: Uint8Array<ArrayBuffer>; view: ISignedView}> => {
    const view = signedView(data)
    const bytes = new TextEncoder().encode(
        data.request.canonical_payload
    ) as Uint8Array<ArrayBuffer>
    if (hex(await sha256(bytes)) !== data.request.payload_sha256.toLowerCase()) {
        throw new PayloadMismatchError(
            PayloadProblem.Digest,
            "the canonical payload does not hash to payload_sha256"
        )
    }
    return {bytes, view}
}

/** Checks downloaded document bytes against the hash the signed payload names. */
export const checkDocument = async (bytes: Uint8Array, view: ISignedView): Promise<void> => {
    if (
        !view.documentSha256 ||
        hex(await sha256(bytes as Uint8Array<ArrayBuffer>)) !== view.documentSha256
    ) {
        throw new PayloadMismatchError(
            PayloadProblem.Document,
            "the downloaded document is not the one the payload names"
        )
    }
}

// --- Handover: the note that survives the sign-out -------------------------

export const RESUME_KEY = "signing:resume"

/** A note older than this is ignored even when the request doesn't expire sooner. */
export const RESUME_MAX_AGE_MS = 30 * 60 * 1000

export interface ISigningResume {
    requestId: string
    tenantId: string
    eventId: string
    /** ISO time after which the note is ignored. */
    expiresAt: string
}

/** The note for a request, expiring with it or after 30 minutes, whichever is first. */
export const resumeFor = (request: ISigningRequest, now: Date = new Date()): ISigningResume => {
    const limit = now.getTime() + RESUME_MAX_AGE_MS
    const expires = request.expires_at ? new Date(request.expires_at).getTime() : limit
    return {
        requestId: request.id,
        tenantId: request.tenant_id,
        eventId: request.election_event_id,
        expiresAt: new Date(Math.min(limit, expires)).toISOString(),
    }
}

/** Throws when the storage refuses the write (e.g. storage disabled). */
export const writeResume = (storage: Storage, resume: ISigningResume): void => {
    storage.setItem(RESUME_KEY, JSON.stringify(resume))
}

/**
 * Reads and removes the note. `null` when there is none, it isn't one of
 * ours, it is for another tenant, it has expired, or storage is unavailable.
 */
export const takeResume = (
    storage: Storage,
    {tenantId, now = new Date()}: {tenantId: string | null; now?: Date}
): ISigningResume | null => {
    let raw: string | null
    try {
        raw = storage.getItem(RESUME_KEY)
        if (raw === null) return null
        storage.removeItem(RESUME_KEY)
    } catch {
        return null
    }
    try {
        const value = JSON.parse(raw) as Partial<ISigningResume>
        const expiresAt = typeof value.expiresAt === "string" ? Date.parse(value.expiresAt) : NaN
        if (
            typeof value.requestId === "string" &&
            value.requestId &&
            typeof value.eventId === "string" &&
            value.tenantId === tenantId &&
            expiresAt > now.getTime()
        ) {
            return {
                requestId: value.requestId,
                tenantId: value.tenantId,
                eventId: value.eventId,
                expiresAt: value.expiresAt as string,
            }
        }
    } catch {
        // Not ours: dropped.
    }
    return null
}
