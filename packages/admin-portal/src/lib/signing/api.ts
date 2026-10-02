// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// What the signing widget asks of Harvest (design §5). Components take the
// `ISigningApi` interface, so stories and tests hand them a fake; only
// `createSigningApi` knows the transport: the Hasura actions that forward to
// the Harvest routes, like the portal's other Harvest calls.
//
// Contract of the Hasura actions (all synchronous, handler = the Harvest route;
// a `jsonb` output field carries the route's JSON as is):
//
//   query    signingGetRequest(request_id: uuid!): {panel: jsonb!}
//            POST /signing-requests/get. `panel` = ISigningPanelData: the
//            ISigningRequestPanel {request, rule, count, signers, document_url};
//            signers are {user_id, username, display_name, title, is_you,
//            status, signed_at, certificate_cn}
//            plus the optional election_name, area_name, document_name,
//            document_pages, details [{key, label?, value}] and time_zone
//            (the election event's IANA zone, e.g. "Asia/Manila").
//   mutation signingCheckCertificate(request_id: uuid!, chain_pem: [String!]!):
//            {checks: jsonb!, certificate, registration, revocation_status} (the widget reads checks)
//            POST …/check-certificate. `checks` = [{id, ok, detail}] (CertificateCheckId).
//            detail: trusted-issuer → the root's name; not-revoked → when the CRL
//            was fetched (RFC 3339); registered → the registration time, null on a
//            first use; registered-to-other → the other person's name.
//   mutation signingPdfPrepare(request_id: uuid!, chain_pem: [String!]!):
//            {revision: Int!, digest_b64: String!, signing_time: String!}
//   mutation signingApprove(request_id: uuid!, chain_pem: [String!]!, algorithm: String!,
//            payload_signature_b64: String!, document_signature_b64: String,
//            pdf_cms_b64: String, revision: Int): {status: String!, count: Int!, required: Int!}
//   mutation signingOpenFailure(request_id: uuid!, file_name: String!, reason: String!): {request_id: uuid!}
//            reason = CertificateOpenFailure (wrong-password | unreadable | no-key).
//   mutation signingHandover(request_id: uuid!): {request_id: uuid!}
//   mutation signingCancel(request_id: uuid!, reason: String): {request_id: uuid!}
//
// Each call sends an explicit `x-hasura-role` the action allows and the user
// holds (roles.ts), so staff without admin-user don't get the portal's
// read-only default.
//
// Errors: Harvest answers `{"message", "extensions": {"code", "check"?, "status"?}}`
// (the codes below, as harvest's signing routes answer them); Hasura
// forwards `extensions` into the GraphQL error. The widget reads
// `extensions.code` first and the HTTP status only when no code is given
// (kept in extensions.internal.response).
//   signing-refused  (422) a certificate check refused the approval; `check` = CertificateCheckId
//   already-signed   (422) this person, certificate, key or holder already signed it
//   request-closed   (409) the request is no longer waiting (`status` = its status) or expired
//   stale-revision   (409) the prepared PDF revision is stale: the only case prepared again
//   forbidden        (403) the permission is missing
//   not-found (404), invalid (400), conflict (409), locked-down (409): shown as a failure
// A 409 without a code counts as stale-revision.
//
// `details` rows label subject fields: `key` must be a key of the payload's
// subject and `value` its display form (strings as is, numbers as text,
// arrays joined with ", "); the dialog refuses any other row. The Post and
// country names are labels of the payload's election_id and area_id.

import {gql, type ApolloClient, type DocumentNode} from "@apollo/client"
import type {IGraphQLActionError} from "@sequentech/ui-core"
import {parseActionResponseBody} from "@/services/graphqlActionError"
import type {IPermissions} from "@/types/keycloak"
import {SigningOperation, signingOperationRole} from "./roles"
import {
    CertificateCheckId,
    type IApproveSigningRequestInput,
    type IApproveSigningRequestOutput,
    type ICancelSigningRequestInput,
    type ICheckCertificateInput,
    type ICheckCertificateOutput,
    type IOpenFailureInput,
    type IPdfPrepareInput,
    type IPdfPrepareOutput,
    type ISigningRequestPanel,
    type SigningAction,
} from "./types"

/** A labelled value of the request's details table (actions without a document). */
export interface ISigningDetail {
    /** Label key under `signing.details`, so an organization can rename it. */
    key: string
    /** The label when the key has no translation; the key itself when absent. */
    label?: string | null
    value: string
}

