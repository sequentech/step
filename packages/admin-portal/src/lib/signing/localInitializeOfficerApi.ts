// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {gql, type ApolloClient} from "@apollo/client"
import {fromBase64, hex, sha256, toArrayBuffer, toBase64} from "./der"
import {localInitializeRegistration, matchLocalInitializeSettings} from "./localInitializeAuthority"
import {isId, isReviewedDigest} from "./operationalGoldReturn"

const NOMINEE_REVIEW = gql`
    query LocalNomineeReviewStatus($request_id: uuid!, $payload_sha256: String!) {
        operationalInitializeNomineeReviewStatus(request_id: $request_id, payload_sha256: $payload_sha256) {phase}
    }
`
const NOMINEE_RESUME = gql`
    mutation LocalNomineeResume($request_id: uuid!, $payload_sha256: String!) {
        operationalInitializeNomineeResume(request_id: $request_id, payload_sha256: $payload_sha256) {active}
    }
`
export type NomineeReviewPhase = "unavailable" | "gold-required" | "resume-ready" | "ready"
const PRESENCE = gql`
    mutation LocalNomineePresence($request_id: uuid!, $payload_sha256: String!) {
        operationalInitializeNomineePresence(request_id: $request_id, payload_sha256: $payload_sha256) {active}
    }
`
const PENDING = gql`
    query LocalOfficerPending($election_event_id: uuid!) {
        operationalInitializeOfficerPending(election_event_id: $election_event_id) {
            items {request_id payload_sha256 official_id election_id}
        }
    }
`
const REVIEW = gql`
    mutation LocalOfficerReview($request_id: uuid!, $payload_sha256: String!, $official_id: uuid!) {
        operationalInitializeOfficerReview(
            request_id: $request_id, payload_sha256: $payload_sha256, official_id: $official_id
        ) {request_id payload_sha256 official_id election_id enrollment csr_pem csr_sha256 spki_sha256}
    }
`
const ATTACH = gql`
    mutation LocalOfficerAttach(
        $request_id: uuid!, $payload_sha256: String!, $official_id: uuid!,
        $csr_sha256: String!, $leaf_sha256: String!, $leaf_der: String!
    ) {
        operationalInitializeOfficerAttach(
            request_id: $request_id, payload_sha256: $payload_sha256, official_id: $official_id,
            csr_sha256: $csr_sha256, leaf_sha256: $leaf_sha256, leaf_der: $leaf_der
        ) {enrollment certificate leaf_sha256 spki_sha256 attached_at}
    }
`

export interface LocalOfficerRef {
    request_id: string
    payload_sha256: string
    official_id: string
    election_id: string
}
interface ReviewedCsr extends LocalOfficerRef {
    enrollment: string
    csr_pem: string
    csr_sha256: string
    spki_sha256: string
}
interface PublicLeaf {leaf_der: string; leaf_sha256: string}
interface AttachmentReceipt {
    enrollment: string
    certificate: string
    leaf_sha256: string
    spki_sha256: string
    attached_at: string
}

const record = (raw: unknown): Record<string, unknown> =>
    raw && typeof raw === "object" && !Array.isArray(raw) ? raw as Record<string, unknown> : {}
const ref = (raw: unknown): LocalOfficerRef => {
    const {request_id, payload_sha256, official_id, election_id} = record(raw)
    if (!isId(request_id) || !isReviewedDigest(payload_sha256) || !isId(official_id) || !isId(election_id))
        throw new Error("Invalid Initialize request")
    return {request_id, payload_sha256, official_id, election_id}
}
const variables = ({request_id, payload_sha256, official_id}: LocalOfficerRef) =>
    ({request_id, payload_sha256, official_id})

export const readPublicLeaf = async (file: Pick<File, "size" | "arrayBuffer">): Promise<PublicLeaf> => {
    if (file.size < 1 || file.size > 46080) throw new Error("Select one public certificate")
    let der = new Uint8Array(await file.arrayBuffer())
    if (der[0] === 0x2d) {
        const pem = new TextDecoder("utf-8", {fatal: true, ignoreBOM: true}).decode(der)
        const match = /^-----BEGIN CERTIFICATE-----\r?\n([A-Za-z0-9+/=\r\n]+)\r?\n-----END CERTIFICATE-----(?:\r?\n)?$(?![\s\S])/.exec(pem)
        if (!match) throw new Error("Select one public certificate")
        const encoded = match[1].replace(/[\r\n]/g, "")
        der = fromBase64(encoded)
        if (toBase64(der) !== encoded) throw new Error("Invalid certificate encoding")
    }
    // A whole DER SEQUENCE only; no trailing objects or BER length aliases.
    const extra = der[1] & 0x80 ? der[1] & 0x7f : 0
    if (der.length < 4 || der.length > 32768 || der[0] !== 0x30 ||
        (der[1] & 0x80 && (extra < 1 || extra > 2 || der[2] === 0)))
        throw new Error("Invalid public certificate")
    let length = extra ? 0 : der[1]
    for (let i = 0; i < extra; i++) length = (length << 8) | der[i + 2]
    if ((extra && length < 128) || length + 2 + extra !== der.length)
        throw new Error("Select exactly one public certificate")
    const {Certificate} = await import("pkijs")
    Certificate.fromBER(toArrayBuffer(der))
    // This is only the public file type and review hash. The server checks
    // issuer, current CRL, enrollment/SPKI, certificate usage and validity.
    return {leaf_der: toBase64(der), leaf_sha256: hex(await sha256(der))}
}

