// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {gql, type ApolloClient} from "@apollo/client"
import {localInitializeRegistration} from "./localInitializeAuthority"
import {isId, isReviewedDigest, parsePage, type IReturnPage} from "./operationalGoldReturn"
import type {ApprovalPhase, IApprovalScope} from "./operationalApprovalApi"

const STATUS = gql`
    query LocalInitializeSetupStatus($request: uuid!, $digest: String!) {
        operationalInitializeSelfSetupStatus(request_id: $request, payload_sha256: $digest) {
            scope {
                tenant
                event
                request
                official
            }
            device_profile
            phase
            attempt
            delivery_revision
        }
    }
`
const START = gql`
    mutation LocalInitializeSetupStart(
        $request: uuid!
        $digest: String!
        $page: OperationalInitializeReturnPageInput!
        $attempt: uuid
    ) {
        operationalInitializeSelfSetupStart(
            request_id: $request
            payload_sha256: $digest
            page: $page
            attempt: $attempt
        ) {
            attempt
            redirect
            page {
                path
                panel
                election_id
            }
        }
    }
`
const REGISTER = gql`
    mutation LocalInitializeSetupRegister(
        $request: uuid!
        $digest: String!
        $attempt: uuid!
        $revision: Int!
    ) {
        operationalInitializeSelfSetupRegister(
            request_id: $request
            payload_sha256: $digest
            attempt: $attempt
            delivery_revision: $revision
        ) {
            scope {
                tenant
                event
                request
                official
            }
            device_profile
            phase
            attempt
            delivery_revision
        }
    }
`

const phases = [
    "setup-available",
    "waiting-for-setup-gold",
    "setup-confirmation-required",
    "setup-retry-available",
    "setup-pending",
    "setup-recovery-required",
    "certificate-on-file",
    "unavailable",
] as const
export type LocalSetupPhase = (typeof phases)[number]
export interface LocalSetupStatus {
    phase: LocalSetupPhase
    attempt?: string
    revision?: number
}

const isRevision = (value: unknown): value is number =>
    typeof value === "number" && Number.isInteger(value) && value >= 0 && value < 2147483647
const record = (value: unknown): Record<string, unknown> =>
    value && typeof value === "object" && !Array.isArray(value)
        ? (value as Record<string, unknown>)
        : {}

export function localSetupStatus(raw: unknown, expected: IApprovalScope): LocalSetupStatus {
    const result = record(raw)
    const actual = record(result.scope)
    const phase = phases.find((phase) => phase === result.phase)
    if (
        !phase ||
        result.device_profile !== "softhsm-test" ||
        !(["tenant", "event", "request", "official"] as const).every(
            (key) => isId(actual[key]) && actual[key] === expected[key]
        )
    )
        throw new Error("Setup status unavailable")
    const revision = result.delivery_revision
    if (phase === "setup-confirmation-required" || phase === "setup-retry-available") {
        if (!isId(result.attempt) || !isRevision(revision))
            throw new Error("Original registration unavailable")
        return {phase, attempt: result.attempt, revision}
    }
    if (revision != null) throw new Error("Invalid setup status")
    if (phase === "waiting-for-setup-gold") {
        if (!isId(result.attempt)) throw new Error("Original setup attempt unavailable")
        return {phase, attempt: result.attempt}
    }
    if (result.attempt != null) throw new Error("Invalid setup status")
    return {phase}
}

export function localSetupControl(
    phase: string,
    reviewed: boolean,
    attempt?: string
): "start-setup" | "register-key" | "approve" | null {
    if (!reviewed) return null
    if (phase === "setup-available" || (phase === "waiting-for-setup-gold" && isId(attempt)))
        return "start-setup"
    if (
        (phase === "setup-confirmation-required" || phase === "setup-retry-available") &&
        isId(attempt)
    )
        return "register-key"
    return phase === "challenge-available" ? "approve" : null
}

export function localDialogPhase(approval: ApprovalPhase, setup: LocalSetupPhase) {
    if (approval !== "unavailable" && approval !== "challenge-available") return approval
    return setup === "certificate-on-file" && approval === "challenge-available" ? approval : setup
}

export function createLocalInitializeSetupApi(
    client: Pick<ApolloClient<unknown>, "query" | "mutate">,
    origin: string | undefined,
    registration: unknown
) {
    const pin = localInitializeRegistration(origin, registration)
    if (!pin?.setupClient) return
    const context = {
        uri: pin.graphql,
        credentials: "omit",
        queryDeduplication: false,
        headers: {"x-hasura-role": "sign-initialize-voting"},
        fetchOptions: {credentials: "omit", redirect: "error", referrerPolicy: "no-referrer"},
    }
    const review = (scope: IApprovalScope, digest: string) => {
        if (
            scope.tenant !== pin.tenant ||
            !Object.values(scope).every(isId) ||
            !isReviewedDigest(digest)
        )
            throw new Error("Invalid setup request")
        return {request: scope.request, digest}
    }
    return {
        status: async (scope: IApprovalScope, digest: string) => {
            const {data, errors} = await client.query({
                query: STATUS,
                context,
                fetchPolicy: "no-cache",
                variables: review(scope, digest),
            })
            if (errors?.length) throw new Error("Setup status unavailable")
            return localSetupStatus(data?.operationalInitializeSelfSetupStatus, scope)
        },
        start: async (
            scope: IApprovalScope,
            digest: string,
            page: IReturnPage,
            attempt?: string
        ) => {
            if (!parsePage(page) || (attempt !== undefined && !isId(attempt)))
                throw new Error("Invalid setup start")
            const {data, errors} = await client.mutate({
                mutation: START,
                context,
                fetchPolicy: "no-cache",
                variables: {...review(scope, digest), page, attempt},
            })
            const result = record(data?.operationalInitializeSelfSetupStart)
            // Apollo may append __typename. Project only the closed navigation fields.
            const p = record(result.page)
            const originalPage = parsePage({
                path: p.path,
                panel: p.panel,
                election_id: p.election_id,
            })
            if (
                errors?.length ||
                !isId(result.attempt) ||
                (attempt && attempt !== result.attempt) ||
                typeof result.redirect !== "string" ||
                !originalPage
            )
                throw new Error("Setup start unconfirmed")
            return {attempt: result.attempt, redirect: result.redirect, page: originalPage}
        },
        register: async (
            scope: IApprovalScope,
            digest: string,
            attempt: string,
            revision: number
        ) => {
            if (!isId(attempt) || !isRevision(revision))
                throw new Error("Invalid setup registration")
            const {data, errors} = await client.mutate({
                mutation: REGISTER,
                context,
                fetchPolicy: "no-cache",
                variables: {...review(scope, digest), attempt, revision},
            })
            if (errors?.length) throw new Error("Registration delivery unconfirmed")
            return localSetupStatus(data?.operationalInitializeSelfSetupRegister, scope)
        },
    }
}