/**
 * `GET /signing-requests/<id>` as the panel reads it: the PR 1 shape plus the
 * display fields the drafts show. They are optional, so an older answer still renders.
 */
export interface ISigningPanelData extends ISigningRequestPanel {
    /** The Post's name (the request's election). */
    election_name?: string | null
    /** The country's name (the request's area). */
    area_name?: string | null
    /** The document being signed, for actions with one. */
    document_name?: string | null
    document_pages?: number | null
    /** What the dialog shows for actions without a document; the subject when absent. */
    details?: ISigningDetail[] | null
    /** The election event's time zone (IANA); times are shown in the browser's when absent. */
    time_zone?: string | null
}

export interface ISigningApi {
    getRequest(requestId: string): Promise<ISigningPanelData>
    /** A dry run of every server check for the chain; nothing is recorded. */
    checkCertificate(
        requestId: string,
        input: ICheckCertificateInput
    ): Promise<ICheckCertificateOutput>
    /** PDF mode: the server writes the next revision and answers its ByteRange digest. */
    pdfPrepare(requestId: string, input: IPdfPrepareInput): Promise<IPdfPrepareOutput>
    approve(
        requestId: string,
        input: IApproveSigningRequestInput
    ): Promise<IApproveSigningRequestOutput>
    /** Records that a file didn't open: its name and the reason, never the file or password. */
    reportOpenFailure(requestId: string, input: IOpenFailureInput): Promise<void>
    handover(requestId: string): Promise<void>
    cancel(requestId: string, input: ICancelSigningRequestInput): Promise<void>
    /** The document's bytes (EML mode signs them). */
    fetchDocument(url: string): Promise<Uint8Array<ArrayBuffer>>
}

export enum SigningApiErrorKind {
    /** A certificate check refused the approval; `check` names it. */
    Refused = "refused",
    /** The prepared PDF revision is stale; prepare again. */
    Stale = "stale",
    AlreadySigned = "already-signed",
    /** The request no longer waits for signatures. */
    Closed = "closed",
    Forbidden = "forbidden",
    Other = "other",
}

/** The `extensions.code` values of the signing routes. */
export enum SigningErrorCode {
    Refused = "signing-refused",
    StaleRevision = "stale-revision",
    AlreadySigned = "already-signed",
    RequestClosed = "request-closed",
    Forbidden = "forbidden",
}

const KIND_OF_CODE: Record<string, SigningApiErrorKind> = {
    [SigningErrorCode.Refused]: SigningApiErrorKind.Refused,
    [SigningErrorCode.StaleRevision]: SigningApiErrorKind.Stale,
    [SigningErrorCode.AlreadySigned]: SigningApiErrorKind.AlreadySigned,
    [SigningErrorCode.RequestClosed]: SigningApiErrorKind.Closed,
    [SigningErrorCode.Forbidden]: SigningApiErrorKind.Forbidden,
}

const KIND_OF_STATUS: Record<number, SigningApiErrorKind> = {
    422: SigningApiErrorKind.Refused,
    409: SigningApiErrorKind.Stale,
    403: SigningApiErrorKind.Forbidden,
    401: SigningApiErrorKind.Forbidden,
}

export class SigningApiError extends Error {
    readonly kind: SigningApiErrorKind
    readonly status: number | null
    readonly check: CertificateCheckId | null

    constructor(
        kind: SigningApiErrorKind,
        message: string,
        {status = null, check = null}: {status?: number | null; check?: CertificateCheckId | null}
    ) {
        super(message)
        // The portal compiles to ES5, where an Error subclass loses its prototype: restore it
        // so `instanceof SigningApiError` holds in the production build.
        Object.setPrototypeOf(this, SigningApiError.prototype)
        this.name = "SigningApiError"
        this.kind = kind
        this.status = status
        this.check = check
    }
}

const CHECK_IDS = new Set<string>(Object.values(CertificateCheckId))

const record = (value: unknown): Record<string, unknown> =>
    value && typeof value === "object" ? (value as Record<string, unknown>) : {}

const checkIdOf = (...candidates: unknown[]): CertificateCheckId | null =>
    (candidates.find((c) => typeof c === "string" && CHECK_IDS.has(c)) as
        | CertificateCheckId
        | undefined) ?? null

