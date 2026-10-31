// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {jest} from "@jest/globals"
import type {KeycloakConfig, KeycloakInitOptions, KeycloakProfile} from "keycloak-js"

const signedInVoter = () => ({
    /** Keycloak.init resolves with this, or rejects when it is an Error. */
    authenticated: true as boolean | Error,
    token: "voter-access-token",
    /** Keycloak.loadUserProfile resolves with this, or rejects when it is an Error. */
    profile: {
        id: "voter-1",
        username: "synthetic-voter",
        email: "voter@example.org",
        firstName: "Sam",
        attributes: {"tenant-id": ["tenant-from-profile"]},
    } as (KeycloakProfile & {attributes: Record<string, string[]>}) | Error,
})

/** The session the next Keycloak client finds; tests adjust it before rendering. */
export const keycloakSession = signedInVoter()

/** Stand-in for the keycloak-js client, installed with jest.mock("keycloak-js", ...). */
export default class FakeKeycloak {
    static instances: FakeKeycloak[] = []

    token?: string
    readonly init = jest.fn(async (_options: KeycloakInitOptions) => {
        const {authenticated, token} = keycloakSession
        if (authenticated instanceof Error) throw authenticated
        if (authenticated) this.token = token
        return authenticated
    })
    /** The real client leaves the page for the login form; this one only resolves. */
    readonly login = jest.fn(async (_options?: KeycloakInitOptions) => undefined)
    /** Resolves true when it issued a new token; by default the token is still valid. */
    readonly updateToken = jest.fn(async (_minValidity: number) => false)
    readonly loadUserProfile = jest.fn(async () => {
        const {profile} = keycloakSession
        if (profile instanceof Error) throw profile
        return profile
    })
    readonly logout = jest.fn(async () => undefined)
    readonly hasRealmRole = jest.fn((role: string) => role === "voter")
    readonly accountManagement = jest.fn(async () => undefined)

    constructor(readonly config: KeycloakConfig) {
        FakeKeycloak.instances.push(this)
    }
}

export const resetKeycloak = () => {
    FakeKeycloak.instances = []
    Object.assign(keycloakSession, signedInVoter())
}
