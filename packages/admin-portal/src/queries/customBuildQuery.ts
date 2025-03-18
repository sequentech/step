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
import {gql} from "@apollo/client"

export interface ParamsSort {
    field: string
    order: string
}

function injectSqlVariables(query: string, ...args: any) {
    let paramIndex = 0
    return query.replace(/\$\d+/g, () => {
        const value = args[paramIndex++]
        if (typeof value === "string") {
            return `'${value.replace(/'/g, "''")}'`
        } else if (value === null) {
            return "NULL"
        } else {
            return value
        }
    })
}

function removeNewlines(sqlString: string) {
    // Use the replace() method with a regular expression to remove all newline characters
    return sqlString.replace(/\n/g, "")
}

export const customBuildQuery =
    (introspectionResults: any) =>
    (
        raFetchType: any,
        resourceName: any,
        params: any
    ): {
        query: any
        variables: any
        parseResponse: (res: any) => any
        countQuery?: any
        sql?: boolean
    } => {
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
            const {
                filter = {},
                pagination = {page: 1, perPage: 20},
                sort = {field: "created_at", order: "DESC"},
            } = params

            console.log("aa IN CUSOM FILTERS")

            // Extract JSONB filters
            const jsonbFilters: {[key: string]: string} = {}
            const regularFilters: {[key: string]: string} = {}

            // Check which filters are for JSONB fields and which are for regular columns
            Object.entries(filter).forEach(([key, value]) => {
                if (key.startsWith("applicant_data")) {
                    // Extract the field name after 'applicant_data.'
                    // const fieldName = key.split(".")[1]
                    Object.keys(filter[key]).forEach((fieldKey) => {
                        const newField = fieldKey
                        const newValue = filter[key][newField]
                        jsonbFilters[newField] = newValue["_ilike"]
                    })
                } else if (["verification_type", "applicant_id", "id", "status"].includes(key)) {
                    if (value && typeof value === "string" && value.trim() !== "") {
                        regularFilters[key as string] = value
                    }
                }
            })

            console.log("aa jsonbFilters", jsonbFilters)
            console.log("aa regularFilters", regularFilters)

            const hasJsonbFilters = Object.keys(jsonbFilters).length > 0

            // If no JSONB filters are present, use the standard approach
            // if (!hasJsonbFilters) {
            //     return buildQuery(introspectionResults)(raFetchType, resourceName, params)
            // }

            // Build SQL query dynamically
            let sql = ""
            //     let sql = `
            // SELECT
            //     id,
            //     applicant_id,
            //     verification_type,
            //     status,
            //     created_at,
            //     updated_at,
            //     applicant_data,
            //     annotations,
            //     area_id,
            //     election_event_id,
            //     tenant_id,
            //     labels,
            //     permission_label
            // FROM
            //     sequent_backend.applications
            // WHERE 1=1
            //         `

            //         // Array to hold parameter values
            //         const paramValues: string[] = []
            //         let paramCounter = 1

            //         // Add regular column filters
            //         Object.entries(regularFilters).forEach(([key, value]) => {
            //             if (key === "id" || key === "applicant_id") {
            //                 sql += ` AND ${key} = $${paramCounter}`
            //                 paramValues.push(value)
            //                 paramCounter++
            //             } else {
            //                 sql += ` AND ${key} ILIKE $${paramCounter}`
            //                 paramValues.push(`%${value}%`)
            //                 paramCounter++
            //             }
            //         })

            //         // Add JSONB filters
            //         Object.entries(jsonbFilters).forEach(([key, value]) => {
            //             // For date fields, use exact match
            //             if (key === "dateOfBirth") {
            //                 sql += ` AND applicant_data->>'${key}' = $${paramCounter}`
            //                 paramValues.push(value)
            //             } else {
            //                 // For text fields, use ILIKE
            //                 sql += ` AND applicant_data->>'${key}' ILIKE $${paramCounter}`
            //                 paramValues.push(`%${value}%`)
            //             }
            //             paramCounter++
            //         })

            //         // Add count query for pagination
            //         const countSql = `
            //             SELECT COUNT(*)
            //             FROM (${sql}) AS filtered_results
            //         `

            //         // Add order and pagination to the main query
            //         const orderByField =
            //             sort.field === "id" || sort.field === "created_at" || sort.field === "updated_at"
            //                 ? sort.field
            //                 : "created_at"

            //         sql += ` ORDER BY ${orderByField} ${sort.order}`
            //         sql += ` LIMIT ${pagination.perPage} OFFSET ${
            //             (pagination.page - 1) * pagination.perPage
            //         }`

            //         const mainQuery = gql`
            //             mutation ExecuteRawQuery($sql: String!, $args: [String!]) {
            //                 run_sql(sql: $sql) {
            //                     rows
            //                 }
            //             }
            //         `

            // const countQuery = gql`
            //     mutation ExecuteCountQuery($countSql: String!, $args: [String!]) {
            //         run_sql(sql: $countSql) {
            //             rows
            //         }
            //     }
            // `

            // sql = removeNewlines(injectSqlVariables(sql, ...paramValues))

            // return {
            //     sql: true,
            //     query: mainQuery,
            //     countQuery: countQuery,
            //     variables: {sql},
            //     parseResponse: (res: any) => {
            //         // Defensive checks to prevent undefined errors
            //         if (!res) {
            //             return {data: [], total: 0}
            //         }

            //         // For v2 API, the response should be under run_sql
            //         const runSqlResult = res

            //         if (!runSqlResult) {
            //             return {data: [], total: 0}
            //         }

            //         let rows: any[] = [...res]

            //         if (!Array.isArray(rows)) {
            //             return {data: [], total: 0}
            //         }

            //         const numRows = res.length - 1

            //         let data: any[] = []

            //         rows = rows.slice(1)

            //         if (numRows > 0) {
            //             // Now safe to map over rows
            //             data = rows.map((row: any[], index: number) => {
            //                 // Add additional safety checks for each row
            //                 if (!Array.isArray(row) || row.length < 13) {
            //                     // Return a default object with required fields
            //                     return {
            //                         id: `unknown-${index}`,
            //                         __typename: "sequent_backend_applications",
            //                     }
            //                 }

            //                 // Safely parse JSON fields
            //                 const safeParseJson = (value: any) => {
            //                     if (typeof value === "string") {
            //                         try {
            //                             return JSON.parse(value)
            //                         } catch (e) {
            //                             return {}
            //                         }
            //                     }
            //                     return value || {}
            //                 }

            //                 return {
            //                     id: row[0] || `unknown-${index}`,
            //                     applicant_id: row[1] || null,
            //                     verification_type: row[2] || null,
            //                     status: row[3] || null,
            //                     created_at: row[4] || null,
            //                     updated_at: row[5] || null,
            //                     applicant_data: safeParseJson(row[6]),
            //                     annotations: safeParseJson(row[7]),
            //                     area_id: row[8] || null,
            //                     election_event_id: row[9] || null,
            //                     tenant_id: row[10] || null,
            //                     labels: safeParseJson(row[11]),
            //                     permission_label: row[12] || null,
            //                     __typename: "sequent_backend_applications",
            //                 }
            //             })
            //         }

            //         return {
            //             data,
            //             total: data.length, // Will be set in App.tsx
            //         }
            //     },
            // }
        }
        return buildQuery(introspectionResults)(raFetchType, resourceName, params)
    }
