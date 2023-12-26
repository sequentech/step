import {ApolloClient, InMemoryCache, createHttpLink} from "@apollo/client"
import {setContext} from "@apollo/client/link/context"

const httpLink = createHttpLink({
    uri: "https://hasura-staging.sequent.vote/v1/graphql",
})

const authLink = setContext((_, {headers}) => {
    // get the authentication token from local storage if it exists
    const token = localStorage.getItem("token")
    // return the headers to the context so httpLink can read them
    return {
        headers: {
            ...headers,
            authorization: token ? `Bearer ${token}` : "",
        },
    }
})

export const apolloClient = new ApolloClient({
    link: authLink.concat(httpLink),
    cache: new InMemoryCache(),
})
