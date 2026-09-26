// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {gql} from "@apollo/client"
import {ETranslationScope, initializeLanguages} from "@sequentech/ui-core"
import {ScenarioChannel} from "@sequentech/ui-test-kit/fixtures/scenarios"
import {
    createPreviewApolloClient,
    initializePreviewLanguages,
    PREVIEW_SETTINGS,
    previewAuth,
    PreviewRequestError,
} from "./context"

jest.mock("@sequentech/ui-core", () => ({
    ...jest.requireActual("@sequentech/ui-core"),
    initializeLanguages: jest.fn(),
}))

test("settings disable authentication and name no service", () => {
    expect(PREVIEW_SETTINGS.DISABLE_AUTH).toBe(true)
    for (const key of [
        "HASURA_URL",
        "KEYCLOAK_URL",
        "PUBLIC_BUCKET_URL",
        "BALLOT_VERIFIER_URL",
        "RESULTS_PORTAL_URL",
    ] as const)
        expect(PREVIEW_SETTINGS[key]).toBe("")
    expect(PREVIEW_SETTINGS.KIOSK_KEYCLOAK_URL).toBeUndefined()
    expect(PREVIEW_SETTINGS.KIOSK_VOTING_PORTAL_URL).toBeUndefined()
})

test("the voter is a kiosk voter only on the kiosk channel", () => {
    const logout = jest.fn()
    const online = previewAuth(ScenarioChannel.ONLINE, logout)
    const kiosk = previewAuth(ScenarioChannel.KIOSK, logout)
    expect(online.isKiosk()).toBe(false)
    expect(kiosk.isKiosk()).toBe(true)
    expect(online).toMatchObject({
        isAuthContextInitialized: false,
        isAuthenticated: false,
        keycloakAccessToken: undefined,
    })
    expect(online.isGoldUser()).toBe(false)
    expect(online.hasRole("voter")).toBe(false)
    expect(online.getExpiry()).toBeUndefined()
    kiosk.logout("https://example.org/finish")
    expect(logout).toHaveBeenCalledWith("https://example.org/finish")
})

test("GraphQL operations fail without a request", async () => {
    const fetchSpy = jest.spyOn(globalThis, "fetch")
    const client = createPreviewApolloClient()
    await expect(
        client.query({
            query: gql`
                query GetElections {
                    elections
                }
            `,
        })
    ).rejects.toThrow(new PreviewRequestError("GetElections"))
    await expect(
        client.query({
            query: gql`
                {
                    anonymous
                }
            `,
        })
    ).rejects.toThrow(
        "The offline preview has no GraphQL service; an anonymous operation was requested"
    )
    expect(fetchSpy).not.toHaveBeenCalled()
    fetchSpy.mockRestore()
})

test("languages are the voting portal's, in its scope, with the given language", () => {
    initializePreviewLanguages("es")
    const [[translations, language, scope]] = jest.mocked(initializeLanguages).mock.calls
    expect(Object.keys(translations).sort()).toEqual(
        ["cat", "en", "es", "eu", "fr", "gl", "nl", "tl"].sort()
    )
    expect(language).toBe("es")
    expect(scope).toBe(ETranslationScope.VOTING_PORTAL)
})
