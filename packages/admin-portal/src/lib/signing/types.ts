// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Mirror of sequent-core's `signing` module and of the Harvest signing API.
// Enum values are the kebab-case wire values the server sends and accepts.

import {IPermissions} from "@/types/keycloak"

/** The catalog of protected actions. Each value is an id; its labels are under `signing.actions.<id>`. */
export enum SigningAction {
    InitializeVoting = "initialize-voting",
    OpenVoting = "open-voting",
    CloseVoting = "close-voting",
    GenerateElectionReturns = "generate-election-returns",
    GenerateReports = "generate-reports",
    TransmitResults = "transmit-results",
    ApproveVoter = "approve-voter",
    ApproveConfiguration = "approve-configuration",
    ConfirmKeyShare = "key-ceremony",
    ContributeKeyShare = "tally-key",
}

/** What a request is bound to. Post = election, country = area. */
export enum SigningScope {
    Post = "post",
    PostAndCountry = "post-and-country",
    Event = "event",
    Trustee = "trustee",
}

export enum ExecutionMode {
    Deferred = "deferred",
    Gate = "gate",
}

export enum DocumentKind {
    NoDocument = "no-document",
    Pdf = "pdf",
    Eml = "eml",
}

/** Labelled by `signing.groups.<value>`. */
export enum SigningActionGroup {
    Voting = "voting",
    ResultsAndReports = "results-and-reports",
    Enrollment = "enrollment",
    ConfigurationAndKeys = "configuration-and-keys",
}

export enum SigningRequirement {
    NotRequired = "not-required",
    Required = "required",
}

export enum RequesterSigning {
    Allowed = "allowed",
    NotAllowed = "not-allowed",
}

/** Labelled by `signing.status.<value>`. */
export enum SigningRequestStatus {
    Waiting = "waiting",
    Completed = "completed",
    Executed = "executed",
    Cancelled = "cancelled",
    Expired = "expired",
    Failed = "failed",
}

/** Labelled by `signing.cancelReasons.<value>`. */
export enum CancelReason {
    ByRequester = "by-requester",
    ByOperator = "by-operator",
    RuleChanged = "rule-changed",
    PayloadChanged = "payload-changed",
    Superseded = "superseded",
    CertificateRevoked = "certificate-revoked",
}

export enum RevocationCheck {
    Check = "check",
    DontCheck = "dont-check",
}

export enum CrlUnavailablePolicy {
    Refuse = "refuse",
    AcceptUnchecked = "accept-unchecked",
}

export enum CertificateRegistration {
    OnFirstUse = "on-first-use",
    SecurityOfficerOnly = "security-officer-only",
}

export enum CertificatePostBinding {
    OnePost = "one-post",
    AnyPost = "any-post",
}

export enum StaffCertificateStatus {
    Active = "active",
    Revoked = "revoked",
}

/** How a staff certificate was registered. */
export enum StaffCertificateRegistration {
    FirstUse = "first-use",
    SecurityOfficer = "security-officer",
}

export enum CertificateAuthorityPurpose {
    VoterSignIn = "voter-sign-in",
    StaffSignatures = "staff-signatures",
}

export enum SignatureAlgorithm {
    RsaPkcs1Sha256 = "rsa-pkcs1-sha256",
    EcdsaP256Sha256 = "ecdsa-p256-sha256",
}

export enum RevocationStatus {
    Checked = "checked",
    Unchecked = "unchecked",
}

export enum CrlStatus {
    Ok = "ok",
    Unavailable = "unavailable",
}

/** Why a certificate file didn't open; logged without the file or its password. */
export enum CertificateOpenFailure {
    WrongPassword = "wrong-password",
    Unreadable = "unreadable",
    NoKey = "no-key",
}

/** The server's certificate checks, in dialog order. Labelled by `signing.dialog.checks.passed.<value>` and `signing.dialog.checks.failed.<value>`. */
export enum CertificateCheckId {
    TrustedIssuer = "trusted-issuer",
    ValidNow = "valid-now",
    SigningKeyUsage = "signing-key-usage",
    NotRevoked = "not-revoked",
    Registered = "registered",
    RegisteredToOther = "registered-to-other",
    AlreadySigned = "already-signed",
    PostBinding = "post-binding",
    Signature = "signature",
}

export interface ISigningActionInfo {
    group: SigningActionGroup
    scope: SigningScope
    mode: ExecutionMode
    document: DocumentKind
    signPermission: IPermissions
}

