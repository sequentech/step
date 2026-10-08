// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Two synthetic organizations for Election Event > Signatures, so that no label,
// role, Post or rule of the tab can be hard-coded: an overseas voting event
// whose Posts have three signers, and a student council that renames its Posts
// and actions through tenant translation overrides and switches most rules off.
import React, {useState, type PropsWithChildren} from "react"
import {EElectionEventLockedDown, ELogTimeZonePolicy} from "@sequentech/ui-core"
import type {FetchResult, Operation} from "@apollo/client"
import {GraphQLError} from "graphql"
import {RecordContextProvider} from "react-admin"
import {AdminStoryProvider, EVENT_ID, TENANT_ID} from "@/__stories__/AdminStoryProvider"
import type {graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {electionPresentation, eventPresentation, eventRecord, storyId} from "@/__stories__/fixtures"
import {documentHandlers} from "@/__stories__/downloads"
import {applyTenantTranslationOverrides} from "@/providers/TenantContextProvider"
import {IPermissions} from "@/types/keycloak"
import {EventTimeZoneProvider} from "@/providers/EventTimeZoneProvider"
import {
    CertificatePostBinding,
    CertificateRegistration,
    CrlStatus,
    CrlUnavailablePolicy,
    RequesterSigning,
    RevocationCheck,
    SIGNING_ACTIONS,
    SigningAction,
    SigningRequestStatus,
    SigningRequirement,
    StaffCertificateRegistration,
    StaffCertificateStatus,
    CancelReason,
    type ISigningChecksRow,
    type ISigningRequestListRow,
    type ISigningRuleCapacity,
    type ISigningRuleRow,
    type IStaffCertificate,
    type IStaffCrl,
    type IStaffIssuer,
} from "@/lib/signing/types"
import {recordsOrPending} from "../../__stories__/ElectionEventFixture"
import {capacityAlias} from "../useSigningSettings"
import {SigningProvider} from "@/components/signing/SigningProvider"
import type {ISigningApi} from "@/lib/signing/api"
import {pending} from "../../../../../../ui-essentials/.storybook/screens"

type Handler = (operation: Operation) => FetchResult | Promise<FetchResult>

const TAB = IPermissions.ELECTION_EVENT_SIGNATURES_TAB

/**
 * The permissions of each role of the sample preset, exactly as the ticket's
 * table gives them, and single-permission roles that prove each gate.
 */
export const SIGNING_ROLES = {
    configurationManager: [TAB, IPermissions.SIGNING_RULES_READ, IPermissions.SIGNING_RULES_WRITE],
    securityOfficer: [
        TAB,
        IPermissions.SIGNING_CERTIFICATES_READ,
        IPermissions.SIGNING_ISSUERS_WRITE,
        IPermissions.SIGNING_CHECKS_WRITE,
        IPermissions.SIGNING_CERTIFICATES_REGISTER,
        IPermissions.SIGNING_CERTIFICATES_REVOKE,
        IPermissions.ROLE_WRITE,
    ],
    ofov: [
        TAB,
        IPermissions.SIGNING_REQUESTS_READ,
        IPermissions.SIGNING_REQUESTS_CANCEL,
        IPermissions.SIGNING_REQUESTS_EXPORT,
    ],
    auditor: [
        TAB,
        IPermissions.SIGNING_RULES_READ,
        IPermissions.SIGNING_CERTIFICATES_READ,
        IPermissions.SIGNING_REQUESTS_READ,
        IPermissions.SIGNING_REQUESTS_EXPORT,
    ],
    /** Edits the rules and administers roles: Who can sign is editable. */
    rulesAndRoles: [
        TAB,
        IPermissions.SIGNING_RULES_READ,
        IPermissions.SIGNING_RULES_WRITE,
        IPermissions.ROLE_READ,
        IPermissions.ROLE_WRITE,
    ],
    /** Administers roles without editing the rules: the drawer stays read-only. */
    rolesWithoutRules: [
        TAB,
        IPermissions.SIGNING_RULES_READ,
        IPermissions.ROLE_READ,
        IPermissions.ROLE_WRITE,
    ],
    /** Edits the rules and roles but can't list the roles (no role-read). */
    rulesAndRoleWriteOnly: [
        TAB,
        IPermissions.SIGNING_RULES_READ,
        IPermissions.SIGNING_RULES_WRITE,
        IPermissions.ROLE_WRITE,
    ],
    issuersOnly: [TAB, IPermissions.SIGNING_CERTIFICATES_READ, IPermissions.SIGNING_ISSUERS_WRITE],
    checksOnly: [TAB, IPermissions.SIGNING_CERTIFICATES_READ, IPermissions.SIGNING_CHECKS_WRITE],
    registerOnly: [
        TAB,
        IPermissions.SIGNING_CERTIFICATES_READ,
        IPermissions.SIGNING_CERTIFICATES_REGISTER,
    ],
    revokeOnly: [
        TAB,
        IPermissions.SIGNING_CERTIFICATES_READ,
        IPermissions.SIGNING_CERTIFICATES_REVOKE,
    ],
    /** Reads requests, without exporting or cancelling them. */
    requestsReader: [TAB, IPermissions.SIGNING_REQUESTS_READ],
    /** The tab permission without any sub-tab's read permission. */
    tabOnly: [TAB],
} satisfies Record<string, IPermissions[]>

export type SigningRole = keyof typeof SIGNING_ROLES

export interface ISigningOrganization {
    posts: Array<{id: string; name: string}>
    /** People a certificate can be registered to. */
    people: Array<{id: string; username: string; first_name: string; last_name: string}>
    countries: Array<{id: string; name: string}>
    roles: string[]
    rules: ISigningRuleRow[]
    capacities: Partial<Record<SigningAction, ISigningRuleCapacity>>
    /** The event's configuration version, as the capacity route answers it. */
    configVersion: number
    /** Who saved the rules last. */
    editor: string
    issuers: IStaffIssuer[]
    certificates: IStaffCertificate[]
    checks: ISigningChecksRow
    crls: IStaffCrl[]
    requests: ISigningRequestListRow[]
    /** Tenant translation overrides (English), as stored in the tenant's settings. */
    overrides: Record<string, string>
    /** The event's time zone, which the event info route answers. */
    timeZone: string
    /** The signers' titles by user id, which the event info route answers. */
    titles: Record<string, string>
}

const DAY = 24 * 60 * 60 * 1000
const CHANGED_AT = "2028-05-02T09:30:00Z"
const REQUESTED_AT = "2028-05-08T11:02:00Z"
const EDITOR = "editor-user-id"
const FINGERPRINT = "f7080f24aa55bb66cc77dd88ee99ff00112233445566778899aabbccddeefe78"
const PEM = "-----BEGIN CERTIFICATE-----\nc3ludGhldGlj\n-----END CERTIFICATE-----\n"
export const EXPORT_DOCUMENT_ID = storyId(9, 1)
/** The short-lived link the requests export returns with its document. */
export const EXPORT_URL = "https://s3.admin-story.invalid/exports/signing-requests.csv"

const rule = (
    editor: string,
    action: SigningAction,
    signatures: number | null,
    expires_minutes: number | null = 60,
    requester_signing = RequesterSigning.Allowed
): ISigningRuleRow => ({
    action,
    requirement: signatures === null ? SigningRequirement.NotRequired : SigningRequirement.Required,
    signatures: signatures ?? 1,
    requester_signing,
    expires_minutes,
    revision: 3,
    updated_by: EDITOR,
    updated_by_name: editor,
    updated_at: CHANGED_AT,
})

type RuleArgs = Parameters<typeof rule> extends [string, ...infer Rest] ? Rest : never
type CapacityArgs = Parameters<typeof capacity> extends [number, ...infer Rest] ? Rest : never

const capacity = (
    configVersion: number,
    roles: string[],
    posts: Array<{id: string; name: string}>,
    signers: number[],
    waiting = 0
): ISigningRuleCapacity => ({
    max: Math.max(0, ...signers),
    posts: posts.map(({id, name}, index) => ({election_id: id, name, count: signers[index]})),
    // The server's short Posts are of the saved number; the drawer counts the draft's itself.
    posts_short: [],
    posts_short_without_requester: [],
    roles: roles.map(roleOf),
    waiting,
    config_version: configVersion,
})

/** A role (Keycloak group) by name: its id is what a rule save sends. */
export const roleId = (name: string) => `group-${name.toLowerCase().replace(/\W+/g, "-")}`
const roleOf = (name: string) => ({id: roleId(name), name, path: `/${name}`})

const issuer = (
    index: number,
    name: string,
    issuedBy: string,
    not_after = "2040-06-30T00:00:00Z"
): IStaffIssuer => ({
    id: storyId(5, index),
    common_name: name,
    subject: `CN=${name},O=Example PKI`,
    issuer: `CN=${issuedBy},O=Example PKI`,
    issuer_common_name: issuedBy,
    not_after,
    fingerprint_sha256: FINGERPRINT.slice(index * 2) + FINGERPRINT.slice(0, index * 2),
})

const certificate = (
    index: number,
    username: string,
    holder: string,
    electionId: string | null,
    overrides: Partial<IStaffCertificate> = {}
): IStaffCertificate => ({
    id: storyId(6, index),
    user_id: storyId(8, index),
    username,
    user_display_name: holder.toLowerCase().replace(/\b\w/g, (letter) => letter.toUpperCase()),
    election_id: electionId,
    fingerprint_sha256: FINGERPRINT.slice(index * 4) + FINGERPRINT.slice(0, index * 4),
    spki_sha256: FINGERPRINT,
    holder_sha256: FINGERPRINT,
    serial: String(index),
    subject: `CN=${holder},O=Example`,
    issuer: "CN=Example Individual CA,O=Example PKI",
    not_before: "2028-01-01T00:00:00Z",
    not_after: "2030-01-11T00:00:00Z",
    status: StaffCertificateStatus.Active,
    registration: StaffCertificateRegistration.FirstUse,
    registered_by: null,
    registered_at: "2028-04-08T10:00:00Z",
    revoked_by: null,
    revoked_at: null,
    revoke_reason: null,
    ...overrides,
})

/** The Keycloak id of a person of the requests, who are named by username. */
export const userIdOf = (username: string) => `user-${username}`

const request = (
    index: number,
    action: SigningAction,
    status: SigningRequestStatus,
    scope: {election_id?: string; area_id?: string},
    required: number,
    signers: string[],
    overrides: Partial<ISigningRequestListRow> = {}
): ISigningRequestListRow => ({
    id: storyId(7, index),
    action,
    election_id: scope.election_id ?? null,
    area_id: scope.area_id ?? null,
    code: `7F3A-91C${index}`,
    required,
    status,
    cancel_reason: null,
    requested_by_username: signers[0] ?? "sbei-madrid-1",
    created_at: REQUESTED_AT,
    expires_at: null,
    requested_by_name: signers[0] ? `Name of ${signers[0]}` : "Name of sbei-madrid-1",
    approvals: signers.map((username, position) => ({
        id: storyId(4, index * 3 + position),
        username,
        display_name: `Name of ${username}`,
        signed_at: new Date(Date.parse(REQUESTED_AT) + (position + 1) * 60_000).toISOString(),
    })),
    ...overrides,
})

const OVERSEAS_POSTS = [
    {id: storyId(3, 1), name: "Madrid PE"},
    {id: storyId(3, 2), name: "Wellington PE"},
    {id: storyId(3, 3), name: "Dili PE"},
]
const [MADRID, WELLINGTON, DILI] = OVERSEAS_POSTS
const OVERSEAS_COUNTRIES = [
    {id: storyId(1, 1), name: "Spain"},
    {id: storyId(1, 2), name: "New Zealand"},
]

/** An overseas voting event: three Posts, most actions signed by the Posts' SBEIs. */
export const overseas = (): ISigningOrganization => {
    const configVersion = 17
    const editor = "Paolo Mendoza"
    const r = (...args: RuleArgs) => rule(editor, ...args)
    const cap = (...args: CapacityArgs) => capacity(configVersion, ...args)
    const sbei = (signers: number[], waiting = 0) => cap(["SBEI"], OVERSEAS_POSTS, signers, waiting)
    return {
        posts: OVERSEAS_POSTS,
        people: [
            {id: storyId(8, 7), username: "sbei-madrid-3", first_name: "Ana", last_name: "Reyes"},
            {
                id: storyId(8, 8),
                username: "trustee-madrid-2",
                first_name: "Jose",
                last_name: "Dela Cruz",
            },
        ],
        countries: OVERSEAS_COUNTRIES,
        roles: ["SBEI", "OFOV", "Configuration Manager", "Security Officer", "Auditor", "Trustee"],
        rules: [
            r(SigningAction.InitializeVoting, 2, 30),
            r(SigningAction.OpenVoting, 2, 30),
            r(SigningAction.CloseVoting, 2, 30),
            r(SigningAction.GenerateElectionReturns, 3, 60),
            r(SigningAction.GenerateReports, 2, 60),
            r(SigningAction.TransmitResults, 2, 60, RequesterSigning.NotAllowed),
            r(SigningAction.ApproveVoter, 1, null),
            r(SigningAction.ApproveConfiguration, 2, 1440),
            r(SigningAction.ConfirmKeyShare, 1),
            r(SigningAction.ContributeKeyShare, 1),
        ],
        capacities: {
            [SigningAction.InitializeVoting]: sbei([3, 2, 2]),
            [SigningAction.OpenVoting]: sbei([3, 2, 2]),
            [SigningAction.CloseVoting]: sbei([3, 2, 2], 1),
            [SigningAction.GenerateElectionReturns]: sbei([3, 3, 3], 1),
            [SigningAction.GenerateReports]: sbei([3, 2, 2]),
            [SigningAction.TransmitResults]: sbei([3, 2, 2], 2),
            [SigningAction.ApproveVoter]: cap(["OFOV", "SBEI"], OVERSEAS_POSTS, [4, 4, 4]),
            [SigningAction.ApproveConfiguration]: cap(
                ["Configuration Manager", "Security Officer"],
                [],
                [2]
            ),
            [SigningAction.ConfirmKeyShare]: cap(["Trustee"], [], [3]),
            [SigningAction.ContributeKeyShare]: cap(["Trustee"], [], [3]),
        },
        configVersion,
        editor,
        issuers: [
            issuer(1, "Example Root CA", "Example Root CA"),
            issuer(2, "Example Individual CA", "Example Root CA", "2033-06-30T00:00:00Z"),
        ],
        certificates: [
            certificate(1, "sbei-madrid-1", "MARIA L. SANTOS", MADRID.id),
            certificate(2, "sbei-madrid-2", "JOSE R. DELA CRUZ", MADRID.id),
            certificate(3, "ofov.aquino", "LIZA M. AQUINO", null, {
                registration: StaffCertificateRegistration.SecurityOfficer,
                registered_by: "security-officer-id",
                registered_by_name: "Grace Villanueva",
            }),
            certificate(4, "sbei-wellington-2", "TERESA M. LIM", WELLINGTON.id, {
                status: StaffCertificateStatus.Revoked,
                revoked_at: "2028-05-02T08:00:00Z",
                revoked_by: "security-officer-id",
                revoke_reason: "Lost token",
            }),
            // Relative to the day the story runs, so that it always expires soon.
            certificate(5, "sbei-wellington-1", "PAULO S. GARCIA", WELLINGTON.id, {
                not_after: new Date(Date.now() + 12 * DAY).toISOString(),
            }),
        ],
        checks: {
            revocation_check: RevocationCheck.Check,
            crl_unavailable: CrlUnavailablePolicy.Refuse,
            registration: CertificateRegistration.OnFirstUse,
            post_binding: CertificatePostBinding.OnePost,
            revision: 5,
            updated_at: CHANGED_AT,
        },
        crls: [
            {
                id: storyId(2, 1),
                issuer_fingerprint: FINGERPRINT,
                url: "http://crl.example.invalid/individual.crl",
                fetched_at: "2028-05-08T10:00:00Z",
                status: CrlStatus.Ok,
            },
        ],
        requests: [
            request(
                1,
                SigningAction.CloseVoting,
                SigningRequestStatus.Waiting,
                {
                    election_id: WELLINGTON.id,
                },
                2,
                ["sbei-wellington-1"],
                {expires_at: "2028-05-08T11:32:00Z"}
            ),
            request(
                2,
                SigningAction.GenerateElectionReturns,
                SigningRequestStatus.Waiting,
                {election_id: MADRID.id, area_id: OVERSEAS_COUNTRIES[0].id},
                3,
                [],
                {requested_by_username: "sbei-madrid-1", expires_at: "2028-05-08T12:08:00Z"}
            ),
            request(
                3,
                SigningAction.CloseVoting,
                SigningRequestStatus.Executed,
                {
                    election_id: MADRID.id,
                },
                2,
                ["sbei-madrid-1", "sbei-madrid-2"]
            ),
            request(
                4,
                SigningAction.TransmitResults,
                SigningRequestStatus.Expired,
                {
                    election_id: DILI.id,
                    area_id: OVERSEAS_COUNTRIES[1].id,
                },
                2,
                ["sbei-dili-1"]
            ),
            request(
                5,
                SigningAction.GenerateElectionReturns,
                SigningRequestStatus.Cancelled,
                {election_id: MADRID.id, area_id: OVERSEAS_COUNTRIES[0].id},
                3,
                ["sbei-madrid-3"],
                {cancel_reason: CancelReason.PayloadChanged}
            ),
            request(6, SigningAction.ApproveConfiguration, SigningRequestStatus.Executed, {}, 2, [
                "cm.mendoza",
                "so.villanueva",
            ]),
            // Past its time, before the expiry job marked it.
            request(
                8,
                SigningAction.OpenVoting,
                SigningRequestStatus.Waiting,
                {election_id: DILI.id},
                2,
                [],
                {requested_by_username: "sbei-dili-1", expires_at: "2026-01-01T00:00:00Z"}
            ),
        ],
        overrides: {},
        timeZone: "Asia/Manila",
        // The certificates' holders: a title, else the group that signs.
        titles: {
            [storyId(8, 1)]: "Chairperson",
            [storyId(8, 2)]: "SBEI",
            [storyId(8, 3)]: "OFOV",
        },
    }
}

const FACULTIES = [
    {id: storyId(3, 4), name: "Faculty of Engineering"},
    {id: storyId(3, 5), name: "Faculty of Law"},
    {id: storyId(3, 6), name: "Faculty of Medicine"},
]

/**
 * A student council: its Posts are faculties, it renames two actions and every
 * "Post" through tenant translation overrides, and most of its rules are off.
 */
export const studentCouncil = (): ISigningOrganization => {
    const configVersion = 3
    const editor = "Elections Office"
    const r = (...args: RuleArgs) => rule(editor, ...args)
    const cap = (...args: CapacityArgs) => capacity(configVersion, ...args)
    const officer = (signers: number[], waiting = 0) =>
        cap(["Returning Officer"], FACULTIES, signers, waiting)
    return {
        posts: FACULTIES,
        people: [
            {
                id: storyId(8, 9),
                username: "returning-officer-med",
                first_name: "Sam",
                last_name: "Okafor",
            },
        ],
        countries: [],
        roles: ["Returning Officer", "Electoral Commission", "Elections Office"],
        rules: [
            r(SigningAction.OpenVoting, 2, 30),
            r(SigningAction.CloseVoting, 2, 30),
            r(SigningAction.GenerateElectionReturns, 2, 120),
            r(SigningAction.ApproveVoter, 1, null),
            r(SigningAction.ApproveConfiguration, 1, 1440),
        ],
        capacities: {
            [SigningAction.OpenVoting]: officer([2, 2, 2]),
            [SigningAction.CloseVoting]: officer([2, 2, 2], 1),
            [SigningAction.GenerateElectionReturns]: officer([2, 2, 1]),
            [SigningAction.ApproveVoter]: cap(["Electoral Commission"], FACULTIES, [1, 1, 1]),
            [SigningAction.ApproveConfiguration]: cap(["Electoral Commission"], [], [3]),
        },
        configVersion,
        editor,
        issuers: [issuer(3, "Riverside University CA", "Riverside University CA")],
        certificates: [
            certificate(6, "returning-officer-law", "ALEX RIVERA", FACULTIES[1].id, {
                issuer: "CN=Riverside University CA",
            }),
        ],
        checks: {
            revocation_check: RevocationCheck.DontCheck,
            crl_unavailable: CrlUnavailablePolicy.AcceptUnchecked,
            registration: CertificateRegistration.SecurityOfficerOnly,
            post_binding: CertificatePostBinding.AnyPost,
            revision: 1,
            updated_at: CHANGED_AT,
        },
        crls: [],
        requests: [
            request(
                7,
                SigningAction.CloseVoting,
                SigningRequestStatus.Waiting,
                {
                    election_id: FACULTIES[0].id,
                },
                2,
                ["returning-officer-eng"]
            ),
        ],
        overrides: {
            // One term renames every Post of the signing settings.
            "adminPortal:signing.terms.post": "Faculty",
            "adminPortal:signing.terms.posts": "Faculties",
            "adminPortal:signing.actions.generate-election-returns.label":
                "Certify faculty results",
            "adminPortal:signing.actions.generate-election-returns.short": "Faculty results",
            "adminPortal:signing.actions.approve-voter.label": "Approve a student manually",
            "adminPortal:signing.actions.approve-voter.appliesTo": "The student's faculty",
            "adminPortal:signing.validation.shortPosts_one":
                "{{posts}} can only give {{n}} of the {{required}} signatures.",
        },
        timeZone: "Europe/Madrid",
        titles: {[storyId(8, 6)]: "Returning Officer"},
    }
}

/** A tenant override of the student council, which its stories take their expectations from. */
export const councilText = (key: string) => {
    const text = studentCouncil().overrides[`adminPortal:${key}`]
    if (text === undefined) throw new Error(`The student council doesn't override ${key}`)
    return text
}

export enum Organization {
    Overseas = "overseas",
    StudentCouncil = "student-council",
}

export const organizationOf = (organization: Organization) =>
    organization === Organization.StudentCouncil ? studentCouncil() : overseas()

/** Applies the organization's tenant overrides; the returned cleanup removes them. */
export function applyOverrides(organization: ISigningOrganization) {
    if (!Object.keys(organization.overrides).length) return undefined
    applyTenantTranslationOverrides({i18n: {en: organization.overrides}})
    return () => applyTenantTranslationOverrides(undefined)
}

const failed = () => ({errors: [new GraphQLError("Synthetic signing service failure")]})

/** A refusal as Hasura forwards a signing route's typed error (see lib/signing/api.ts). */
export const refusal = (extensions: Record<string, unknown>) => ({
    errors: [new GraphQLError("Synthetic refusal", {extensions})],
})

export interface ISigningHandlerOptions {
    /** Every write fails without a typed code. */
    failWrites?: boolean
    /** Writes, by operation, refused with these `extensions` (e.g. `{code: "forbidden"}`). */
    refuse?: Partial<Record<string, Record<string, unknown>>>
    /** The Posts a rule save reports as short of signers. */
    shortPosts?: Array<{election_id: string; name: string; count: number}>
}

/** The signing reads of the organization and its writes, which succeed unless told otherwise. */
export function signingHandlers(
    organization: ISigningOrganization,
    {failWrites = false, refuse = {}, shortPosts = []}: ISigningHandlerOptions = {}
): Record<string, Handler> {
    const write =
        (operation: string, field: string, value: (operation: Operation) => unknown) =>
        (current: Operation) => {
            if (failWrites) return failed()
            const extensions = refuse[operation]
            if (extensions) return refusal(extensions)
            return {data: {[field]: value(current)}}
        }
    return {
        GetSigningRules: () => ({data: {sequent_backend_signing_rule: organization.rules}}),
        GetSigningRuleCapacities: () => ({
            data: Object.fromEntries(
                Object.values(SigningAction).map((action) => [
                    capacityAlias(action),
                    capacityOf(organization, action),
                ])
            ),
        }),
        GetSigningCertificates: () => ({
            data: {
                checks: [organization.checks],
                issuers: organization.issuers,
                certificates: organization.certificates,
                crls: organization.crls,
            },
        }),
        GetSigningRequests: () => ({
            data: {sequent_backend_signing_request: organization.requests},
        }),
        SigningEventInfo: () => ({
            data: {
                signingEventInfo: {time_zone: organization.timeZone, titles: organization.titles},
            },
        }),
        // As Hasura answers a signer: the waiting requests of the action of the role sent.
        GetWaitingSigningRequests: (operation) => {
            const role = operation.getContext().headers?.["x-hasura-role"]
            return {
                data: {
                    sequent_backend_signing_request: organization.requests
                        .filter(
                            (request) =>
                                request.status === SigningRequestStatus.Waiting &&
                                SIGNING_ACTIONS[request.action].signPermission === role
                        )
                        .map(({approvals, ...request}) => ({
                            ...request,
                            approvals: approvals.map(({id, username, signed_at}) => ({
                                id,
                                user_id: userIdOf(username),
                                signed_at,
                            })),
                        })),
                },
            }
        },
        SigningPutRule: write("SigningPutRule", "signingPutRule", ({variables}) => ({
            revision: Number(variables.expected_revision) + 1,
            cancelled: [],
            rule: {...variables, revision: Number(variables.expected_revision) + 1},
            short_posts: shortPosts,
            warnings: shortPosts.length ? ["short-posts"] : [],
        })),
        SigningImportIssuers: write("SigningImportIssuers", "signingImportIssuers", () => ({
            imported: 1,
            skipped: 0,
            errors: [],
        })),
        SigningDeleteIssuer: write("SigningDeleteIssuer", "signingDeleteIssuer", ({variables}) => ({
            issuer_id: variables.issuer_id,
        })),
        SigningPutChecks: write("SigningPutChecks", "signingPutChecks", () => ({
            revision: organization.checks.revision + 1,
        })),
        SigningRegisterCertificate: write(
            "SigningRegisterCertificate",
            "signingRegisterCertificate",
            () => ({certificate_id: storyId(6, 9)})
        ),
        SigningRevokeCertificate: write(
            "SigningRevokeCertificate",
            "signingRevokeCertificate",
            ({variables}) => ({certificate_id: variables.certificate_id})
        ),
        SigningExportRequests: write("SigningExportRequests", "signingExportRequests", () => ({
            document_id: EXPORT_DOCUMENT_ID,
            sha256: "0".repeat(64),
            rows: organization.requests.length,
            url: EXPORT_URL,
        })),
        ...documentHandlers({[EXPORT_DOCUMENT_ID]: {name: "signing-requests.csv"}}),
    }
}

/** An action's capacity as the route answers it: an action nobody can sign has an empty one. */
export const capacityOf = (organization: ISigningOrganization, action: SigningAction) =>
    organization.capacities[action] ?? capacity(organization.configVersion, [], [], [])

/** The event's Posts, countries, people and roles, as react-admin reads them. */
export function signingRecords(organization: ISigningOrganization) {
    const data = recordsOrPending({
        sequent_backend_election: organization.posts.map(({id, name}) => ({
            id,
            tenant_id: TENANT_ID,
            election_event_id: EVENT_ID,
            name,
            presentation: electionPresentation(name, name),
        })),
        sequent_backend_area: organization.countries.map(({id, name}) => ({
            id,
            tenant_id: TENANT_ID,
            election_event_id: EVENT_ID,
            name,
        })),
        user: organization.people.map((person) => ({...person})),
        role: organization.roles.map((name) => ({id: roleId(name), name})),
    })
    return {...data, timeZone: organization.timeZone}
}

/** A signing widget api whose requests never load: the stories that don't open a panel. */
export const idleSigningApi = (): ISigningApi =>
    new Proxy({} as ISigningApi, {get: () => () => pending()})

/**
 * The signed-in role, the event record (locked down or not) and the signing
 * widget's provider, as the portal mounts it around every screen.
 */
export function SignaturesStory({
    boundary,
    data,
    role,
    lockedDown = false,
    signingApi,
    children,
}: PropsWithChildren<{
    boundary: ReturnType<typeof graphqlBoundary>
    data: ReturnType<typeof signingRecords>
    role: SigningRole
    lockedDown?: boolean
    signingApi?: ISigningApi
}>) {
    const [api] = useState(() => signingApi ?? idleSigningApi())
    const record = eventRecord(undefined, {
        presentation: {
            ...eventPresentation,
            timezones: {
                configured: [data.timeZone],
                primary: data.timeZone,
                logs: ELogTimeZonePolicy.PRIMARY,
            },
            locked_down: lockedDown
                ? EElectionEventLockedDown.LOCKED_DOWN
                : EElectionEventLockedDown.NOT_LOCKED_DOWN,
        },
    })
    return (
        <AdminStoryProvider
            boundary={boundary}
            dataProvider={data.provider}
            roles={SIGNING_ROLES[role]}
        >
            <SigningProvider api={api}>
                <EventTimeZoneProvider event={record}>
                    <RecordContextProvider value={record}>{children}</RecordContextProvider>
                </EventTimeZoneProvider>
            </SigningProvider>
        </AdminStoryProvider>
    )
}
