import {ApolloClient, InMemoryCache, createHttpLink} from "@apollo/client"
import {setContext} from "@apollo/client/link/context"

const httpLink = createHttpLink({
    uri: "http://localhost:8080/v1/graphql",
})

const authLink = setContext((_, {headers}) => {
    // get the authentication token from local storage if it exists
    const token = localStorage.getItem("token")

    // get the tenant and election-event from the local store
    const tenantId = localStorage.getItem("tenantId")
    const electionEventId = localStorage.getItem("electionEventId")

    // return the headers to the context so httpLink can read them
    return {
        headers: {
            ...headers,
            "authorization": token ? `Bearer ${token}` : "",
            "x-hasura-tenant-id": tenantId || "whatever",
            "x-hasura-election-event-id": electionEventId || "defaultdb",
            "x-hasura-role": "test-all",
        },
    }
})

export const apolloClient = new ApolloClient({
    link: authLink.concat(httpLink),
    cache: new InMemoryCache(),
})