export function localOfficerSettings(
    origin: string,
    tenant: string,
    settings: Parameters<typeof matchLocalInitializeSettings>[2] & {
        FELIX3_LOCAL_OFFICER_BRIDGE?: unknown
    }
) {
    const requested = settings.FELIX3_LOCAL_OFFICER_BRIDGE === true
    const pin = requested ? matchLocalInitializeSettings(origin, tenant, settings) : undefined
    return pin?.setupClient && pin.deviceProfile === "softhsm-test" ? pin : undefined
}

export function createLocalInitializeOfficerApi(
    client: Pick<ApolloClient<unknown>, "query" | "mutate">,
    origin: string | undefined, registration: unknown
) {
    const pin = localInitializeRegistration(origin, registration)
    if (!pin?.setupClient || pin.deviceProfile !== "softhsm-test") return
    const context = (role: string) => ({
        uri: pin.graphql, credentials: "omit", queryDeduplication: false,
        headers: {"x-hasura-role": role},
        fetchOptions: {credentials: "omit", redirect: "error", referrerPolicy: "no-referrer"},
    })
    const officer = context("signing-certificates-register")
    return {
        nomineeStatus: async (request_id: string, payload_sha256: string): Promise<NomineeReviewPhase> => {
            if (!isId(request_id) || !isReviewedDigest(payload_sha256)) throw new Error("Invalid request")
            const {data, errors} = await client.query({query: NOMINEE_REVIEW,
                context: context("sign-initialize-voting"), fetchPolicy: "no-cache",
                variables: {request_id, payload_sha256}})
            const phase: unknown = data?.operationalInitializeNomineeReviewStatus?.phase
            if (errors?.length || (phase !== "unavailable" && phase !== "gold-required" && phase !== "resume-ready" && phase !== "ready"))
                throw new Error("Certificate review status unavailable")
            return phase
        },
        nomineeResume: async (request_id: string, payload_sha256: string) => {
            if (!isId(request_id) || !isReviewedDigest(payload_sha256)) throw new Error("Invalid request")
            const {data, errors} = await client.mutate({mutation: NOMINEE_RESUME,
                context: context("sign-initialize-voting"), fetchPolicy: "no-cache",
                variables: {request_id, payload_sha256}})
            if (errors?.length || data?.operationalInitializeNomineeResume?.active !== true)
                throw new Error("Original key resume unavailable")
        },
        presence: async (request_id: string, payload_sha256: string) => {
            if (!isId(request_id) || !isReviewedDigest(payload_sha256)) throw new Error("Invalid request")
            const {data, errors} = await client.mutate({mutation: PRESENCE,
                context: context("sign-initialize-voting"), fetchPolicy: "no-cache",
                variables: {request_id, payload_sha256}})
            if (errors?.length || data?.operationalInitializeNomineePresence?.active !== true)
                throw new Error("Presence unavailable")
        },
        pending: async (election_event_id: string): Promise<LocalOfficerRef[]> => {
            if (!isId(election_event_id)) throw new Error("Invalid event")
            const {data, errors} = await client.query({query: PENDING, context: officer,
                fetchPolicy: "no-cache", variables: {election_event_id}})
            const items = data?.operationalInitializeOfficerPending?.items
            if (errors?.length || !Array.isArray(items) || items.length > 128)
                throw new Error("Pending requests unavailable")
            return items.map(ref)
        },
        review: async (selected: LocalOfficerRef): Promise<ReviewedCsr> => {
            const {data, errors} = await client.mutate({mutation: REVIEW, context: officer,
                fetchPolicy: "no-cache", variables: variables(ref(selected))})
            const raw = record(data?.operationalInitializeOfficerReview)
            const result = ref(raw)
            const {enrollment, csr_pem, csr_sha256, spki_sha256} = raw
            if (errors?.length || !(Object.keys(result) as (keyof LocalOfficerRef)[]).every(
                (key) => selected[key] === result[key]
            ) || !isId(enrollment) || !isReviewedDigest(csr_sha256) || !isReviewedDigest(spki_sha256) ||
                typeof csr_pem !== "string" || csr_pem.length > 16384 ||
                !/^-----BEGIN CERTIFICATE REQUEST-----\n[A-Za-z0-9+/=\n]+\n-----END CERTIFICATE REQUEST-----\n$(?![\s\S])/.test(csr_pem) ||
                hex(await sha256(new TextEncoder().encode(csr_pem))) !== csr_sha256)
                throw new Error("Public request could not be verified")
            return {...result, enrollment, csr_pem, csr_sha256, spki_sha256}
        },
        attach: async (review: ReviewedCsr, leaf: PublicLeaf): Promise<AttachmentReceipt> => {
            const {data, errors} = await client.mutate({mutation: ATTACH,
                context: officer, fetchPolicy: "no-cache",
                variables: {...variables(ref(review)), csr_sha256: review.csr_sha256, ...leaf}})
            const {enrollment, certificate, leaf_sha256, spki_sha256, attached_at} =
                record(data?.operationalInitializeOfficerAttach)
            if (errors?.length || !isId(enrollment) || enrollment !== review.enrollment || !isId(certificate) ||
                leaf_sha256 !== leaf.leaf_sha256 || spki_sha256 !== review.spki_sha256 ||
                typeof attached_at !== "string" || !Number.isFinite(Date.parse(attached_at)))
                throw new Error("Attachment not confirmed")
            return {enrollment, certificate, leaf_sha256: leaf.leaf_sha256,
                spki_sha256: review.spki_sha256, attached_at}
        },
    }
}
export type LocalOfficerApi = NonNullable<ReturnType<typeof createLocalInitializeOfficerApi>>
