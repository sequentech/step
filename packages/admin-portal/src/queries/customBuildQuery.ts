import {
    buildQuery,
    buildVariables,
    getResponseParser,
    buildFields
    
} from "ra-data-hasura"
import {getList} from './ListPgAudit'

export const customBuildQuery = (introspectionResults: any) => 
  (raFetchType: any, resourceName: any, params: any) => {
    if (resourceName === 'pgaudit' && raFetchType === 'GET_LIST') {
      // TODO:
      // const fields = buildFieldList(introspectionResults, resource, raFetchType)
      return {
        query: getList({}),
        variables: params,
        parseResponse: (res: any) => {
          const response = res.data.listPgaudit;
          let output = {
              data: response.items,
              total: response.total.aggregate.count
          };
          return output;
        },
      }
    }
    return buildQuery(introspectionResults)(raFetchType, resourceName, params)
  }