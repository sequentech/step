// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The reads and writes of Election Event > Signatures. Each signing table is
// selected, and each signing action sent, with its own permission as the Hasura
// role, so staff without admin-user don't get the portal's read-only default;
// Posts (elections), countries (areas) and people come from the portal's
// existing lists.
import {useCallback, useContext, useEffect, useMemo, useState} from "react"
import {useApolloClient, useMutation, useQuery, type DocumentNode} from "@apollo/client"
import {useGetList, useNotify} from "react-admin"
import {useTranslation} from "react-i18next"
import {AuthContext} from "@/providers/AuthContextProvider"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {useAliasRenderer} from "@/hooks/useAliasRenderer"
import {IPermissions} from "@/types/keycloak"
import type {Sequent_Backend_Area, Sequent_Backend_Election} from "@/gql/graphql"
import {
    SIGNING_ACTIONS,
    SigningAction,
    SigningScope,
    type IImportSigningIssuersOutput,
    type ISaveSigningRuleOutput,
    type ISigningChecksRow,
    type ISigningRequestListRow,
    type ISigningRequestsExport,
    type ISigningRuleCapacity,
    type ISigningRuleRow,
    type IStaffCertificate,
    type IStaffCrl,
    type IStaffIssuer,
    type IWaitingSigningRequest,
} from "@/lib/signing/types"
import {
    GET_SIGNING_CERTIFICATES,
    GET_SIGNING_REQUESTS,
    GET_SIGNING_RULES,
    GET_SIGNING_RULE_CAPACITIES,
    GET_WAITING_SIGNING_REQUESTS,
    SIGNING_EVENT_INFO,
    SIGNING_DELETE_ISSUER,
    SIGNING_EXPORT_REQUESTS,
    SIGNING_IMPORT_ISSUERS,
    SIGNING_PUT_CHECKS,
    SIGNING_PUT_RULE,
    SIGNING_REGISTER_CERTIFICATE,
    SIGNING_REVOKE_CERTIFICATE,
} from "@/queries/SigningSettings"
import {errorKey, signaturesAccess, signingError} from "./signingSettings"

/** Every list of the tab fits in one page. */
const ALL = {page: 1, perPage: 1000}

const asRole = (role: IPermissions) => ({headers: {"x-hasura-role": role}})

/** What the signed-in user may see and change in the tab. */
export function useSignaturesAccess() {
    const auth = useContext(AuthContext)
    const [tenantId] = useTenantStore()
    return useMemo(
        () => signaturesAccess((permission) => auth.isAuthorized(true, tenantId, permission)),
        [auth, tenantId]
    )
}

export function useSigningRules(electionEventId: string) {
    const {data, loading, error, refetch} = useQuery<{
        sequent_backend_signing_rule: ISigningRuleRow[]
    }>(GET_SIGNING_RULES, {
        variables: {electionEventId},
        context: asRole(IPermissions.SIGNING_RULES_READ),
        fetchPolicy: "network-only",
    })
    return {rules: data?.sequent_backend_signing_rule, loading, error, refetch}
}

/** The alias of an action's capacity in GetSigningRuleCapacities. */
export const capacityAlias = (action: SigningAction) => action.replace(/-/g, "_")

export function useRuleCapacities(electionEventId: string) {
    const {data, error, refetch} = useQuery<Record<string, ISigningRuleCapacity | null>>(
        GET_SIGNING_RULE_CAPACITIES,
        {
            variables: {electionEventId},
            context: asRole(IPermissions.SIGNING_RULES_READ),
            fetchPolicy: "network-only",
        }
    )
    const capacityOf = useCallback(
        (action: SigningAction) => data?.[capacityAlias(action)] ?? undefined,
        [data]
    )
    // Every action's answer carries the event's configuration version.
    const configVersion =
        Object.values(SigningAction)
            .map((action) => data?.[capacityAlias(action)]?.config_version)
            .find((version) => version !== undefined && version !== null) ?? null
    return {capacityOf, configVersion, loaded: !!data, error, refetch}
}

export interface ISigningCertificatesData {
    checks: ISigningChecksRow[]
    issuers: IStaffIssuer[]
    certificates: IStaffCertificate[]
    crls: IStaffCrl[]
}