/** The same mapping as `SigningAction`'s helpers in sequent-core, in catalog order. */
export const SIGNING_ACTIONS: Record<SigningAction, ISigningActionInfo> = {
    [SigningAction.InitializeVoting]: {
        group: SigningActionGroup.Voting,
        scope: SigningScope.Post,
        mode: ExecutionMode.Deferred,
        document: DocumentKind.NoDocument,
        signPermission: IPermissions.SIGN_INITIALIZE_VOTING,
    },
    [SigningAction.OpenVoting]: {
        group: SigningActionGroup.Voting,
        scope: SigningScope.Post,
        mode: ExecutionMode.Deferred,
        document: DocumentKind.NoDocument,
        signPermission: IPermissions.SIGN_OPEN_VOTING,
    },
    [SigningAction.CloseVoting]: {
        group: SigningActionGroup.Voting,
        scope: SigningScope.Post,
        mode: ExecutionMode.Deferred,
        document: DocumentKind.NoDocument,
        signPermission: IPermissions.SIGN_CLOSE_VOTING,
    },
    [SigningAction.GenerateElectionReturns]: {
        group: SigningActionGroup.ResultsAndReports,
        scope: SigningScope.PostAndCountry,
        mode: ExecutionMode.Deferred,
        document: DocumentKind.Pdf,
        signPermission: IPermissions.SIGN_GENERATE_ELECTION_RETURNS,
    },
    [SigningAction.GenerateReports]: {
        group: SigningActionGroup.ResultsAndReports,
        scope: SigningScope.Post,
        mode: ExecutionMode.Deferred,
        document: DocumentKind.Pdf,
        signPermission: IPermissions.SIGN_GENERATE_REPORTS,
    },
    [SigningAction.TransmitResults]: {
        group: SigningActionGroup.ResultsAndReports,
        scope: SigningScope.PostAndCountry,
        mode: ExecutionMode.Deferred,
        document: DocumentKind.Eml,
        signPermission: IPermissions.SIGN_TRANSMIT_RESULTS,
    },
    [SigningAction.ApproveVoter]: {
        group: SigningActionGroup.Enrollment,
        scope: SigningScope.Post,
        mode: ExecutionMode.Deferred,
        document: DocumentKind.NoDocument,
        signPermission: IPermissions.SIGN_APPROVE_VOTER,
    },
    [SigningAction.ApproveConfiguration]: {
        group: SigningActionGroup.ConfigurationAndKeys,
        scope: SigningScope.Event,
        mode: ExecutionMode.Deferred,
        document: DocumentKind.NoDocument,
        signPermission: IPermissions.SIGN_APPROVE_CONFIGURATION,
    },
    [SigningAction.ConfirmKeyShare]: {
        group: SigningActionGroup.ConfigurationAndKeys,
        scope: SigningScope.Trustee,
        mode: ExecutionMode.Gate,
        document: DocumentKind.NoDocument,
        signPermission: IPermissions.SIGN_KEY_CEREMONY,
    },
    [SigningAction.ContributeKeyShare]: {
        group: SigningActionGroup.ConfigurationAndKeys,
        scope: SigningScope.Trustee,
        mode: ExecutionMode.Gate,
        document: DocumentKind.NoDocument,
        signPermission: IPermissions.SIGN_TALLY_KEY,
    },
}

/** The expiries the rule drawer offers, in minutes; `null` is no limit. */
export const SIGNING_EXPIRY_OPTIONS: ReadonlyArray<number | null> = [30, 60, 120, 1440, null]

export interface ISigningRule {
    action: SigningAction
    requirement: SigningRequirement
    /** Different people who must sign, as configured; a trustee action always needs 1. */
    signatures: number
    requester_signing: RequesterSigning
    /** `null`: requests never expire. Absent on import: 60. */
    expires_minutes: number | null
    revision: number
}

export interface ISigningChecks {
    revocation_check: RevocationCheck
    crl_unavailable: CrlUnavailablePolicy
    registration: CertificateRegistration
    post_binding: CertificatePostBinding
    revision: number
}

/** What a guarded route answers instead of running the action. */
export interface ISigningRequestSummary {
    id: string
    code: string
    required: number
    expires_at: string | null
}

/** The optional field existing action outputs gain. */
export interface ISigningRequiredOutput {
    signing_request?: ISigningRequestSummary | null
}

export interface ISigningRequest {
    id: string
    tenant_id: string
    election_event_id: string
    action: SigningAction
    election_id: string | null
    area_id: string | null
    trustee_id: string | null
    subject: Record<string, unknown>
    canonical_payload: string
    payload_sha256: string
    document_id: string | null
    document_sha256: string | null
    code: string
    config_revision: string | null
    rule_revision: number
    required: number
    status: SigningRequestStatus
    cancel_reason: CancelReason | null
    cancelled_by: string | null
    requested_by: string
    requested_by_username: string
    /** The requester's first and last name when they started it. */
    requested_by_name?: string | null
    created_at: string
    expires_at: string | null
    completed_at: string | null
    executed_at: string | null
    execution_result: Record<string, unknown> | null
    /** Certificate files that didn't open in a signer's browser. */
    open_failures?: number
}

/** A signer's status on a request. */
export enum SigningSignerStatus {
    Signed = "signed",
    NotSigned = "not-signed",
}

/** A person who can sign a request, with their status on it. */
export interface ISigningSigner {
    user_id: string
    username: string
    /** First and last name; the username without one. */
    display_name: string
    /** The Keycloak `title` attribute, falling back to the group name. */
    title: string | null
    /** The viewer ("(you)"). */
    is_you?: boolean
    status?: SigningSignerStatus
    signed_at: string | null
    certificate_subject: string | null
    /** The CN of the certificate they signed with. */
    certificate_cn?: string | null
}

