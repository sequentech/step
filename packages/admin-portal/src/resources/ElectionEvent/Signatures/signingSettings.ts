// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// What the Election Event > Signatures tab decides without the network: who may
// see and change each part, the rule an action has without a row, whether a
// number of signatures is possible, what a drawer save sends, and how
// certificates and requests are shown and filtered.

import {shownRequestStatus} from "@/lib/signing/status"
import {IPermissions} from "@/types/keycloak"
import {
    CertificatePostBinding,
    CertificateRegistration,
    CrlUnavailablePolicy,
    ExecutionMode,
    MAX_SIGNATURES,
    RequesterSigning,
    RevocationCheck,
    SIGNING_ACTIONS,
    SIGNING_EXPIRY_OPTIONS,
    SigningAction,
    SigningActionGroup,
    SigningRequestStatus,
    SigningRequirement,
    StaffCertificateStatus,
    type IPutSigningRuleInput,
    type ISigningChecks,
    type ISigningRequestListRow,
    type ISigningRule,
    type ISigningRuleCapacity,
    type ISigningRuleRow,
    type IStaffCertificate,
} from "@/lib/signing/types"

export interface ISignaturesAccess {
    /** The tab: its own permission and at least one sub-tab's read permission. */
    tab: boolean
    rulesRead: boolean
    rulesWrite: boolean
    /**
     * Who can sign changes from the drawer: the options come from /get-roles
     * (role-read), the change needs role-write, and it is saved with the rule.
     */
    rolesWrite: boolean
    certificatesRead: boolean
    issuersWrite: boolean
    checksWrite: boolean
    certificatesRegister: boolean
    certificatesRevoke: boolean
    requestsRead: boolean
    requestsCancel: boolean
    requestsExport: boolean
}

export function signaturesAccess(has: (permission: IPermissions) => boolean): ISignaturesAccess {
    const rulesRead = has(IPermissions.SIGNING_RULES_READ)
    const certificatesRead = has(IPermissions.SIGNING_CERTIFICATES_READ)
    const requestsRead = has(IPermissions.SIGNING_REQUESTS_READ)
    const rulesWrite = has(IPermissions.SIGNING_RULES_WRITE)
    const issuersWrite = has(IPermissions.SIGNING_ISSUERS_WRITE)
    const checksWrite = has(IPermissions.SIGNING_CHECKS_WRITE)
    const certificatesRegister = has(IPermissions.SIGNING_CERTIFICATES_REGISTER)
    const certificatesRevoke = has(IPermissions.SIGNING_CERTIFICATES_REVOKE)
    return {
        tab:
            has(IPermissions.ELECTION_EVENT_SIGNATURES_TAB) &&
            (rulesRead || certificatesRead || requestsRead),
        rulesRead,
        rulesWrite,
        rolesWrite: rulesWrite && has(IPermissions.ROLE_READ) && has(IPermissions.ROLE_WRITE),
        certificatesRead,
        issuersWrite,
        checksWrite,
        certificatesRegister,
        certificatesRevoke,
        requestsRead,
        requestsCancel: has(IPermissions.SIGNING_REQUESTS_CANCEL),
        requestsExport: has(IPermissions.SIGNING_REQUESTS_EXPORT),
    }
}

/**
 * After lockdown the rules belong to the configuration version: changing them
 * goes through a new one, so the rules and who signs them are read-only.
 */
export const afterLockdown = (access: ISignaturesAccess, lockedDown: boolean): ISignaturesAccess =>
    lockedDown ? {...access, rulesWrite: false, rolesWrite: false} : access

/** The rule of an action without a row: no signatures needed (design §1). */
export const defaultRule = (action: SigningAction): ISigningRule => ({
    action,
    requirement: SigningRequirement.NotRequired,
    signatures: 1,
    requester_signing: RequesterSigning.NotAllowed,
    expires_minutes: 60,
    revision: 0,
})

export const ruleOf = <R extends ISigningRule>(
    action: SigningAction,
    rules: ReadonlyArray<R>
): R | ISigningRule => rules.find((rule) => rule.action === action) ?? defaultRule(action)

/** The checks of an event without a row (design §1). */
export const DEFAULT_CHECKS: ISigningChecks = {
    revocation_check: RevocationCheck.Check,
    crl_unavailable: CrlUnavailablePolicy.Refuse,
    registration: CertificateRegistration.OnFirstUse,
    post_binding: CertificatePostBinding.OnePost,
    revision: 0,
}

