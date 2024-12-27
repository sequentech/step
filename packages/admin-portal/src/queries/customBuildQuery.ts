// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
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

export interface ParamsSort {
    field: string
    order: string
}

export const customBuildQuery =
    (introspectionResults: any) => (raFetchType: any, resourceName: any, params: any) => {
        let sort: ParamsSort | undefined | null = params.sort
        if (isString(resourceName) && raFetchType === "GET_LIST") {
            if (
                sort?.field &&
                COLUMNS_MAP[resourceName] &&
                !COLUMNS_MAP[resourceName].includes(sort.field)
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
                ]
                ret.variables.order_by = Object.fromEntries(
                    Object.entries(ret?.variables?.order_by || {}).filter(([key]) =>
                        validOrderBy.includes(key)
                    )
                )
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

            return ret
        }
        return buildQuery(introspectionResults)(raFetchType, resourceName, params)
    }
