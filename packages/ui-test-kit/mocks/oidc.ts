// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {createHash} from "node:crypto"
import {
    escapeHtml,
    html,
    json,
    MOCK_PATHS,
    redirect,
    type MockRequest,
    type MockResponse,
} from "./http"
import {ViolationLog} from "./violations"

export interface OidcUser {
    id: string
    username: string
    email?: string
    firstName?: string
    lastName?: string
    attributes?: Record<string, string[]>
}

export interface OidcClaimsContext {
    realm: string
    clientId: string
    user: OidcUser
    acr: string
}

export interface OidcRealm {
    name: string
    clients: string[]
    user: OidcUser
    /** Extra access token claims, such as `https://hasura.io/jwt/claims`. */
    claims?: (context: OidcClaimsContext) => Record<string, unknown>
}

export interface OidcMockOptions {
    origin: string
    basePath?: string
    realms: OidcRealm[]
    violations: ViolationLog
    /** Milliseconds since the epoch; share it with a fake browser clock. */
    now?: () => number
    accessTokenLifespanSecs?: number
    refreshTokenLifespanSecs?: number
    /** Assurance level of an ordinary login; Keycloak's first level is "1". */
    defaultAcr?: string
    /** Origins, besides the portal's own, allowed as redirect targets. */
    redirectOrigins?: string[]
}

export type AuthorizationKind = "login" | "registration"

export interface AuthorizationRecord {
    kind: AuthorizationKind
    realm: string
    url: string
    params: Record<string, string>
    /** The assurance level the client demanded through the `claims` parameter. */
    requestedAcr?: string
}

export type PkceCheck = "valid" | "invalid" | "missing"

export interface TokenRecord {
    realm: string
    grantType: string
    clientId: string
    status: number
    pkce?: PkceCheck
    refreshToken?: string
    issued?: TokenSet
}

export interface LogoutRecord {
    realm: string
    params: Record<string, string>
}

export interface AccountRecord {
    realm: string
    authorization?: string
    status: number
}

export interface TokenSet {
    access_token: string
    expires_in: number
    refresh_expires_in: number
    refresh_token: string
    token_type: "Bearer"
    id_token: string
    "not-before-policy": number
    "session_state": string
    "scope": string
}

export type JwtClaims = Record<string, unknown>

interface PendingCode {
    code: string
    realm: string
    clientId: string
    redirectUri: string
    codeChallenge: string
    nonce?: string
    acr: string
    expiresAt: number
    used: boolean
}

interface Session {
    id: string
    realm: string
    clientId: string
    acr: string
    authTime: number
}

interface IssuedRefresh {
    session: Session
    expiresAt: number
}

const TOKEN_SCOPE = "openid email profile"
const CODE_LIFESPAN_SECS = 60

const base64Url = (value: string): string => Buffer.from(value, "utf8").toString("base64url")

/** Unsigned (`alg: none`) JWTs: the portal decodes tokens but never verifies them. */
export const encodeJwt = (claims: JwtClaims): string =>
    `${base64Url(JSON.stringify({alg: "none", typ: "JWT"}))}.${base64Url(JSON.stringify(claims))}.`

export const decodeJwt = (token: string): JwtClaims => {
    const payload = token.split(".")[1]
    if (!payload) {
        throw new Error("Not a JWT")
    }
    return JSON.parse(Buffer.from(payload, "base64url").toString("utf8")) as JwtClaims
}

export const pkceChallenge = (verifier: string): string =>
    createHash("sha256").update(verifier, "ascii").digest("base64url")

const requestedAcr = (claimsParameter: string | undefined): string | undefined => {
    if (!claimsParameter) {
        return undefined
    }
    try {
        const claims = JSON.parse(claimsParameter) as {
            id_token?: {acr?: {essential?: boolean; values?: string[]; value?: string}}
        }
        const acr = claims.id_token?.acr
        return acr?.essential ? (acr.values?.[0] ?? acr.value) : undefined
    } catch {
        return undefined
    }
}

/**
 * A fake Keycloak for the authorization code flow with PKCE, as keycloak-js
 * drives it with `checkLoginIframe: false`. By default the identity provider
 * authenticates the voter without interaction; after a logout it shows a
 * sign-in form, as a real provider does once its session has ended.
 */
export class OidcMock {
    readonly basePath: string
    readonly authorizations: AuthorizationRecord[] = []
    readonly tokenRequests: TokenRecord[] = []
    readonly logouts: LogoutRecord[] = []
    readonly accountRequests: AccountRecord[] = []
    /** When set, every token request is answered with this response. */
    tokenFailure?: {status: number; body?: string}
    /** Whether the provider has a signed-in session, so `auth` needs no interaction. */
    signedIn = true