/** Trustee actions: each trustee signs their own step; the rule only switches on or off. */
export const isTrusteeAction = (action: SigningAction) =>
    SIGNING_ACTIONS[action].mode === ExecutionMode.Gate

/** The catalog's groups, each with its actions, in catalog order. */
export const actionGroups = (): Array<{group: SigningActionGroup; actions: SigningAction[]}> =>
    Object.values(SigningActionGroup).map((group) => ({
        group,
        actions: Object.values(SigningAction).filter(
            (action) => SIGNING_ACTIONS[action].group === group
        ),
    }))

/** The key under `signing.expiry` that labels an expiry; `other` for a value the drawer doesn't offer. */
export const expiryKey = (minutes: number | null): string =>
    SIGNING_EXPIRY_OPTIONS.includes(minutes) ? String(minutes ?? "none") : "other"

/** The most recent save among the rules, and the name of who saved it when it was stored. */
export function lastChange(
    rules: ReadonlyArray<Pick<ISigningRuleRow, "updated_at" | "updated_by_name">>
): {name: string | null; updated_at: string} | null {
    return rules.reduce<{name: string | null; updated_at: string} | null>(
        (latest, {updated_by_name, updated_at}) =>
            !latest || Date.parse(updated_at) > Date.parse(latest.updated_at)
                ? {name: updated_by_name ?? null, updated_at}
                : latest,
        null
    )
}

export enum SignaturesProblem {
    AtLeastOne = "at-least-one",
    /** More than the Post with most signers (or the event) can give. */
    TooMany = "too-many",
    /** More than any rule may need (`MAX_SIGNATURES`, the server's own maximum). */
    OutOfRange = "out-of-range",
}

/** Posts with the same number of signers, all fewer than the number asked for. */
export interface IShortPosts {
    signers: number
    posts: string[]
}

export interface ISignaturesCheck {
    problem: SignaturesProblem | null
    /** The most signers any Post has; unknown without the capacity. */
    max: number | null
    shortPosts: IShortPosts[]
    /**
     * When the person who starts a request can't sign: the Posts that are short
     * without them, by how many others can sign (Posts short anyway aren't repeated).
     */
    requesterShort: IShortPosts[]
    /** The fewest signers any Post has: every Post can give this many. */
    minSigners: number | null
}

/** The number typed in Signatures needed, or null when it isn't a whole number. */
export function parseSignatures(text: string): number | null {
    const trimmed = text.trim()
    return /^-?\d+$/.test(trimmed) ? Number(trimmed) : null
}

/**
 * Checks a number of signatures against what the Posts can give (design §5).
 * The capacity is of the saved roles: when the draft changes them, only the
 * range is checked here and the server checks the rest on save.
 */
export function checkSignatures(
    signatures: number | null,
    savedCapacity: ISigningRuleCapacity | null | undefined,
    {
        rolesChanged = false,
        requesterExcluded = false,
    }: {rolesChanged?: boolean; requesterExcluded?: boolean} = {}
): ISignaturesCheck {
    const capacity = rolesChanged ? null : savedCapacity
    const max = capacity?.max ?? null
    const posts = capacity?.posts ?? []
    const minSigners = capacity
        ? posts.length
            ? Math.min(...posts.map(({count}) => count))
            : capacity.max
        : null
    if (signatures === null || signatures < 1) {
        return {
            problem: SignaturesProblem.AtLeastOne,
            max,
            shortPosts: [],
            requesterShort: [],
            minSigners,
        }
    }
    if (signatures > MAX_SIGNATURES) {
        return {
            problem: SignaturesProblem.OutOfRange,
            max: MAX_SIGNATURES,
            shortPosts: [],
            requesterShort: [],
            minSigners,
        }
    }
    if (max !== null && signatures > max) {
        return {
            problem: SignaturesProblem.TooMany,
            max,
            shortPosts: [],
            requesterShort: [],
            minSigners,
        }
    }
    const shortPosts: IShortPosts[] = []
    const requesterShort: IShortPosts[] = []
    const add = (groups: IShortPosts[], signers: number, name: string) => {
        const group = groups.find((entry) => entry.signers === signers)
        if (group) group.posts.push(name)
        else groups.push({signers, posts: [name]})
    }
    for (const {name, count: signers} of posts) {
        if (signers < signatures) add(shortPosts, signers, name)
        else if (requesterExcluded && signers - 1 < signatures)
            add(requesterShort, signers - 1, name)
    }
    shortPosts.sort((a, b) => a.signers - b.signers)
    requesterShort.sort((a, b) => a.signers - b.signers)
    return {problem: null, max, shortPosts, requesterShort, minSigners}
}