/**
 * Classifies a failed Hasura action by Harvest's `extensions.code`, promoted
 * by Hasura or left in the forwarded body, and by the HTTP status when no
 * code is given.
 */
export const toSigningApiError = (error: unknown): SigningApiError => {
    if (error instanceof SigningApiError) {
        return error
    }
    const actionError = error as IGraphQLActionError | undefined
    for (const graphQLError of actionError?.graphQLErrors ?? []) {
        const extensions = record(graphQLError.extensions)
        const response = graphQLError.extensions?.internal?.response
        const status = typeof response?.status === "number" ? response.status : null
        const rawBody = response?.body ?? undefined
        const body = record(parseActionResponseBody(rawBody))
        const bodyExtensions = record(body.extensions)
        const code = [extensions.code, bodyExtensions.code, body.code].find(
            (c): c is string => typeof c === "string" && c in KIND_OF_CODE
        )
        const kind = code
            ? KIND_OF_CODE[code]
            : status !== null
              ? KIND_OF_STATUS[status]
              : undefined
        if (kind) {
            const check =
                kind === SigningApiErrorKind.Refused
                    ? checkIdOf(
                          extensions.check,
                          bodyExtensions.check,
                          body.check,
                          body.id,
                          bodyExtensions.code,
                          body.code,
                          rawBody?.trim()
                      )
                    : null
            return new SigningApiError(kind, graphQLError.message ?? "signing request failed", {
                status,
                check,
            })
        }
    }
    const message = error instanceof Error ? error.message : String(error)
    return new SigningApiError(SigningApiErrorKind.Other, message, {})
}

const isString = (value: unknown): value is string => typeof value === "string"
const isStringOrNull = (value: unknown) => value === null || value === undefined || isString(value)

/**
 * Refuses an answer that isn't a panel, so a malformed response shows as a
 * load failure instead of breaking the page.
 */
export const validatePanel = (value: unknown): ISigningPanelData => {
    const panel = record(value)
    const request = record(panel.request)
    const rule = record(panel.rule)
    const valid =
        ["id", "tenant_id", "election_event_id", "action", "status", "code"].every((field) =>
            isString(request[field])
        ) &&
        isString(request.canonical_payload) &&
        isString(request.payload_sha256) &&
        typeof request.required === "number" &&
        isStringOrNull(request.expires_at) &&
        isStringOrNull(request.document_sha256) &&
        isString(rule.requester_signing) &&
        typeof panel.count === "number" &&
        Array.isArray(panel.signers) &&
        panel.signers.every((signer) => {
            const s = record(signer)
            return (
                isString(s.user_id) &&
                isString(s.username) &&
                isString(s.display_name) &&
                isStringOrNull(s.signed_at)
            )
        }) &&
        isStringOrNull(panel.document_url) &&
        (panel.details === undefined ||
            panel.details === null ||
            (Array.isArray(panel.details) &&
                panel.details.every((d) => isString(record(d).key) && isString(record(d).value))))
    if (!valid) {
        throw new SigningApiError(SigningApiErrorKind.Other, "the signing request is malformed", {})
    }
    return value as ISigningPanelData
}

export const SIGNING_REQUEST = gql`
    query SigningGetRequest($request_id: uuid!) {
        signingGetRequest(request_id: $request_id) {
            panel
        }
    }
`

export const SIGNING_CHECK_CERTIFICATE = gql`
    mutation SigningCheckCertificate($request_id: uuid!, $chain_pem: [String!]!) {
        signingCheckCertificate(request_id: $request_id, chain_pem: $chain_pem) {
            checks
        }
    }
`

export const SIGNING_PDF_PREPARE = gql`
    mutation SigningPdfPrepare($request_id: uuid!, $chain_pem: [String!]!) {
        signingPdfPrepare(request_id: $request_id, chain_pem: $chain_pem) {
            revision
            digest_b64
            signing_time
        }
    }
`

export const SIGNING_APPROVE = gql`
    mutation SigningApprove(
        $request_id: uuid!
        $chain_pem: [String!]!
        $algorithm: String!
        $payload_signature_b64: String!
        $document_signature_b64: String
        $pdf_cms_b64: String
        $revision: Int
    ) {
        signingApprove(
            request_id: $request_id
            chain_pem: $chain_pem
            algorithm: $algorithm
            payload_signature_b64: $payload_signature_b64
            document_signature_b64: $document_signature_b64
            pdf_cms_b64: $pdf_cms_b64
            revision: $revision
        ) {
            status
            count
            required
        }
    }
`

