// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// A local stand-in for Keycloak: just enough OpenID Connect for keycloak-js
// (the voting and admin portals' login). It signs in the fixture user without
// asking for a password and issues UNSIGNED development tokens (alg "none"),
// which keycloak-js reads but never verifies. Never use it for anything but
// this preview.

import crypto from "node:crypto";

const b64u = (value) => Buffer.from(typeof value === "string" ? value : JSON.stringify(value)).toString("base64url");
const unsigned = (claims) => `${b64u({ alg: "none", typ: "JWT" })}.${b64u(claims)}.`;

const parseJwt = (token) => {
  try {
    return JSON.parse(Buffer.from(token.split(".")[1], "base64url").toString("utf8"));
  } catch {
    return undefined;
  }
};

/** Decodes the claims of a bearer token the stand-in issued, if it is one. */
export const claimsOf = (header) => {
  const token = /^Bearer (.+)$/i.exec(header ?? "")?.[1];
  return token ? parseJwt(token) : undefined;
};

export class FakeKeycloak {
  /**
   * `users(realm, clientId)` gives the signed-in user's claims for a realm:
   * `{sub, username, email, firstName, lastName, attributes, extra}`.
   */
  constructor({ origin, users, log = () => {} }) {
    this.origin = origin;
    this.users = users;
    this.log = log;
    this.codes = new Map();
    this.sessions = new Map();
  }

  issuer(realm) {
    return `${this.origin}/realms/${realm}`;
  }

  tokens({ realm, clientId, nonce, sessionId, authTime }) {
    const user = this.users(realm, clientId);
    const now = Math.floor(Date.now() / 1000);
    const base = {
      iat: now,
      auth_time: authTime ?? now,
      jti: crypto.randomUUID(),
      iss: this.issuer(realm),
      sub: user.sub,
      typ: "Bearer",
      azp: clientId,
      sid: sessionId,
      session_state: sessionId,
      acr: "1",
      scope: "openid profile email",
      email_verified: true,
      preferred_username: user.username,
      email: user.email,
      given_name: user.firstName,
      family_name: user.lastName,
      name: `${user.firstName} ${user.lastName}`.trim(),
      ...user.extra,
    };
    const lifespan = 900;
    return {
      access_token: unsigned({ ...base, exp: now + lifespan, aud: "account" }),
      refresh_token: unsigned({ ...base, exp: now + 1800, typ: "Refresh", aud: this.issuer(realm) }),
      id_token: unsigned({ ...base, exp: now + lifespan, typ: "ID", aud: clientId, ...(nonce ? { nonce } : {}) }),
      token_type: "Bearer",
      expires_in: lifespan,
      refresh_expires_in: 1800,
      session_state: sessionId,
      scope: "openid profile email",
      "not-before-policy": 0,
    };
  }

  cors(req, res) {
    const origin = req.headers.origin;
    if (origin) {
      res.setHeader("Access-Control-Allow-Origin", origin);
      res.setHeader("Vary", "Origin");
      res.setHeader("Access-Control-Allow-Headers", "authorization,content-type,x-hasura-admin-secret,x-hasura-role");
      res.setHeader("Access-Control-Allow-Methods", "GET,POST,OPTIONS");
      res.setHeader("Access-Control-Allow-Credentials", "true");
    }
  }