/** What the rule drawer edits. */
export interface IRuleDraft {
    requirement: SigningRequirement
    /** As typed, so an empty or partial number can be shown and refused. */
    signatures: string
    requesterSigning: RequesterSigning
    expiresMinutes: number | null
    /** The ids of the roles (groups) holding the action's sign permission. */
    roles: string[]
}

export const draftOf = (rule: ISigningRule, roles: ReadonlyArray<string>): IRuleDraft => ({
    requirement: rule.requirement,
    signatures: String(rule.signatures),
    requesterSigning: rule.requester_signing,
    expiresMinutes: rule.expires_minutes,
    roles: [...roles],
})

const sameSet = (a: ReadonlyArray<string>, b: ReadonlyArray<string>) =>
    a.length === b.length && a.every((value) => b.includes(value))

/** The typed number, or the text when it isn't one, so "03" equals "3" but "" differs. */
const typedSignatures = (text: string) => parseSignatures(text) ?? text.trim()

export const rolesChanged = (initial: IRuleDraft, draft: IRuleDraft) =>
    !sameSet(initial.roles, draft.roles)

export const draftChanged = (initial: IRuleDraft, draft: IRuleDraft) =>
    initial.requirement !== draft.requirement ||
    typedSignatures(initial.signatures) !== typedSignatures(draft.signatures) ||
    initial.requesterSigning !== draft.requesterSigning ||
    initial.expiresMinutes !== draft.expiresMinutes ||
    !sameSet(initial.roles, draft.roles)

/** The body of `PUT /signing-rules/<action>` for a draft of the rule; roles are group ids. */
export function ruleInputOf(
    electionEventId: string,
    rule: ISigningRule,
    draft: IRuleDraft,
    initialRoles: ReadonlyArray<string>
): IPutSigningRuleInput {
    const trustee = isTrusteeAction(rule.action)
    const signatures = parseSignatures(draft.signatures)
    const input: IPutSigningRuleInput = {
        election_event_id: electionEventId,
        action: rule.action,
        requirement: draft.requirement,
        signatures: trustee
            ? 1
            : signatures !== null && signatures >= 1
              ? signatures
              : rule.signatures,
        requester_signing: trustee ? RequesterSigning.Allowed : draft.requesterSigning,
        expires_minutes: trustee ? rule.expires_minutes : draft.expiresMinutes,
        expected_revision: rule.revision,
    }
    // An off rule has no signers to choose.
    if (draft.requirement === SigningRequirement.NotRequired) return input
    const add = draft.roles.filter((role) => !initialRoles.includes(role))
    const remove = initialRoles.filter((role) => !draft.roles.includes(role))
    return add.length || remove.length ? {...input, roles: {add, remove}} : input
}

/** How a registered certificate is shown; "Expires soon" is under 30 days. */
export enum CertificateDisplayStatus {
    Active = "active",
    ExpiresSoon = "expires-soon",
    Expired = "expired",
    Revoked = "revoked",
}

export const EXPIRES_SOON_MS = 30 * 24 * 60 * 60 * 1000

export function certificateStatus(
    certificate: Pick<IStaffCertificate, "status" | "not_after">,
    now: Date
): CertificateDisplayStatus {
    if (certificate.status === StaffCertificateStatus.Revoked) {
        return CertificateDisplayStatus.Revoked
    }
    const left = Date.parse(certificate.not_after) - now.getTime()
    if (left <= 0) return CertificateDisplayStatus.Expired
    if (left < EXPIRES_SOON_MS) return CertificateDisplayStatus.ExpiresSoon
    return CertificateDisplayStatus.Active
}

type ListedCertificate = Pick<
    IStaffCertificate,
    | "username"
    | "user_display_name"
    | "subject"
    | "issuer"
    | "fingerprint_sha256"
    | "election_id"
    | "status"
    | "not_after"
>

