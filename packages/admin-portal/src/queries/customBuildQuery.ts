import {buildQuery, buildVariables} from "ra-data-hasura"
import {getList} from "./ListPgAudit"
import { getUsers } from "./GetUsers"

export const customBuildQuery =
    (introspectionResults: any) => (raFetchType: any, resourceName: any, params: any) => {
        if (resourceName === "pgaudit" && raFetchType === "GET_LIST") {
            const resource: any = {
                type: {
                    fields: [],
                    name: "pgaudit",
                },
            }
            return {
                query: getList({}),
                variables: buildVariables(introspectionResults)(
                    resource,
                    raFetchType,
                    params,
                    null
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
        }
        if (resourceName === "user" && raFetchType === "GET_LIST") {
            const resource: any = {
                type: {
                    fields: [],
                    name: "user",
                },
            }
            return {
                query: getUsers(params.filter),
                variables: buildVariables(introspectionResults)(
                    resource,
                    raFetchType,
                    params,
                    null
                ),
                parseResponse: (res: any) => {
                    const response = res.data.get_users
                    let output = {
                        data: res.data.get_users,
                        total: res.data.get_users.length,
                    }
                    return output
                },
            }
        }
        return buildQuery(introspectionResults)(raFetchType, resourceName, params)
    }
