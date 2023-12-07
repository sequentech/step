type GlobalSettings = {
    QUERY_POLL_INTERVAL_MS: number
    DEFAULT_TENANT_ID: string
    ONLINE_VOTING_CLIENT_ID: string
    KEYCLOAK_URL: string
}

const globalSettings: GlobalSettings = {
    QUERY_POLL_INTERVAL_MS: 2000,
    DEFAULT_TENANT_ID: "90505c8a-23a9-4cdf-a26b-4e19f6a097d5",
    ONLINE_VOTING_CLIENT_ID: "admin-portal",
    KEYCLOAK_URL: "http://127.0.0.1:8090/",
}

export default globalSettings