export function filterCertificates<C extends ListedCertificate>(
    certificates: ReadonlyArray<C>,
    {search, status}: {search: string; status: CertificateDisplayStatus | null},
    now: Date,
    postName: (electionId: string | null) => string
): C[] {
    const needle = search.trim().toLowerCase()
    return certificates.filter((certificate) => {
        if (status && certificateStatus(certificate, now) !== status) return false
        if (!needle) return true
        return [
            certificate.username,
            certificate.user_display_name ?? "",
            certificate.subject,
            certificate.issuer,
            certificate.fingerprint_sha256,
            postName(certificate.election_id),
        ].some((value) => value.toLowerCase().includes(needle))
    })
}

/** The CN of a distinguished name (RFC 4514 or OpenSSL's slash form), or the name itself. */
export function commonName(dn: string): string {
    const match = /(?:^|[,/+]\s*)CN=((?:\\.|[^,/+])*)/.exec(dn)
    return match ? match[1].trim() : dn
}

/** "F7:08:0F:24:…:FE:78": the first four and last two bytes of a hex fingerprint. */
export function shortFingerprint(hex: string | null | undefined): string {
    const bytes =
        (hex ?? "")
            .replace(/[^0-9a-f]/gi, "")
            .toUpperCase()
            .match(/.{2}/g) ?? []
    if (bytes.length <= 6) return bytes.join(":")
    return [...bytes.slice(0, 4), "…", ...bytes.slice(-2)].join(":")
}

const PEM_HEADER = "-----BEGIN CERTIFICATE-----"

const base64 = (bytes: Uint8Array) => {
    let binary = ""
    for (let index = 0; index < bytes.length; index += 1) {
        binary += String.fromCharCode(bytes[index])
    }
    return btoa(binary)
}

/** An issuer file as `signingImportIssuers` takes it: PEM text, or a .cer/.der's bytes in base64. */
export function issuerUpload(bytes: Uint8Array): {pem: string | null; der_base64: string | null} {
    const text = toPemText(bytes)
    return text ? {pem: text, der_base64: null} : {pem: null, der_base64: base64(bytes)}
}

/** PEM text with OpenSSL's older "X509 CERTIFICATE" label renamed, or null for binary DER. */
const toPemText = (bytes: Uint8Array): string | null => {
    const text = new TextDecoder()
        .decode(bytes)
        .replace(/(-----(?:BEGIN|END)) X509 CERTIFICATE-----/g, "$1 CERTIFICATE-----")
    return text.includes(PEM_HEADER) ? text : null
}

/**
 * A certificate file's contents as PEM: PEM text as it is (OpenSSL's older
 * "X509 CERTIFICATE" label renamed), DER (.cer) bytes wrapped.
 */
export function toPem(bytes: Uint8Array): string {
    const text = toPemText(bytes)
    if (text) return text
    const lines = base64(bytes).match(/.{1,64}/g) ?? []
    return `${PEM_HEADER}\n${lines.join("\n")}\n-----END CERTIFICATE-----\n`
}

/** The status a request shows, as the signing panel shows it. */
export const requestStatus = shownRequestStatus

export const filterRequests = <R extends Pick<ISigningRequestListRow, "status" | "expires_at">>(
    requests: ReadonlyArray<R>,
    status: SigningRequestStatus | null,
    now: Date
): R[] => requests.filter((request) => !status || requestStatus(request, now) === status)

export function lastSignature(
    request: Pick<ISigningRequestListRow, "approvals">
): ISigningRequestListRow["approvals"][number] | null {
    return request.approvals.reduce<ISigningRequestListRow["approvals"][number] | null>(
        (latest, approval) =>
            !latest || Date.parse(approval.signed_at) > Date.parse(latest.signed_at)
                ? approval
                : latest,
        null
    )
}

/** A person's display name, or their username when the row has none. */
export const personName = (name: string | null | undefined, username: string) =>
    name?.trim() ? name : username

/** The DER bytes of the first certificate of a PEM, or null when it has none. */
export function pemToDer(pem: string): Uint8Array<ArrayBuffer> | null {
    const body = /-----BEGIN CERTIFICATE-----([^-]*)-----END CERTIFICATE-----/.exec(pem)?.[1]
    if (!body) return null
    try {
        const binary = atob(body.replace(/\s/g, ""))
        return Uint8Array.from(binary, (character) => character.charCodeAt(0))
    } catch {
        return null
    }
}

