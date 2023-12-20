type GlobalSettings = {
    QUERY_POLL_INTERVAL_MS: number
    DEFAULT_TENANT_ID: string
    ONLINE_VOTING_CLIENT_ID: string
    KEYCLOAK_URL: string
    APP_VERSION: string
    DEFAULT_EMAIL_SUBJECT: {[langCode: string]: string}
    DEFAULT_EMAIL_HTML_BODY: {[langCode: string]: string}
    DEFAULT_EMAIL_PLAINTEXT_BODY: {[langCode: string]: string}
    DEFAULT_SMS_MESSAGE: {[langCode: string]: string}
}

const globalSettings: GlobalSettings = {
    QUERY_POLL_INTERVAL_MS: 3000,
    DEFAULT_TENANT_ID: "90505c8a-23a9-4cdf-a26b-4e19f6a097d5",
    ONLINE_VOTING_CLIENT_ID: "admin-portal",
    KEYCLOAK_URL: "http://keycloak.staging.sequent.vote/",
    APP_VERSION: "10.0.0",
    DEFAULT_EMAIL_SUBJECT: {en: "Participate in {{election_event.name}}"},
    DEFAULT_EMAIL_HTML_BODY: {
        en: "<p>Hello {{user.first_name}},<br><br>Enter in {{vote_url}} to vote</p>",
    },
    DEFAULT_EMAIL_PLAINTEXT_BODY: {
        en: "Hello {{user.first_name}},\n\nEnter in {{vote_url}} to vote",
    },
    DEFAULT_SMS_MESSAGE: {
        en: "Enter in {{vote_url}} to vote",
    },
}

export default globalSettings