/** A labelled value of what a request signs: one per field of its subject. */
export interface ISigningRequestDetail {
    key: string
    value: string
}

/** `POST /signing-requests/get` (`signingGetRequest`): everything the panel shows. */
export interface ISigningRequestPanel {
    request: ISigningRequest
    /** The rule as it was when the request started. */
    rule: ISigningRule
    count: number
    /** Signatures needed (the request's, from its rule when it started). */
    required?: number
    signers: ISigningSigner[]
    document_url: string | null
    document_name?: string | null
    document_pages?: number | null
    details?: ISigningRequestDetail[] | null
    /** The Post's name. */
    election_name?: string | null
    /** The country's name. */
    area_name?: string | null
}

export interface ICertificateCheckResult {
    id: CertificateCheckId
    ok: boolean
    detail: string | null
}

/** `POST /signing-requests/<id>/check-certificate`. */
export interface ICheckCertificateInput {
    chain_pem: string[]
}

export interface ICheckCertificateOutput {
    checks: ICertificateCheckResult[]
}

/** `POST /signing-requests/<id>/pdf-prepare`. */
export interface IPdfPrepareInput {
    chain_pem: string[]
}

export interface IPdfPrepareOutput {
    revision: number
    digest_b64: string
    signing_time: string
}

/** `POST /signing-requests/<id>/approve`. */
export interface IApproveSigningRequestInput {
    chain_pem: string[]
    algorithm: SignatureAlgorithm
    payload_signature_b64: string
    document_signature_b64?: string
    pdf_cms_b64?: string
    revision?: number
}

export interface IApproveSigningRequestOutput {
    status: SigningRequestStatus
    count: number
    required: number
}

/** `POST /signing-requests/<id>/open-failures`: never the file or its password. */
export interface IOpenFailureInput {
    file_name: string
    reason: CertificateOpenFailure
}

/** `POST /signing-requests/<id>/cancel`. */
export interface ICancelSigningRequestInput {
    reason?: string
}

/** `POST /signing-rules/put` (`signingPutRule`). */
export interface IPutSigningRuleInput {
    election_event_id: string
    action: SigningAction
    requirement: SigningRequirement
    signatures: number
    requester_signing: RequesterSigning
    expires_minutes: number | null
    /** Group ids gaining or losing `sign-<action>`; needs role-read and role-write. */
    roles?: {add: string[]; remove: string[]}
    expected_revision: number
}

/** What saving a rule answers. */
export interface ISaveSigningRuleOutput {
    /** The rule's new revision. */
    revision: number
    rule: ISigningRule
    /** The Posts with fewer people who can sign than the rule needs. */
    short_posts: Array<{election_id: string; name: string; count: number}>
    warnings: SigningRuleWarning[]
    /** The waiting requests the save cancelled. */
    cancelled: string[]
}

/** `POST /signing-rules/capacity` (`signingRuleCapacity`): eligible signers per Post. */
export interface ISigningRuleCapacity {
    max: number
    posts: Array<{election_id: string; name: string; count: number}>
    /** The Posts with fewer signers than the number asked for (the saved rule's by default). */
    posts_short: Array<{election_id: string; name: string; count: number}>
    /** When the requester can't sign: the Posts where the others are fewer than needed. */
    posts_short_without_requester: Array<{election_id: string; name: string; count: number}>
    /** The groups that hold the action's sign permission. */
    roles: Array<{id: string; name: string; path: string}>
    /** The action's waiting requests, which saving the rule cancels. */
    waiting: number
    /** The event's configuration version: its published event-level ballot publications. */
    config_version: number
}

/** What a saved rule should know about the Posts. */
export enum SigningRuleWarning {
    NoPosts = "no-posts",
    ShortPosts = "short-posts",
    RequesterExcluded = "requester-excluded",
}

/** `POST /signing-requests/export` (`signingExportRequests`): the CSV as a document. */
export interface ISigningRequestsExport {
    document_id: string
    sha256: string
    rows: number
}

/** `PUT /signing-checks`. */
export interface IPutSigningChecksInput {
    revocation_check: RevocationCheck
    crl_unavailable: CrlUnavailablePolicy
    registration: CertificateRegistration
    post_binding: CertificatePostBinding
    expected_revision: number
}

/** `POST /staff-certificates`. */
export interface IRegisterStaffCertificateInput {
    user_id: string
    election_id?: string | null
    pem: string
}

/** `POST /staff-certificates/<id>/revoke`. */
export interface IRevokeStaffCertificateInput {
    reason: string
}

export interface IStaffCertificate {
    id: string
    user_id: string
    username: string
    election_id: string | null
    fingerprint_sha256: string
    spki_sha256: string
    holder_sha256: string
    serial: string
    subject: string
    issuer: string
    not_before: string
    not_after: string
    status: StaffCertificateStatus
    registration: StaffCertificateRegistration
    registered_by: string | null
    registered_at: string
    revoked_by: string | null
    revoked_at: string | null
    revoke_reason: string | null
}
