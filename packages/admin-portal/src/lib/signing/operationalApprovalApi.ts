// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {gql, type ApolloClient} from "@apollo/client"
import {parseActionResponseBody} from "@/services/graphqlActionError"
import {type IReturnPage, isId} from "./operationalGoldReturn"

export interface IApprovalScope {
    tenant: string
    event: string
    request: string
    official: string
}

const STATUS = gql`
    query OperationalInitializeStatus($request: uuid!) {
        operationalInitializeStatus(request_id: $request) {
            scope {
                tenant
                event
                request
                official
            }
            phase
            device_profile
            cancellable_attempt
        }
    }
`
const SETUP_STATUS = gql`
    query OperationalInitializeSetupStatus($request: uuid!) {
        operationalInitializeSetupStatus(request_id: $request) {
            scope {
                tenant
                event
                request
                official
            }
            phase
        }
    }
`
const SETUP = gql`
    mutation OperationalInitializeSetup($request: uuid!) {
        operationalInitializeSetup(request_id: $request) {
            scope {
                tenant
                event
                request
                official
            }
            phase
        }
    }
`
const START = gql`
    mutation OperationalInitializeStart(
        $request: uuid!
        $page: OperationalInitializeReturnPageInput!
        $digest: String!
    ) {
        operationalInitializeStart(request_id: $request, page: $page, payload_sha256: $digest) {
            attempt
            redirect
        }
    }
`
const CANCEL = gql`
    mutation OperationalInitializeCancel($request: uuid!, $attempt: uuid!) {
        operationalInitializeCancel(request_id: $request, attempt: $attempt) {
            phase
        }
    }
`
const RECONCILE = gql`
    mutation OperationalInitializeReconcile($request: uuid!) {
        operationalInitializeReconcile(request_id: $request) {
            phase
        }
    }
`

const context = {headers: {"x-hasura-role": "sign-initialize-voting"}}
const record = (value: unknown): Record<string, unknown> =>
    value && typeof value === "object" && !Array.isArray(value)
        ? (value as Record<string, unknown>)
        : {}

export type ApprovalPhase =
    | "challenge-available"
    | "waiting-for-identity"
    | "pending"
    | "approval-recorded"
    | "reconcile"
    | "unavailable"
type SetupPhase =
    | "setup-available"
    | "setup-pending"
    | "setup-retry-available"
    | "setup-recovery-required"
    | "certificate-on-file"
    | "unavailable"

const scoped = (raw: unknown, expected: IApprovalScope) => {
    const result = record(raw)
    const scope = record(result.scope)
    if (
        !(["tenant", "event", "request", "official"] as const).every(
            (key) => isId(scope[key]) && scope[key] === expected[key]
        )
    )
        throw new Error("Invalid operational signing scope")
    return result
}

export const parseApprovalStatus = (
    raw: unknown,
    expected: IApprovalScope
): {
    phase: ApprovalPhase
    softwareDemo: boolean
    cancellableAttempt?: string
} => {
    const result = scoped(raw, expected)
    const phase = result.phase
    if (
        phase !== "challenge-available" &&
        phase !== "waiting-for-identity" &&
        phase !== "pending" &&
        phase !== "approval-recorded" &&
        phase !== "reconcile" &&
        phase !== "unavailable"
    )
        throw new Error("Invalid operational signing state")
    const profile = result.device_profile
    if (profile !== "yubihsm" && profile !== "softhsm-test")
        throw new Error("Invalid device profile")
    const attempt = result.cancellable_attempt
    if (attempt != null && (phase !== "waiting-for-identity" || !isId(attempt))) {
        throw new Error("Invalid unused signing challenge")
    }
    return {
        phase,
        softwareDemo: profile === "softhsm-test",
        cancellableAttempt: isId(attempt) ? attempt : undefined,
    }
}

export const parseSetupStatus = (raw: unknown, expected: IApprovalScope): SetupPhase => {
    const {phase} = scoped(raw, expected)
    if (
        phase !== "setup-available" &&
        phase !== "setup-pending" &&
        phase !== "setup-retry-available" &&
        phase !== "setup-recovery-required" &&
        phase !== "certificate-on-file" &&
        phase !== "unavailable"
    )
        throw new Error("Invalid setup state")
    return phase
}