export function useSigningCertificates(electionEventId: string) {
    const {data, loading, error, refetch} = useQuery<ISigningCertificatesData>(
        GET_SIGNING_CERTIFICATES,
        {
            variables: {electionEventId},
            context: asRole(IPermissions.SIGNING_CERTIFICATES_READ),
            fetchPolicy: "network-only",
        }
    )
    return {data, loading, error, refetch}
}

export function useSigningRequests(electionEventId: string) {
    const {data, loading, error} = useQuery<{
        sequent_backend_signing_request: ISigningRequestListRow[]
    }>(GET_SIGNING_REQUESTS, {
        variables: {electionEventId},
        context: asRole(IPermissions.SIGNING_REQUESTS_READ),
        fetchPolicy: "network-only",
    })
    return {requests: data?.sequent_backend_signing_request, loading, error}
}

/**
 * The sign permissions whose requests a signer finds in Hasura: every
 * action's but a trustee's, whose requests the trustee's ceremony step reads
 * through Harvest.
 */
export const LISTED_SIGN_PERMISSIONS: IPermissions[] = Object.values(SIGNING_ACTIONS)
    .filter(({scope}) => scope !== SigningScope.Trustee)
    .map(({signPermission}) => signPermission)

/** The permissions the user holds in the selected tenant. */
const useHolds = () => {
    const auth = useContext(AuthContext)
    const [tenantId] = useTenantStore()
    return useCallback(
        (permission: IPermissions) => auth.isAuthorized(true, tenantId, permission),
        [auth, tenantId]
    )
}

/**
 * The event's requests waiting for signatures of the actions the user may
 * sign, in their Posts, oldest first: one query per `sign-<action>` they
 * hold, each as that role (its select permission keeps to its action).
 */
/** The listed sign permissions the user holds ([`LISTED_SIGN_PERMISSIONS`]). */
export function useListedSignPermissions() {
    const holds = useHolds()
    return useMemo(() => LISTED_SIGN_PERMISSIONS.filter(holds), [holds])
}

export function useWaitingSigningRequests(electionEventId: string) {
    const client = useApolloClient()
    const roles = useListedSignPermissions()
    const [requests, setRequests] = useState<IWaitingSigningRequest[] | null>(null)
    const [error, setError] = useState(false)
    const [version, setVersion] = useState(0)
    const reload = useCallback(() => setVersion((previous) => previous + 1), [])

    useEffect(() => {
        let current = true
        Promise.all(
            roles.map((role) =>
                client.query<{sequent_backend_signing_request: IWaitingSigningRequest[]}>({
                    query: GET_WAITING_SIGNING_REQUESTS,
                    variables: {electionEventId},
                    // The same query as each role: never merged in flight nor in the cache.
                    context: {...asRole(role), queryDeduplication: false},
                    fetchPolicy: "no-cache",
                })
            )
        ).then(
            (answers) => {
                if (!current) return
                const byId = new Map<string, IWaitingSigningRequest>()
                for (const {data} of answers) {
                    for (const request of data?.sequent_backend_signing_request ?? []) {
                        byId.set(request.id, request)
                    }
                }
                setError(false)
                setRequests(
                    Array.from(byId.values()).sort((a, b) =>
                        a.created_at.localeCompare(b.created_at)
                    )
                )
            },
            () => current && setError(true)
        )
        return () => {
            current = false
        }
    }, [client, electionEventId, roles, version])

    return {requests, error, roles, reload}
}

/** The roles that read the event's signing information, the one that also gets the titles first. */
const EVENT_INFO_ROLES: IPermissions[] = [
    IPermissions.SIGNING_CERTIFICATES_READ,
    IPermissions.SIGNING_REQUESTS_READ,
    IPermissions.SIGNING_RULES_READ,
    ...Object.values(SIGNING_ACTIONS).map(({signPermission}) => signPermission),
]

/**
 * The event's time zone, which the tab and a signer's list show times in
 * (as the signing panel does), and the signers' titles for a reader of the
 * certificates. `timeZone` is null until known, or without such a permission.
 */
