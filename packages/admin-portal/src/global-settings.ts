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
    QUERY_POLL_INTERVAL_MS: 2000,
    DEFAULT_TENANT_ID: "90505c8a-23a9-4cdf-a26b-4e19f6a097d5",
    ONLINE_VOTING_CLIENT_ID: "admin-portal",
    KEYCLOAK_URL: "http://127.0.0.1:8090/",
    APP_VERSION: "10.0.0",
    DEFAULT_EMAIL_SUBJECT: {en: "Participate in __TITLE__"},
    DEFAULT_EMAIL_HTML_BODY: {en: "<p>Vote in __URL__ with Code __CODE__</p>"},
    DEFAULT_EMAIL_PLAINTEXT_BODY: {en: "Vote in __URL__ with Code __CODE__"},
    DEFAULT_SMS_MESSAGE: {
        en: "Your authentication code is __CODE__ and is valid for __expiration_mins__ minutes.",
    },
}

export default globalSettings
