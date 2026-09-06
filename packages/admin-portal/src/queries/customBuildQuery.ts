import {Order_By} from "./../../../voting-portal/src/gql/graphql"
// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {buildQuery, buildVariables} from "ra-data-hasura"
import {getPgauditVariables, getPgAudit} from "./ListPgAudit"
import {getElectoralLogVariables, getElectoralLog} from "./ListElectoralLog"
import {LIST_USERS, customBuildGetUsersVariables} from "./GetUsers"
import {getPermissions} from "./GetPermissions"
import {getRoles} from "./GetRoles"
import {isString} from "lodash"
import {COLUMNS_MAP} from "@/types/query"
import {GetCastVotesByIp} from "./GetCastVotesByIp"
import {GET_ELECTIONS_BY_EXTERNAL_ID} from "./GetElectionsByExternalID"

export interface ParamsSort {
    field: string
    order: string
}

const UUID_REGEXP = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i

/**
 * `labels`/`annotations` are free-form jsonb columns. ra-data-hasura's default
 * jsonb filter tries to JSON.parse the typed text and use `_contains`, which
 * silently no-ops for plain text (see typeAwareKeyValueReducer in
 * ra-data-hasura). Route these through a raw query that casts the column to
 * text and does a substring search instead.
 */
function applyJsonbTextSearchFilter(filter: Record<string, unknown>, field: string): void {
    const value = filter[field]
    if (typeof value === "string" && value.length > 0) {
        filter[field] = {
            format: "hasura-raw-query",
            value: {_cast: {String: {_ilike: `%${value}%`}}},
        }
    }
}

/**
 * `uuid` columns reject non-uuid input at the Postgres level, so a partial
 * value typed by the user (before finishing pasting a full id) would error
 * out the whole list query. Only forward the filter once it's a full uuid.
 */
function applyUuidFilter(filter: Record<string, unknown>, field: string): void {
    const value = filter[field]
    if (typeof value !== "string" || value.length === 0) {
        return
    }
    if (UUID_REGEXP.test(value)) {
        filter[field] = {format: "hasura-raw-query", value: {_eq: value}}
    } else {
        delete filter[field]
    }
}