    private readonly origin: string
    private readonly realms: Map<string, OidcRealm>
    private readonly violations: ViolationLog
    private readonly now: () => number
    private readonly accessLifespan: number
    private readonly refreshLifespan: number
    private readonly defaultAcr: string
    private readonly redirectOrigins: Set<string>
    private readonly codes = new Map<string, PendingCode>()
    private readonly pendingLogins = new Map<string, AuthorizationRecord>()
    private readonly refreshTokens = new Map<string, IssuedRefresh>()
    private readonly accessTokens = new Map<string, JwtClaims>()
    private counter = 0

    constructor(options: OidcMockOptions) {
        this.origin = options.origin
        this.basePath = options.basePath ?? MOCK_PATHS.keycloak
        this.realms = new Map(options.realms.map((realm) => [realm.name, realm]))
        this.violations = options.violations
        this.now = options.now ?? Date.now
        this.accessLifespan = options.accessTokenLifespanSecs ?? 900
        this.refreshLifespan = options.refreshTokenLifespanSecs ?? 1800
        this.defaultAcr = options.defaultAcr ?? "1"
        this.redirectOrigins = new Set([options.origin, ...(options.redirectOrigins ?? [])])
    }

    /** The value for a portal's `KEYCLOAK_URL` setting. */
    serverUrl(): string {
        return `${this.origin}${this.basePath}/`
    }

    issuer(realm: string): string {
        return `${this.origin}${this.basePath}/realms/${realm}`
    }

    handles(url: URL): boolean {
        return url.origin === this.origin && url.pathname.startsWith(`${this.basePath}/`)
    }

    /** Claims of an access token this provider issued and that has not expired. */
    verifyAccessToken(token: string): JwtClaims | undefined {
        const claims = this.accessTokens.get(token)
        if (!claims || typeof claims.exp !== "number" || claims.exp <= this.nowSecs()) {
            return undefined
        }
        return claims
    }

    lastIssued(): TokenSet | undefined {
        return this.tokenRequests.filter((request) => request.issued).at(-1)?.issued
    }

    handle(request: MockRequest): MockResponse {
        const path = request.url.pathname.slice(this.basePath.length)
        const match = /^\/realms\/([^/]+)\/(.+)$/.exec(path)
        const realm = match ? this.realms.get(decodeURIComponent(match[1])) : undefined
        if (!match || !realm) {
            this.violations.add(`Keycloak request for an unknown realm: ${request.url}`)
            return html(404, "<h1>Realm not found</h1>")
        }
        const endpoint = match[2]
        switch (`${request.method} ${endpoint}`) {
            case "GET protocol/openid-connect/auth":
                return this.authorize(realm, request, "login")
            case "GET protocol/openid-connect/registrations":
                return this.authorize(realm, request, "registration")
            case "POST login-actions/authenticate":
                return this.completeLogin(realm, request)
            case "POST protocol/openid-connect/token":
                return this.token(realm, request)
            case "GET protocol/openid-connect/logout":
                return this.logout(realm, request)
            case "GET account":
                return this.account(realm, request)
            default:
                this.violations.add(`Unexpected Keycloak request: ${request.method} ${request.url}`)
                return html(404, "<h1>Not found</h1>")
        }
    }

    private nowSecs(): number {
        return Math.floor(this.now() / 1000)
    }

    private nextId(prefix: string): string {
        this.counter += 1
        return `${prefix}-${this.counter}`
    }

    private redirectAllowed(value: string | undefined): value is string {
        if (!value) {
            return false
        }
        try {
            return this.redirectOrigins.has(new URL(value).origin)
        } catch {
            return false
        }
    }

