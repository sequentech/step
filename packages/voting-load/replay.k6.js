// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import http from "k6/http";
import crypto from "k6/crypto";
import encoding from "k6/encoding";
import { sleep } from "k6";

function origin(url) {
  return /^https?:\/\/[^/]+/.exec(url)?.[0];
}
function fields(query) {
  return Object.fromEntries(
    query
      .split("&")
      .filter(Boolean)
      .map((part) => {
        const split = part.indexOf("=");
        return [
          decodeURIComponent(part.slice(0, split)),
          decodeURIComponent(part.slice(split + 1)),
        ];
      }),
  );
}
function query(parameters) {
  return Object.entries(parameters)
    .map(([k, v]) => `${encodeURIComponent(k)}=${encodeURIComponent(v)}`)
    .join("&");
}

/** Execute the authenticated protocol with a fresh cookie jar and PKCE verifier. */
export function replayJourney(profile, ballot, index, config, cast) {
  const started = Date.now();
  const jar = new http.CookieJar();
  let form, authorization, code, file, authParameters;
  const publications = {};
  const timings = {};
  const state = encoding.b64encode(crypto.randomBytes(24), "rawurl");
  const verifier = encoding.b64encode(crypto.randomBytes(32), "rawurl");
  const nonce = encoding.b64encode(crypto.randomBytes(24), "rawurl");
  const checkUrl = (url) => {
    if (!config.allowed_origins.includes(origin(url)))
      throw new Error("Unapproved profile origin");
    return url;
  };
  const send = (method, url, body, kind, headers = {}, binary = false) => {
    checkUrl(url);
    const at = Date.now();
    const response = http.request(method, url, body, {
      jar,
      headers,
      redirects: 0,
      timeout: config.request_timeout || "30s",
      responseType: binary ? "binary" : "text",
      tags: { name: kind },
    });
    timings[kind] = Date.now() - at;
    if (config.trace_http !== false)
      console.log(
        JSON.stringify({
          kind: "http",
          index,
          method,
          phase: kind,
          operation: kind === "GetVoterStatus" ? kind : null,
          url: url.split(/[?#]/)[0],
          status: response.status,
          started_at_ms: at,
          duration_ms: Date.now() - at,
          response_bytes: binary
            ? response.body?.byteLength || 0
            : encodeURIComponent(response.body || "").replace(
                /%[0-9A-F]{2}/g,
                "_",
              ).length,
        }),
      );
    if (
      !(
        response.status === 200 ||
        (kind === "login" && response.status === 302)
      )
    )
      throw new Error(`HTTP failure at ${kind}: ${response.status}`);
    return response;
  };
  for (const step of profile.steps) {
    const delay = started + step.offset_ms * config.pacing - Date.now();
    if (delay > 0) sleep(delay / 1000);
    switch (step.kind) {
      case "auth": {
        authParameters = {
          ...step.parameters,
          state,
          nonce,
          code_challenge: crypto.sha256(verifier, "base64rawurl"),
          code_challenge_method: "S256",
        };
        form = send(
          "GET",
          step.url + "?" + query(authParameters),
          null,
          "auth",
        );
        break;
      }
      case "login": {
        const parsed = form.html();
        const action = parsed.find("#kc-form-login").attr("action");
        if (!action) throw new Error("Unsupported login form");
        const url = action.startsWith("/") ? origin(form.url) + action : action;
        const values = {};
        parsed.find("#kc-form-login input[name]").each((_, element) => {
          const name = element.getAttribute("name");
          values[name] =
            ballot.credentials[name] ?? element.getAttribute("value") ?? "";
        });
        const result = send("POST", url, values, "login");
        const location = result.headers.Location;
        if (
          !location ||
          !location.startsWith(authParameters.redirect_uri.split("?")[0])
        )
          throw new Error("Unexpected login redirect");
        const parameters = fields(
          location.split("#")[1] || location.split("?")[1] || "",
        );
        if (parameters.state !== state || !parameters.code)
          throw new Error("OAuth callback mismatch");
        code = parameters.code;
        break;
      }
      case "token": {
        const result = send(
          "POST",
          step.url,
          {
            grant_type: "authorization_code",
            code,
            client_id: authParameters.client_id,
            redirect_uri: authParameters.redirect_uri,
            code_verifier: verifier,
          },
          "token",
        ).json();
        if (!result.access_token) throw new Error("Missing access token");
        if (!result.id_token) throw new Error("Missing ID token");
        const claims = JSON.parse(
          encoding.b64decode(result.id_token.split(".")[1], "rawurl", "s"),
        );
        if (claims.nonce !== nonce) throw new Error("OIDC nonce mismatch");
        authorization = "Bearer " + result.access_token;
        break;
      }
      case "status": {
        const payload = JSON.parse(JSON.stringify(step.payload));
        payload.variables.electionEventId = ballot.election_event_id;
        const result = send(
          "POST",
          step.url,
          JSON.stringify(payload),
          "GetVoterStatus",
          { Authorization: authorization, "Content-Type": "application/json" },
        ).json();
        const status = result.data?.get_ballot_files_urls;
        if (result.errors || status?.event_id !== ballot.election_event_id) {
          // Bootstrap output is captured privately by the coordinator.
          if (config.bootstrap)
            console.log("BOOTSTRAP_ERROR " + JSON.stringify(result));
          throw new Error("Status rejected");
        }
        file = status.files?.find(
          (item) => item.election_id === ballot.payload.variables.electionId,
        );
        if (
          !file ||
          (!config.bootstrap && file.id !== ballot.style_id) ||
          (!config.bootstrap && file.version !== ballot.publication_version)
        )
          throw new Error("Prepared publication changed");
        break;
      }
      case "publication": {
        if (!file?.urls[step.binding])
          throw new Error("Publication binding missing");
        const value = send(
          "GET",
          file.urls[step.binding],
          null,
          step.binding,
        ).json();
        const expected =
          step.binding === "event_url"
            ? ballot.election_event_id
            : step.binding === "election_url"
              ? ballot.payload.variables.electionId
              : file.id;
        if (value.id !== expected)
          throw new Error("Publication scope mismatch");
        if (config.bootstrap) publications[step.binding] = value;
        break;
      }
      case "cast":
        if (!authorization || !file)
          throw new Error("Missing authenticated election");
        if (!cast(authorization)) throw new Error("Cast rejected");
        break;
      case "account":
        send("GET", step.url, null, "account", {
          Authorization: authorization,
        });
        break;
      case "resource":
        send("GET", step.url, null, "resource", {}, true);
        break;
      default:
        throw new Error("Unknown profile step");
    }
  }
  return { file, publications, timings };
}
