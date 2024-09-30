import {gql} from "@apollo/client"

export const CREATE_RESOURCE = gql`
    mutation CreateResource($resourceId: String!) {
        createResource(resource_id: $resourceId) {
            id
        }
    }
`