    private authorize(realm: OidcRealm, request: MockRequest, kind: AuthorizationKind) {
        const params = Object.fromEntries(request.url.searchParams)
        const record: AuthorizationRecord = {
            kind,
            realm: realm.name,
            url: request.url.toString(),
            params,
            requestedAcr: requestedAcr(params.claims),
        }
        this.authorizations.push(record)
        const problems: string[] = []
        if (!realm.clients.includes(params.client_id)) {
            problems.push(`unknown client_id "${params.client_id}"`)
        }
        if (!this.redirectAllowed(params.redirect_uri)) {
            problems.push(`redirect_uri "${params.redirect_uri}" is not allowed`)
        }
        if (params.response_type !== "code") {
            problems.push(`response_type "${params.response_type}" is not "code"`)
        }
        if (!params.scope?.split(" ").includes("openid")) {
            problems.push("scope lacks openid")
        }
        if (!params.state) {
            problems.push("state is missing")
        }
        if (params.code_challenge_method !== "S256" || !params.code_challenge) {
            problems.push("PKCE S256 challenge is missing")
        }
        if (problems.length > 0) {
            this.violations.add(`Invalid ${kind} request to ${realm.name}: ${problems.join("; ")}`)
            return html(400, `<h1>Invalid request</h1><p>${escapeHtml(problems.join("; "))}</p>`)
        }
        if (!this.signedIn) {
            const loginId = this.nextId("login")
            this.pendingLogins.set(loginId, record)
            return html(
                200,
                `<!doctype html><html lang="en"><head><title>Sign in</title></head><body><main>` +
                    `<h1>Sign in to ${escapeHtml(realm.name)}</h1>` +
                    `<form method="post" action="${this.basePath}/realms/${encodeURIComponent(realm.name)}` +
                    `/login-actions/authenticate?session_code=${loginId}">` +
                    `<button type="submit">Sign in</button></form></main></body></html>`
            )
        }
        return this.redirectWithCode(realm, record)
    }

    private completeLogin(realm: OidcRealm, request: MockRequest): MockResponse {
        const loginId = request.url.searchParams.get("session_code") ?? ""
        const record = this.pendingLogins.get(loginId)
        if (!record || record.realm !== realm.name) {
            this.violations.add(`Unknown login session ${loginId}`)
            return html(400, "<h1>Login session expired</h1>")
        }
        this.pendingLogins.delete(loginId)
        this.signedIn = true
        return this.redirectWithCode(realm, record)
    }

    private redirectWithCode(realm: OidcRealm, record: AuthorizationRecord): MockResponse {
        const code = this.nextId("code")
        const {params} = record
        this.codes.set(code, {
            code,
            realm: realm.name,
            clientId: params.client_id,
            redirectUri: params.redirect_uri,
            codeChallenge: params.code_challenge,
            nonce: params.nonce,
            acr: record.requestedAcr ?? this.defaultAcr,
            expiresAt: this.nowSecs() + CODE_LIFESPAN_SECS,
            used: false,
        })
        const response = new URLSearchParams({
            state: params.state,
            session_state: this.nextId("session-state"),
            iss: this.issuer(realm.name),
            code,
        })
        const target = new URL(params.redirect_uri)
        if (params.response_mode === "query") {
            response.forEach((value, name) => target.searchParams.set(name, value))
        } else {
            target.hash = response.toString()
        }
        return redirect(target.toString())
    }

    private token(realm: OidcRealm, request: MockRequest): MockResponse {
        const form = new URLSearchParams(request.body ?? "")
        const grantType = form.get("grant_type") ?? ""
        const clientId = form.get("client_id") ?? ""
        const record: TokenRecord = {realm: realm.name, grantType, clientId, status: 200}
        this.tokenRequests.push(record)
        if (this.tokenFailure) {
            record.status = this.tokenFailure.status
            return {status: this.tokenFailure.status, body: this.tokenFailure.body ?? ""}
        }
        const fail = (description: string): MockResponse => {
            record.status = 400
            this.violations.add(`Rejected ${grantType} token request: ${description}`)
            return json(400, {error: "invalid_grant", error_description: description})
        }
        if (grantType === "authorization_code") {
            const pending = this.codes.get(form.get("code") ?? "")
            const verifier = form.get("code_verifier")
            record.pkce = !verifier
                ? "missing"
                : pending && pkceChallenge(verifier) === pending.codeChallenge
                  ? "valid"
                  : "invalid"
            if (!pending || pending.realm !== realm.name) {
                return fail("unknown code")
            }
            if (pending.used || pending.expiresAt < this.nowSecs()) {
                return fail("code already used or expired")
            }
            pending.used = true
            if (pending.clientId !== clientId) {
                return fail("code was issued to another client")
            }
            if (pending.redirectUri !== form.get("redirect_uri")) {
                return fail("redirect_uri differs from the authorization request")
            }
            if (record.pkce !== "valid") {
                return fail(`PKCE verification ${record.pkce}`)
            }
            const session: Session = {
                id: this.nextId("session"),
                realm: realm.name,
                clientId,
                acr: pending.acr,
                authTime: this.nowSecs(),
            }
            record.issued = this.issue(realm, session, pending.nonce)
            return json(200, record.issued)
        }
        if (grantType === "refresh_token") {
            const presented = form.get("refresh_token") ?? ""
            record.refreshToken = presented
            const issued = this.refreshTokens.get(presented)
            if (!issued || issued.expiresAt <= this.nowSecs()) {
                return fail("refresh token is unknown or expired")
            }
            if (issued.session.clientId !== clientId) {
                return fail("refresh token was issued to another client")
            }
            record.issued = this.issue(realm, issued.session)
            return json(200, record.issued)
        }
        return fail(`unsupported grant_type "${grantType}"`)
    }

