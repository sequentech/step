// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {ApolloClient, ApolloLink, InMemoryCache} from "@apollo/client"
import {ETranslationScope, initializeLanguages} from "@sequentech/ui-core"
import {ScenarioChannel} from "@sequentech/ui-test-kit/fixtures/scenarios"
import type {AuthContextValues} from "../providers/AuthContextProvider"
import type {GlobalSettings} from "../providers/SettingsContextProvider"
import {votingPortalTranslations} from "../translations"

export const PREVIEW_APP_VERSION = "preview"

/**
 * The settings of the portal's own publication preview: authentication is disabled, so
 * the screens skip every query. No service URL is set because none may be contacted.
 */
export const PREVIEW_SETTINGS: GlobalSettings = {
    DISABLE_AUTH: true,
    QUERY_POLL_INTERVAL_MS: 2000,
    DEFAULT_TENANT_ID: "",
    DEFAULT_EVENT_ID: "",
    ONLINE_VOTING_CLIENT_ID: "voting-portal",
    BALLOT_VERIFIER_URL: "",
    RESULTS_PORTAL_URL: "",
    KEYCLOAK_URL: "",
    HASURA_URL: "",
    APP_VERSION: PREVIEW_APP_VERSION,
    APP_HASH: PREVIEW_APP_VERSION,
    PUBLIC_BUCKET_URL: "",
    KEYCLOAK_ACCESS_TOKEN_LIFESPAN_SECS: 900,
    POLLING_DURATION_TIMEOUT: 12000,
}

/** An unauthenticated voter, as with disabled authentication, reaching the portal by `channel`. */
export const previewAuth = (
    channel: ScenarioChannel,
    logout: (redirectUrl?: string) => void
): AuthContextValues => ({
    isAuthContextInitialized: false,
    isAuthenticated: false,
    userId: "",
    username: "",
    email: "",
    firstName: "",
    keycloakAccessToken: undefined,
    logout,
    getExpiry: () => undefined,
    setTenantEvent: () => undefined,
    hasRole: () => false,
    isKiosk: () => channel === ScenarioChannel.KIOSK,
    openProfileLink: async () => undefined,
    isGoldUser: () => false,
    reauthWithGold: async () => undefined,
})

export class PreviewRequestError extends Error {
    constructor(operationName: string) {
        super(`The offline preview has no GraphQL service; ${operationName} was requested`)
        this.name = "PreviewRequestError"
    }
}

/** Any GraphQL operation fails at once instead of reaching a network. */
export const createPreviewApolloClient = () =>
    new ApolloClient({
        cache: new InMemoryCache(),
        link: new ApolloLink((operation) => {
            throw new PreviewRequestError(operation.operationName ?? "an anonymous operation")
        }),
    })

/** All voting portal languages in its translation scope, starting with an explicit language. */
export const initializePreviewLanguages = (language: string) =>
    initializeLanguages(votingPortalTranslations, language, ETranslationScope.VOTING_PORTAL)