export const customBuildQuery =
    (introspectionResults: any) => (raFetchType: any, resourceName: any, params: any) => {
        let sort: ParamsSort | undefined | null = params.sort
        if (isString(resourceName) && raFetchType === "GET_LIST") {
            if (
                !sort?.field ||
                (COLUMNS_MAP[resourceName] && !COLUMNS_MAP[resourceName].includes(sort.field))
            ) {
                params.sort = undefined
            }

            let validFilters = COLUMNS_MAP[resourceName]
            if (validFilters) {
                Object.keys(params.filter).forEach((f) => {
                    if (!validFilters.includes(f)) {
                        console.log(`removing ${resourceName}.filter.${f}`)
                        delete params.filter[f]
                    }
                })
            }
        }

        if (resourceName.startsWith("pgaudit") && raFetchType === "GET_LIST") {
            const resource: any = {
                type: {
                    fields: [],
                    name: resourceName,
                },
            }
            return {
                query: getPgAudit(params, resourceName),
                variables: getPgauditVariables(
                    buildVariables(introspectionResults)(resource, raFetchType, params, null)
                ),
                parseResponse: (res: any) => {
                    const response = res.data.listPgaudit
                    let output = {
                        data: response.items,
                        total: response.total.aggregate.count,
                    }
                    return output
                },
            }
        } else if (resourceName === "electoral_log" && raFetchType === "GET_LIST") {
            let validFilters = [
                "election_event_id",
                "user_id",
                "username",
                "created",
                "statement_timestamp",
                "statement_kind",
            ]
            Object.keys(params.filter).forEach((f) => {
                if (!validFilters.includes(f)) {
                    delete params.filter[f]
                }
            })
            const resource: any = {
                type: {
                    fields: [],
                    name: resourceName,
                },
            }
            return {
                query: getElectoralLog(params),
                variables: getElectoralLogVariables(
                    buildVariables(introspectionResults)(resource, raFetchType, params, null)
                ),
                parseResponse: (res: any) => {
                    const response = res.data.listElectoralLog
                    let output = {
                        data: response.items,
                        total: response.total.aggregate.count,
                    }
                    return output
                },
            }
        } else if (resourceName === "sequent_backend_report" && raFetchType === "GET_LIST") {
            let ret = buildQuery(introspectionResults)(raFetchType, resourceName, params)
            if (ret?.variables?.order_by) {
                const validOrderBy = [
                    "id",
                    "created_at",
                    "election_id",
                    "report_type",
                    "template_alias",
                    "encryption_policy",
                ]
                ret.variables.order_by = Object.fromEntries(
                    Object.entries(ret?.variables?.order_by || {}).filter(([key]) =>
                        validOrderBy.includes(key)
                    )
                )
                if (ret?.variables?.order_by) {
                    ret.variables.order_by = [{...ret.variables.order_by}, {id: "asc"}]
                }
            }
            return ret
        } else if (
            resourceName === "sequent_backend_tasks_execution" &&
            raFetchType === "GET_LIST"
        ) {
            let ret = buildQuery(introspectionResults)(raFetchType, resourceName, params)
            if (ret?.variables?.order_by) {
                const validOrderBy = [
                    "annotations",
                    "created_at",
                    "election_event_id",
                    "end_at",
                    "executed_by_user",
                    "execution_status",
                    "id",
                    "labels",
                    "logs",
                    "name",
                    "start_at",
                    "tenant",
                    "tenant_id",
                    "type",
                ]
                ret.variables.order_by = Object.fromEntries(
                    Object.entries(ret?.variables?.order_by || {}).filter(([key]) =>
                        validOrderBy.includes(key)
                    )
                )
            }
            return ret
        } else if (
            resourceName === "sequent_backend_scheduled_event" &&
            raFetchType === "GET_LIST"
        ) {
            let ret = buildQuery(introspectionResults)(raFetchType, resourceName, params)

            if (ret?.variables?.order_by) {
                ret.variables.order_by = [{...ret.variables.order_by}, {id: "asc"}]
            }

            let electionIds: Array<string> | undefined =
                params?.filter?.event_payload?.value?._contains?.election_id
            if (electionIds) {
                let newAnd = ret.variables.where._and.filter(
                    (and: object) => !("event_payload" in and)
                )
                newAnd.push({
                    _or: [
                        ...electionIds.map((electionId) => ({
                            event_payload: {
                                _contains: {
                                    election_id: electionId,
                                },
                            },
                        })),
                        {
                            event_payload: {
                                _contains: {
                                    election_id: null,
                                },
                            },
                        },
                    ],
                })
                ret.variables.where._and = newAnd
            }
            return ret
        } else if (
            resourceName === "sequent_backend_ballot_publication" &&
            raFetchType === "GET_LIST"
        ) {
            let ret = buildQuery(introspectionResults)(raFetchType, resourceName, params)
            if (ret?.variables?.where?._and) {
                if (!params?.filter?.election_id) {
                    ret.variables.where._and.push({
                        election_id: {_is_null: true},
                    })
                } else {
                    let indexToReplace = ret.variables.where._and.findIndex(
                        (el: {election_id?: any}) => el?.election_id
                    )
                    ret.variables.where._and[indexToReplace] = {
                        election_ids: {_contains: [params?.filter?.election_id]},
                    }
                }
            }
            return ret
        } else if (resourceName === "user" && raFetchType === "GET_LIST") {
            const resource: any = {
                type: {
                    fields: [],
                    name: "user",
                },
            }
            return {
                query: LIST_USERS,
                variables: customBuildGetUsersVariables(introspectionResults)(
                    resource,
                    raFetchType,
                    params,
                    null
                ),
                parseResponse: (res: any) => {
                    const response = res.data.get_users
                    let output = {
                        data: response.items,
                        total: response.total.aggregate.count,
                    }
                    return output
                },
            }
        } else if (resourceName === "role" && raFetchType === "GET_LIST") {
            const resource: any = {
                type: {
                    fields: [],
                    name: "role",
                },
            }
            return {
                query: getRoles(params.filter),
                variables: buildVariables(introspectionResults)(
                    resource,
                    raFetchType,
                    params,
                    null
                ),
                parseResponse: (res: any) => {
                    const response = res.data.get_roles
                    let output = {
                        data: response.items,
                        total: response.total.aggregate.count,
                    }
                    return output
                },
            }
        } else if (resourceName === "permission" && raFetchType === "GET_LIST") {
            const resource: any = {
                type: {
                    fields: [],
                    name: "role",
                },
            }
            return {
                query: getPermissions(params.filter),
                variables: buildVariables(introspectionResults)(
                    resource,
                    raFetchType,
                    params,
                    null
                ),
                parseResponse: (res: any) => {
                    const response = res.data.get_permissions
                    let output = {
                        data: response.items,
                        total: response.total.aggregate.count,
                    }
                    return output
                },
            }
        } else if (resourceName === "ip_address" && raFetchType === "GET_LIST") {
            const resource: any = {
                type: {
                    fields: [],
                    name: "ip_address",
                },
            }

            return {
                query: GetCastVotesByIp(params),
                variables: buildVariables(introspectionResults)(
                    resource,
                    raFetchType,
                    params,
                    null
                ),
                parseResponse: (res: any) => {
                    const response = res.data.get_top_votes_by_ip
                    let output = {
                        data: response.items,
                        total: response.total.aggregate.count,
                    }
                    return output
                },
            }
        } else if (resourceName === "sequent_backend_applications" && raFetchType === "GET_LIST") {
            let ret = buildQuery(introspectionResults)(raFetchType, resourceName, params)

            if (ret?.variables?.order_by) {
                const validOrderBy = [
                    "id",
                    "created_at",
                    "updated_at",
                    "applicant_id",
                    "verification_type",
                    "status",
                ]
                ret.variables.order_by = Object.fromEntries(
                    Object.entries(ret?.variables?.order_by || {}).filter(([key]) =>
                        validOrderBy.includes(key)
                    )
                )
            }

            const {filter} = params
            const transformedRawParams = {...ret?.variables.where}
            const transformedParams = ret?.variables.where["_and"]

            // Transform applicant_data
            Object.keys(filter).forEach((key) => {
                if (key === "applicant_data" && typeof filter[key] === "object") {
                    const flattened = flattenObject(filter[key])
                    Object.keys(flattened).forEach((newField) => {
                        transformedParams.push({
                            applicant_data: {
                                _contains: {[newField]: flattened[newField]},
                            },
                        })
                    })
                }
            })

            ret.variables.where = transformedRawParams

            return ret
        } else if (
            resourceName === "sequent_backend_election_by_external_id" &&
            raFetchType === "GET_MANY"
        ) {
            const externalIds: string[] = (params?.ids ?? []).map(String)
            const electionEventId =
                params?.meta?.filter?.election_event_id ??
                params?.meta?.filter?.electionEventId ??
                null

            return {
                query: GET_ELECTIONS_BY_EXTERNAL_ID,
                variables: {
                    external_ids: externalIds,
                    election_event_id: electionEventId,
                },
                parseResponse: (res: any) => {
                    const rows = res?.data?.sequent_backend_election ?? []
                    return {
                        data: rows.map((r: any) => ({
                            ...r,
                            id: r.external_id,
                        })),
                    }
                },
            }
        } else if (
            resourceName === "sequent_backend_tally_session_execution" &&
            raFetchType === "GET_LIST" &&
            params?.meta?.latestPerTallySession
        ) {
            params.filter = {
                ...params.filter,
                distinct_on: ["tally_session_id"],
            }
            const ret = buildQuery(introspectionResults)(raFetchType, resourceName, params)
            if (ret?.variables?.order_by) {
                ret.variables.order_by = [
                    {tally_session_id: "asc"},
                    {created_at: "desc"},
                    {id: "desc"},
                ]
            }
            return ret
        } else if (resourceName === "sequent_backend_tally_sheet" && raFetchType === "GET_LIST") {
            applyJsonbTextSearchFilter(params.filter, "labels")
            applyJsonbTextSearchFilter(params.filter, "annotations")
            applyUuidFilter(params.filter, "import_id")

            if (params?.meta?.distinctBallotBoxes) {
                // Show only one row per ballot box (area/contest/channel)
                params.filter = {
                    ...params.filter,
                    distinct_on: ["area_id", "contest_id", "channel"],
                }
            }

            let ret = buildQuery(introspectionResults)(raFetchType, resourceName, params)
            if (params?.meta?.distinctBallotBoxes && ret?.variables?.order_by) {
                ret.variables.order_by = [
                    {area_id: "asc"},
                    {contest_id: "asc"},
                    {channel: "asc"},
                    {version: "desc"},
                ]
            }
            return ret
        }
        return buildQuery(introspectionResults)(raFetchType, resourceName, params)
    }

function flattenObject(obj: any, prefix = "") {
    let result: any = {}

    Object.keys(obj).forEach((key) => {
        const newKey = prefix ? `${prefix}.${key}` : key
        if (typeof obj[key] === "object" && obj[key] !== null && !("_ilike" in obj[key])) {
            // Recursively flatten only if it's an object and doesn't have `_ilike`
            Object.assign(result, flattenObject(obj[key], newKey))
        } else if ("_ilike" in obj[key]) {
            // Extract `_ilike` value
            result[newKey] = obj[key]["_ilike"]
        }
    })

    return result
}