/** The certificate's `fingerprint_sha256`: SHA-256 of its DER, lowercase hex. */
export async function certificateFingerprint(pem: string): Promise<string | null> {
    const der = pemToDer(pem)
    if (!der) return null
    const digest = await crypto.subtle.digest("SHA-256", der)
    return Array.from(new Uint8Array(digest), (byte) => byte.toString(16).padStart(2, "0")).join("")
}

/**
 * The account to link a registration to (decided O1): the one an active
 * registration of the same certificate belongs to, or the first account it
 * already links.
 */
export function linkTarget(
    certificates: ReadonlyArray<
        Pick<IStaffCertificate, "user_id" | "linked_to" | "status" | "fingerprint_sha256">
    >,
    fingerprint: string | null
): string | null {
    const holder = certificates.find(
        (certificate) =>
            certificate.status === StaffCertificateStatus.Active &&
            certificate.fingerprint_sha256 === fingerprint
    )
    return holder ? (holder.linked_to ?? holder.user_id) : null
}

/** The error codes of the signing routes (api-contract.md), as the portal handles them. */
export enum SigningErrorCode {
    Forbidden = "forbidden",
    Invalid = "invalid",
    /** A stale expected revision: someone else saved first. */
    Conflict = "conflict",
    NotFound = "not-found",
    /** A certificate check refused: `check` names it. */
    Refused = "signing-refused",
    /** The event is locked down (PR 5 sends it as `invalid`; a distinct code is also read). */
    LockedDown = "locked-down",
    Other = "other",
}

const CODES: Record<string, SigningErrorCode> = {
    "forbidden": SigningErrorCode.Forbidden,
    "invalid": SigningErrorCode.Invalid,
    "conflict": SigningErrorCode.Conflict,
    "stale-revision": SigningErrorCode.Conflict,
    "not-found": SigningErrorCode.NotFound,
    "signing-refused": SigningErrorCode.Refused,
    "already-signed": SigningErrorCode.Refused,
    "locked-down": SigningErrorCode.LockedDown,
}

const STATUSES: Record<number, SigningErrorCode> = {
    400: SigningErrorCode.Invalid,
    403: SigningErrorCode.Forbidden,
    404: SigningErrorCode.NotFound,
    409: SigningErrorCode.Conflict,
    422: SigningErrorCode.Refused,
}

const objectOf = (value: unknown): Record<string, unknown> =>
    value && typeof value === "object" ? (value as Record<string, unknown>) : {}

export interface ISigningError {
    code: SigningErrorCode
    check: string | null
    /** A registered-to-other refusal: the account holding the key or holder. */
    holder?: {userId: string; name: string | null}
}

/** Classifies a failed action by `extensions.code`, falling back to the HTTP status. */
export function signingError(error: unknown): ISigningError {
    const errors = objectOf(error).graphQLErrors
    for (const graphQLError of Array.isArray(errors) ? errors : []) {
        const extensions = objectOf(objectOf(graphQLError).extensions)
        const check = typeof extensions.check === "string" ? extensions.check : null
        const holder =
            typeof extensions.user_id === "string"
                ? {
                      userId: extensions.user_id,
                      name:
                          typeof extensions.display_name === "string"
                              ? extensions.display_name
                              : null,
                  }
                : undefined
        const code = typeof extensions.code === "string" ? CODES[extensions.code] : undefined
        if (code) return holder ? {code, check, holder} : {code, check}
        const status = objectOf(objectOf(extensions.internal).response).status
        if (typeof status === "number" && STATUSES[status]) return {code: STATUSES[status], check}
    }
    return {code: SigningErrorCode.Other, check: null}
}

/**
 * The message of a refused write, or null for the write's own fallback. The
 * server refuses a rule edit of a locked-down event as invalid input.
 */
export function errorKey(code: SigningErrorCode, lockedDown: boolean): string | null {
    switch (code) {
        case SigningErrorCode.Forbidden:
            return "signing.errors.forbidden"
        case SigningErrorCode.Invalid:
            return lockedDown ? "signing.errors.lockedDown" : "signing.errors.invalid"
        case SigningErrorCode.LockedDown:
            return "signing.errors.lockedDown"
        case SigningErrorCode.Conflict:
            return "signing.errors.conflict"
        case SigningErrorCode.NotFound:
            return "signing.errors.notFound"
        default:
            return null
    }
}