  /** Handles a request under `/realms/`; false if it isn't one. */
  async handle(req, res, url, readBody) {
    const m = /^\/realms\/([^/]+)\/(.*)$/.exec(url.pathname);
    if (!m) return false;
    const [, realm, rest] = m;
    this.cors(req, res);
    if (req.method === "OPTIONS") {
      res.writeHead(204).end();
      return true;
    }
    const json = (status, body) => {
      res.writeHead(status, { "Content-Type": "application/json" }).end(JSON.stringify(body));
      return true;
    };
    const oidc = `${this.issuer(realm)}/protocol/openid-connect`;
    if (rest === ".well-known/openid-configuration") {
      return json(200, {
        issuer: this.issuer(realm),
        authorization_endpoint: `${oidc}/auth`,
        token_endpoint: `${oidc}/token`,
        userinfo_endpoint: `${oidc}/userinfo`,
        end_session_endpoint: `${oidc}/logout`,
        jwks_uri: `${oidc}/certs`,
        check_session_iframe: `${oidc}/login-status-iframe.html`,
        grant_types_supported: ["authorization_code", "refresh_token"],
        response_types_supported: ["code"],
        response_modes_supported: ["query", "fragment"],
        subject_types_supported: ["public"],
        id_token_signing_alg_values_supported: ["none"],
        code_challenge_methods_supported: ["S256"],
      });
    }
    if (rest === "protocol/openid-connect/certs") return json(200, { keys: [] });
    if (rest === "protocol/openid-connect/auth" || rest === "protocol/openid-connect/registrations") {
      const q = url.searchParams;
      const redirect = q.get("redirect_uri");
      if (!redirect) return json(400, { error: "invalid_request", error_description: "redirect_uri is required" });
      const code = crypto.randomUUID();
      const sessionId = crypto.randomUUID();
      this.codes.set(code, { realm, clientId: q.get("client_id"), nonce: q.get("nonce"), sessionId, redirect, authTime: Math.floor(Date.now() / 1000) });
      const back = new URL(redirect);
      const params = new URLSearchParams({ state: q.get("state") ?? "", session_state: sessionId, iss: this.issuer(realm), code });
      if (q.get("response_mode") === "query") back.search = new URLSearchParams([...back.searchParams, ...params]).toString();
      else back.hash = params.toString();
      res.writeHead(302, { Location: back.toString(), "Cache-Control": "no-store" }).end();
      return true;
    }
    if (rest === "protocol/openid-connect/token") {
      const form = new URLSearchParams(await readBody(req));
      const grant = form.get("grant_type");
      if (grant === "authorization_code") {
        const issued = this.codes.get(form.get("code"));
        this.codes.delete(form.get("code"));
        if (!issued || issued.realm !== realm) return json(400, { error: "invalid_grant", error_description: "Code not valid" });
        this.sessions.set(issued.sessionId, issued);
        return json(200, this.tokens({ ...issued }));
      }
      if (grant === "refresh_token") {
        const claims = parseJwt(form.get("refresh_token") ?? "");
        if (!claims) return json(400, { error: "invalid_grant", error_description: "Invalid refresh token" });
        const kept = this.sessions.get(claims.sid) ?? { realm, clientId: claims.azp, sessionId: claims.sid, authTime: claims.auth_time };
        return json(200, this.tokens({ ...kept, realm, clientId: claims.azp, sessionId: claims.sid, nonce: undefined }));
      }
      return json(400, { error: "unsupported_grant_type" });
    }
    if (rest === "protocol/openid-connect/logout") {
      const back = url.searchParams.get("post_logout_redirect_uri") ?? url.searchParams.get("redirect_uri");
      if (back) res.writeHead(302, { Location: back }).end();
      else res.writeHead(200, { "Content-Type": "text/plain" }).end("Logged out");
      return true;
    }
    if (rest === "protocol/openid-connect/userinfo" || rest === "account") {
      const claims = claimsOf(req.headers.authorization);
      if (!claims) return json(401, { error: "invalid_token" });
      const user = this.users(realm, claims.azp);
      return json(200, {
        id: user.sub,
        sub: user.sub,
        username: user.username,
        email: user.email,
        firstName: user.firstName,
        lastName: user.lastName,
        emailVerified: true,
        attributes: user.attributes,
      });
    }
    if (rest === "protocol/openid-connect/login-status-iframe.html" || rest.startsWith("protocol/openid-connect/3p-cookies/")) {
      res.writeHead(200, { "Content-Type": "text/html" }).end("<!doctype html><title>login status</title>");
      return true;
    }
    return json(404, { error: "not_found", path: rest });
  }
}