    private issue(realm: OidcRealm, session: Session, nonce?: string): TokenSet {
        const iat = this.nowSecs()
        const {user} = realm
        const issuer = this.issuer(realm.name)
        const profile = {
            email_verified: true,
            preferred_username: user.username,
            ...(user.email ? {email: user.email} : {}),
            ...(user.firstName ? {given_name: user.firstName} : {}),
            ...(user.lastName ? {family_name: user.lastName} : {}),
        }
        const access: JwtClaims = {
            "exp": iat + this.accessLifespan,
            iat,
            "auth_time": session.authTime,
            "jti": this.nextId("access"),
            "iss": issuer,
            "aud": "account",
            "sub": user.id,
            "typ": "Bearer",
            "azp": session.clientId,
            "sid": session.id,
            "acr": session.acr,
            "allowed-origins": [this.origin],
            "realm_access": {roles: [`default-roles-${realm.name}`]},
            "scope": TOKEN_SCOPE,
            ...profile,
            ...realm.claims?.({
                realm: realm.name,
                clientId: session.clientId,
                user,
                acr: session.acr,
            }),
        }
        const refreshToken = encodeJwt({
            exp: iat + this.refreshLifespan,
            iat,
            jti: this.nextId("refresh"),
            iss: issuer,
            aud: issuer,
            sub: user.id,
            typ: "Refresh",
            azp: session.clientId,
            sid: session.id,
            scope: TOKEN_SCOPE,
        })
        const accessToken = encodeJwt(access)
        this.accessTokens.set(accessToken, access)
        this.refreshTokens.set(refreshToken, {session, expiresAt: iat + this.refreshLifespan})
        return {
            "access_token": accessToken,
            "expires_in": this.accessLifespan,
            "refresh_expires_in": this.refreshLifespan,
            "refresh_token": refreshToken,
            "token_type": "Bearer",
            "id_token": encodeJwt({
                exp: iat + this.accessLifespan,
                iat,
                auth_time: session.authTime,
                jti: this.nextId("id"),
                iss: issuer,
                aud: session.clientId,
                sub: user.id,
                typ: "ID",
                azp: session.clientId,
                sid: session.id,
                acr: session.acr,
                ...(nonce ? {nonce} : {}),
                ...profile,
            }),
            "not-before-policy": 0,
            "session_state": session.id,
            "scope": TOKEN_SCOPE,
        }
    }

    private logout(realm: OidcRealm, request: MockRequest): MockResponse {
        const params = Object.fromEntries(request.url.searchParams)
        this.logouts.push({realm: realm.name, params})
        this.signedIn = false
        const target = params.post_logout_redirect_uri
        if (!target) {
            return html(200, "<h1>You are logged out</h1>")
        }
        if (!this.redirectAllowed(target)) {
            this.violations.add(`Logout to a disallowed post_logout_redirect_uri: ${target}`)
            return html(400, "<h1>Invalid redirect uri</h1>")
        }
        return redirect(target)
    }

    private account(realm: OidcRealm, request: MockRequest): MockResponse {
        const authorization = request.headers.authorization
        const record: AccountRecord = {realm: realm.name, authorization, status: 200}
        this.accountRequests.push(record)
        if (!authorization) {
            return html(200, `<h1>Account of ${escapeHtml(realm.user.username)}</h1>`)
        }
        const token = /^bearer\s+(.+)$/i.exec(authorization)?.[1] ?? ""
        const claims = this.verifyAccessToken(token)
        if (!claims || claims.iss !== this.issuer(realm.name)) {
            record.status = 401
            this.violations.add(`Account request with an invalid token for ${realm.name}`)
            return json(401, {error: "invalid_token"})
        }
        const {user} = realm
        return json(200, {
            id: user.id,
            username: user.username,
            email: user.email,
            firstName: user.firstName,
            lastName: user.lastName,
            emailVerified: true,
            attributes: user.attributes ?? {},
        })
    }
}
