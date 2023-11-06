import {buildQuery, buildVariables} from "ra-data-hasura"
import {getList} from "./ListPgAudit"

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
        return buildQuery(introspectionResults)(raFetchType, resourceName, params)
    }