export function useSigningEventInfo(electionEventId: string) {
    const holds = useHolds()
    const role = EVENT_INFO_ROLES.find(holds)
    const {data} = useQuery<{
        signingEventInfo?: {time_zone?: string | null; titles?: Record<string, string> | null}
    }>(SIGNING_EVENT_INFO, {
        variables: {electionEventId},
        context: role ? asRole(role) : undefined,
        skip: !role,
    })
    return {
        timeZone: data?.signingEventInfo?.time_zone ?? null,
        titles: data?.signingEventInfo?.titles ?? {},
    }
}

/** A signing action of the tab, sent with its own permission as the Hasura role. */
const useSigningWrite = <TData = unknown>(mutation: DocumentNode, permission: IPermissions) =>
    useMutation<TData>(mutation, {context: asRole(permission)})

export const usePutRule = () =>
    useSigningWrite<{signingPutRule: ISaveSigningRuleOutput}>(
        SIGNING_PUT_RULE,
        IPermissions.SIGNING_RULES_WRITE
    )

export const useImportIssuers = () =>
    useSigningWrite<{signingImportIssuers: IImportSigningIssuersOutput}>(
        SIGNING_IMPORT_ISSUERS,
        IPermissions.SIGNING_ISSUERS_WRITE
    )

export const useDeleteIssuer = () =>
    useSigningWrite(SIGNING_DELETE_ISSUER, IPermissions.SIGNING_ISSUERS_WRITE)

export const usePutChecks = () =>
    useSigningWrite(SIGNING_PUT_CHECKS, IPermissions.SIGNING_CHECKS_WRITE)

export const useRegisterCertificate = () =>
    useSigningWrite(SIGNING_REGISTER_CERTIFICATE, IPermissions.SIGNING_CERTIFICATES_REGISTER)

export const useRevokeCertificate = () =>
    useSigningWrite(SIGNING_REVOKE_CERTIFICATE, IPermissions.SIGNING_CERTIFICATES_REVOKE)

export const useExportRequests = () =>
    useSigningWrite<{signingExportRequests?: ISigningRequestsExport | null}>(
        SIGNING_EXPORT_REQUESTS,
        IPermissions.SIGNING_REQUESTS_EXPORT
    )

/** Names of the event's Posts (elections) and countries (areas); the id until they load. */
export function useScopeNames(electionEventId: string, withCountries = false) {
    const aliasRenderer = useAliasRenderer()
    const filter = {election_event_id: electionEventId}
    const {data: elections} = useGetList<Sequent_Backend_Election>("sequent_backend_election", {
        filter,
        pagination: ALL,
    })
    const {data: areas} = useGetList<Sequent_Backend_Area>(
        "sequent_backend_area",
        {filter, pagination: ALL},
        {enabled: withCountries}
    )
    const postName = useCallback(
        (electionId: string | null) => {
            const election = elections?.find(({id}) => id === electionId)
            return election ? aliasRenderer(election) : (electionId ?? "")
        },
        [elections, aliasRenderer]
    )
    const countryName = useCallback(
        (areaId: string | null) =>
            areas?.find(({id}) => id === areaId)?.name ?? (areaId as string | null) ?? "",
        [areas]
    )
    return {elections: elections ?? [], postName, countryName}
}

/** A person to register a certificate to, found by (part of) their username. */
export interface IStaffPerson {
    id: string
    username: string
    first_name?: string | null
    last_name?: string | null
}

/** The tenant's users matching the typed username; nothing until something is typed. */
export function usePeopleSearch(search: string) {
    const [tenantId] = useTenantStore()
    const username = search.trim()
    const {data, isLoading} = useGetList<IStaffPerson>(
        "user",
        {filter: {tenant_id: tenantId, username}, pagination: {page: 1, perPage: 20}},
        {enabled: username.length > 0}
    )
    return {people: username ? (data ?? []) : [], loading: username.length > 0 && isLoading}
}

/**
 * Tells the user why a write was refused: the permission, the input, someone
 * else's save, the lockdown or a missing record; otherwise the write's own
 * message.
 */
export function useWriteError() {
    const {t} = useTranslation()
    const notify = useNotify()
    return useCallback(
        (error: unknown, fallbackKey: string, {lockedDown = false} = {}) =>
            notify(t(errorKey(signingError(error).code, lockedDown) ?? fallbackKey), {
                type: "error",
            }),
        [notify, t]
    )
}