export function trustedSigningRedirect(value: string, keycloakBase: string, tenant: string) {
    if (!isId(tenant) || value.length > 2048) throw new Error("Invalid verification redirect")
    const base = new URL(keycloakBase)
    const target = new URL(value)
    const path =
        base.pathname.replace(/\/$/, "") + `/realms/tenant-${tenant}/protocol/openid-connect/auth`
    const client = target.searchParams.get("client_id")
    const request = target.searchParams.get("request_uri")
    if (
        base.protocol !== "https:" ||
        base.username ||
        base.password ||
        base.search ||
        base.hash ||
        target.origin !== base.origin ||
        target.pathname !== path ||
        target.username ||
        target.password ||
        target.hash ||
        Array.from(target.searchParams).length !== 2 ||
        !client ||
        !/^[a-zA-Z0-9_.-]{1,150}$/.test(client) ||
        !request ||
        !/^urn:ietf:params:oauth:request_uri:[a-zA-Z0-9_.-]{1,256}$/.test(request)
    ) {
        throw new Error("Invalid verification redirect")
    }
    return target.href
}

export class SetupGoldRequired extends Error {
    constructor() {
        super("Fresh Gold authentication required")
        Object.setPrototypeOf(this, SetupGoldRequired.prototype)
    }
}

export function createOperationalApprovalApi(
    client: Pick<ApolloClient<unknown>, "query" | "mutate">
) {
    return {
        status: async (scope: IApprovalScope) => {
            const {data, errors} = await client.query({
                query: STATUS,
                variables: {request: scope.request},
                context,
                fetchPolicy: "no-cache",
            })
            if (errors?.length) throw new Error("Signing status unavailable")
            return parseApprovalStatus(data?.operationalInitializeStatus, scope)
        },
        setupStatus: async (scope: IApprovalScope) => {
            const {data, errors} = await client.query({
                query: SETUP_STATUS,
                variables: {request: scope.request},
                context,
                fetchPolicy: "no-cache",
            })
            if (errors?.length) throw new Error("Setup status unavailable")
            return parseSetupStatus(data?.operationalInitializeSetupStatus, scope)
        },
        setup: async (scope: IApprovalScope) => {
            const {data, errors} = await client.mutate({
                mutation: SETUP,
                variables: {request: scope.request},
                context,
                fetchPolicy: "no-cache",
                errorPolicy: "all",
            })
            if (
                errors?.some(
                    (error) =>
                        error.extensions?.gold_required === true ||
                        record(
                            record(
                                parseActionResponseBody(
                                    record(record(error.extensions?.internal).response).body as
                                        | string
                                        | undefined
                                )
                            ).extensions
                        ).gold_required === true
                )
            ) {
                throw new SetupGoldRequired()
            }
            if (errors?.length) throw new Error("Setup unavailable")
            return parseSetupStatus(data?.operationalInitializeSetup, scope)
        },
        start: async (scope: IApprovalScope, page: IReturnPage, digest: string) => {
            const {data, errors} = await client.mutate({
                mutation: START,
                variables: {request: scope.request, page, digest},
                context,
                fetchPolicy: "no-cache",
            })
            const started = record(data?.operationalInitializeStart)
            if (errors?.length || !isId(started.attempt) || typeof started.redirect !== "string") {
                throw new Error("Signing start unconfirmed")
            }
            return {attempt: started.attempt, redirect: started.redirect}
        },
        cancelUnused: async (scope: IApprovalScope, attempt: string) => {
            if (!isId(attempt)) throw new Error("Invalid unused signing challenge")
            const {data, errors} = await client.mutate({
                mutation: CANCEL,
                variables: {request: scope.request, attempt},
                context,
                fetchPolicy: "no-cache",
            })
            if (errors?.length || typeof data?.operationalInitializeCancel?.phase !== "string") {
                throw new Error("Challenge cancellation unconfirmed")
            }
        },
        reconcile: async (scope: IApprovalScope) => {
            const {errors} = await client.mutate({
                mutation: RECONCILE,
                variables: {request: scope.request},
                context,
                fetchPolicy: "no-cache",
            })
            if (errors?.length) throw new Error("Signing status unconfirmed")
        },
    }
}
