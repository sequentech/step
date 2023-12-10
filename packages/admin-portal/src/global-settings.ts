type GlobalSettings = {
    QUERY_POLL_INTERVAL_MS: number
    DEFAULT_TENANT_ID: string
    ONLINE_VOTING_CLIENT_ID: string
    KEYCLOAK_URL: string
    APP_VERSION: string
    DEFAULT_EMAIL_SUBJECT: string
    DEFAULT_EMAIL_HTML_BODY: string
    DEFAULT_EMAIL_PLAINTEXT_BODY: string
}

const globalSettings: GlobalSettings = {
    QUERY_POLL_INTERVAL_MS: 2000,
    DEFAULT_TENANT_ID: "90505c8a-23a9-4cdf-a26b-4e19f6a097d5",
    ONLINE_VOTING_CLIENT_ID: "admin-portal",
    KEYCLOAK_URL: "http://127.0.0.1:8090/",
    APP_VERSION: "10.0.0",
    DEFAULT_EMAIL_SUBJECT: "Participate in __election.title__",
    DEFAULT_EMAIL_HTML_BODY: "<p>Vote in __url__ with Code __code__</p>",
    DEFAULT_EMAIL_PLAINTEXT_BODY: "Vote in __url__ with Code __code__",
}

export default globalSettings
