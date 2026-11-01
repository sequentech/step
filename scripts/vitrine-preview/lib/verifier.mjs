// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Step's ballot verifier, served as one transparency hub's ballot verifier
// (hub-b's): the settings its front serves, the policy's `hub-ballot-verifier`
// line that names it, and the link that opens an audited ballot on it.

/** The hub whose ballot verifier the preview serves. */
export const VERIFIER_HUB = "hub-b";

/** The /global-settings.json the verifier's front serves. It has no sign-in (a
 * hub's verifier is public), sends GraphQL to its own origin (the front forwards
 * it to the stand-in, whose locator answers a request with no token with the
 * published audited ballots only), and reads the event's public config, with
 * the sealed policy, from the stand-in's bucket. */
export function verifierSettings({ origins, tenantId, eventId, release }) {
  return {
    DISABLE_AUTH: true,
    QUERY_POLL_INTERVAL_MS: 2000,
    DEFAULT_TENANT_ID: tenantId,
    DEFAULT_EVENT_ID: eventId,
    ONLINE_VOTING_CLIENT_ID: "ballot-verifier",
    KEYCLOAK_URL: `${origins.standin}/`,
    HASURA_URL: `${origins.verifier}/v1/graphql`,
    APP_VERSION: release.version,
    APP_HASH: release.version,
    PUBLIC_BUCKET_URL: `${origins.standin}/public/`,
    HUB_NAME: VERIFIER_HUB,
    VOTING_PORTAL_URL: origins.voting,
  };
}

/** The ballot verifier URL the policy names for a hub (`hub-ballot-verifier
 * <hub> <url>`): the first such line after the hub's own `hub` line, as Vitrine
 * reads it. `policy` is the sealed policy's text. */
export function policyBallotVerifier(policy, hub) {
  const text = typeof policy === "string" ? policy : "";
  let defined = false;
  for (const line of text.split("\n")) {
    const [key, name, url] = line.split(" ");
    if (name !== hub) continue;
    if (key === "hub") defined = true;
    else if (key === "hub-ballot-verifier" && defined && url) return url;
  }
  return undefined;
}

const withSlash = (url) => (url.endsWith("/") ? url : `${url}/`);

/** Whether the policy names the preview's verifier as the hub's, and what to
 * do if not. */
export function verifierInPolicy(policy, origin, hub = VERIFIER_HUB) {
  const named = policyBallotVerifier(policy, hub);
  if (named && withSlash(named) === withSlash(origin)) return { ok: true, url: named };
  const hint = `re-provision the Vitrine stack with VITRINE_HUB_B_BALLOT_VERIFIER=${withSlash(origin)}`;
  return { ok: false, url: named, hint: named ? `the policy names ${named} for ${hub}; ${hint}` : `the policy names no ballot verifier for ${hub}; ${hint}` };
}

/** The link that opens an audited ballot on a hub's ballot verifier:
 * `<base>tenant/<t>/event/<e>/election/<el>/audited#id=<Tracking ID>&policy=<policy ID>`. */
export function auditedLink(base, { tenantId, eventId, electionId, trackingId, policyId }) {
  const path = ["tenant", tenantId, "event", eventId, "election", electionId, "audited"].map(encodeURIComponent).join("/");
  return `${withSlash(base)}${path}#id=${trackingId}&policy=${policyId}`;
}