export const SIGNING_OPEN_FAILURE = gql`
    mutation SigningOpenFailure($request_id: uuid!, $file_name: String!, $reason: String!) {
        signingOpenFailure(request_id: $request_id, file_name: $file_name, reason: $reason) {
            request_id
        }
    }
`

export const SIGNING_HANDOVER = gql`
    mutation SigningHandover($request_id: uuid!) {
        signingHandover(request_id: $request_id) {
            request_id
        }
    }
`

export const SIGNING_CANCEL = gql`
    mutation SigningCancel($request_id: uuid!, $reason: String) {
        signingCancel(request_id: $request_id, reason: $reason) {
            request_id
        }
    }
`

type Client = Pick<ApolloClient<object>, "query" | "mutate">

export interface ISigningApiOptions {
    /**
     * Whether the signed-in user holds a permission. Each call then names a
     * Hasura role its action allows and the user holds (see `signingOperationRole`);
     * without it, the portal's default role goes.
     */
    holds?: (permission: IPermissions) => boolean
}

/** The production api: one Hasura action per Harvest route. */
export const createSigningApi = (
    client: Client,
    fetchImpl: typeof fetch = (...args) => fetch(...args),
    {holds}: ISigningApiOptions = {}
): ISigningApi => {
    // The action of each request loaded, so its calls send its sign permission.
    const actions = new Map<string, SigningAction>()
    const context = (operation: SigningOperation, requestId: string) => {
        const role = holds
            ? signingOperationRole(operation, actions.get(requestId) ?? null, holds)
            : null
        return role ? {headers: {"x-hasura-role": role}} : undefined
    }
    const mutate = async <T>(
        operation: SigningOperation,
        mutation: DocumentNode,
        field: string,
        variables: {request_id: string} & Record<string, unknown>
    ): Promise<T> => {
        try {
            const {data} = await client.mutate({
                mutation,
                variables,
                fetchPolicy: "no-cache",
                context: context(operation, variables.request_id),
            })
            return record(data)[field] as T
        } catch (error) {
            throw toSigningApiError(error)
        }
    }
    const sign = <T>(mutation: DocumentNode, field: string, variables: {request_id: string}) =>
        mutate<T>(SigningOperation.Sign, mutation, field, variables)
    return {
        getRequest: async (requestId) => {
            try {
                const {data} = await client.query({
                    query: SIGNING_REQUEST,
                    variables: {request_id: requestId},
                    fetchPolicy: "no-cache",
                    context: context(SigningOperation.GetRequest, requestId),
                })
                const panel = validatePanel(record(record(data).signingGetRequest).panel)
                actions.set(requestId, panel.request.action)
                return panel
            } catch (error) {
                throw toSigningApiError(error)
            }
        },
        checkCertificate: (requestId, input) =>
            sign<ICheckCertificateOutput>(SIGNING_CHECK_CERTIFICATE, "signingCheckCertificate", {
                request_id: requestId,
                ...input,
            }),
        pdfPrepare: (requestId, input) =>
            sign<IPdfPrepareOutput>(SIGNING_PDF_PREPARE, "signingPdfPrepare", {
                request_id: requestId,
                ...input,
            }),
        approve: (requestId, input) =>
            sign<IApproveSigningRequestOutput>(SIGNING_APPROVE, "signingApprove", {
                request_id: requestId,
                ...input,
            }),
        reportOpenFailure: async (requestId, input) => {
            await sign(SIGNING_OPEN_FAILURE, "signingOpenFailure", {
                request_id: requestId,
                ...input,
            })
        },
        handover: async (requestId) => {
            await sign(SIGNING_HANDOVER, "signingHandover", {request_id: requestId})
        },
        cancel: async (requestId, input) => {
            await mutate(SigningOperation.Cancel, SIGNING_CANCEL, "signingCancel", {
                request_id: requestId,
                reason: input.reason ?? null,
            })
        },
        fetchDocument: async (url) => {
            const response = await fetchImpl(url)
            if (!response.ok) {
                throw new SigningApiError(
                    SigningApiErrorKind.Other,
                    `document download failed: ${response.status}`,
                    {status: response.status}
                )
            }
            return new Uint8Array(await response.arrayBuffer())
        },
    }
}
