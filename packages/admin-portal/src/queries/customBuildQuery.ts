// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {buildQuery, buildVariables} from "ra-data-hasura"
import {getPgauditVariables, getPgAudit} from "./ListPgAudit"
import {getElectoralLogVariables, getElectoralLog} from "./ListElectoralLog"
import {getUsers} from "./GetUsers"
import {getPermissions} from "./GetPermissions"
import {getRoles} from "./GetRoles"
import {isString} from "lodash"
import {COLUMNS_MAP} from "@/types/query"

export interface ParamsSort {
    field: string
    order: string
}

export const customBuildQuery =
    (introspectionResults: any) => (raFetchType: any, resourceName: any, params: any) => {
        let sort: ParamsSort | undefined | null = params.sort
        if (
            isString(resourceName) &&
            raFetchType === "GET_LIST" &&
            sort?.field &&
            COLUMNS_MAP[resourceName] &&
            !COLUMNS_MAP[resourceName].includes(sort.field)
        ) {
            params.sort = undefined
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
                query: getUsers(params),
                variables: buildVariables(introspectionResults)(
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
        }
        return buildQuery(introspectionResults)(raFetchType, resourceName, params)
    }
