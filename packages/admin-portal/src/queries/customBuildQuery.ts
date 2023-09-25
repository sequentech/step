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
        parseResponse: (response: any) => {
          console.log(response.data)
          return getResponseParser(response);
        },
    }
    }
    return buildQuery(introspectionResults)(raFetchType, resourceName, params)
  }