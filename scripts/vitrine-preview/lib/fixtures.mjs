// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The election the stand-in serves: an election event, one election with one
// contest, its ballot style (the text the portal hashes and encrypts from),
// the area, and the voter. IDs come from the live Vitrine stack's policy (its
// event ID and one of its election IDs), so that the ballots log accepts what
// the portal casts.

import { readFileSync } from "node:fs";

import { sha256, styleDigest, styleList, stylesHash } from "./vitrine.mjs";
import { createBallotBox } from "./receipts.mjs";

export const TENANT_ID = "90505c8a-23a9-4cdf-a26b-4e19f6a097d5";
export const AREA_ID = "5e1c2a3b-7f64-4b08-9d51-6c3a0e8f2b17";
export const VOTER_ID = "4b0f5f4c-6a1e-4f5e-8c2f-3d9a7c1e5b20";
export const ADMIN_ID = "a7d0c6f2-2b4e-4d0b-9a61-0f3e8d5c1b94";
export const VOTER_USERNAME = "voter@example.org";

/** The header logo of the event's organizer, Valmora, a made-up municipality, as a data URL so
 * that the style carries it and the portal needs no other request. */
export const LOGO_URL = "data:image/svg+xml;base64," + readFileSync(new URL("./valmora-logo.svg", import.meta.url)).toString("base64");

/** The election public key the portal encrypts to: a valid Ristretto point
 * (the one Step's own portal fixtures use). Nobody holds its secret half. */
export const ELECTION_PUBLIC_KEY = "ajR/I9RqyOwbpsVRucSNOgXVLCvLpfQxCgPoXGQ2RF4";

/** The keys ceremony the key came from: the portal's encryption names it, and refuses a non-demo key without one. */
export const KEYS_CEREMONY_ID = "5f1c3b7e-0a4d-4c8e-9b2a-6d7e8f901234";

/** The preview's ballot box signs with an Ed25519 key from a fixed seed, so that its published
 * half, which Step puts in each ballot style, is the same in the style the events log's
 * config statement commits to (`provision-events`) and in the one the stand-in serves.
 * The key is the preview's own: it signs nothing outside it. */
export const BALLOT_BOX_SEED = sha256(Buffer.from("vitrine-preview/ballot-box-key/v1"));
export const ballotBox = createBallotBox(BALLOT_BOX_SEED);

const CANDIDATES = [
  ["ana-ferrer", "Ana Ferrer", "Teacher and former deputy mayor."],
  ["jon-etxeberria", "Jon Etxeberria", "Civil engineer, city planner."],
  ["maria-santos", "Maria Santos", "Hospital director."],
  ["pieter-de-vries", "Pieter de Vries", "Cooperative founder."],
];

const i18n = (name, description) => ({
  en: { name, description },
  es: { name, description },
});

/** The release the config statement and the portal's own build name. */
export const PORTAL_RELEASE = (version) => ({ package: "sequent/voting-portal", version });

/** The ballot style's JSON text: its exact bytes are what the config
 * statement's `styles` commits to. */
export function ballotStyleText({ live, electionId, policyNote }) {
  const contestId = `${electionId}-contest-1`;
  const common = { tenant_id: TENANT_ID, election_event_id: live.eventId, election_id: electionId };
  const style = {
    id: `${electionId}-style`,
    ...common,
    area_id: AREA_ID,
    num_allowed_revotes: 0,
    description: "Municipal council election",
    public_key: { public_key: ELECTION_PUBLIC_KEY, is_demo: false, keys_ceremony_id: KEYS_CEREMONY_ID },
    area_presentation: { allow_early_voting: "no_early_voting" },
    contests: [
      {
        id: contestId,
        ...common,
        name: "Who should chair the municipal council?",
        description: "Choose one person. You can change your vote until voting closes.",
        max_votes: 1,
        min_votes: 0,
        winning_candidates_num: 1,
        voting_type: "first-past-the-post",
        counting_algorithm: "plurality-at-large",
        is_encrypted: true,
        candidates: CANDIDATES.map(([id, name, description], sort_order) => ({
          id: `${electionId}-${id}`,
          ...common,
          contest_id: contestId,
          name,
          description,
          candidate_type: "A",
          presentation: {
            is_explicit_invalid: false,
            is_disabled: false,
            is_category_list: false,
            is_write_in: false,
            sort_order,
          },
        })),
        presentation: {
          allow_writeins: false,
          invalid_vote_policy: "allowed",
          blank_vote_policy: "allowed",
          i18n: { en: { name: "Who should chair the municipal council?", description: "Choose one person. You can change your vote until voting closes." } },
        },
      },
    ],
    election_event_presentation: eventPresentation({ live, policyNote }),
    election_presentation: {
      i18n: i18n("Municipal council election", "Choose who chairs the municipal council."),
    },
    election_dates: {},
    // With receipts signed by the ballot box, Step publishes the box's key in the style.
    ballot_box_key: ballotBox.key,
  };
  return JSON.stringify(style, null, 2);
}

/** The event's `presentation` as Step stores it, with the Vitrine part
 * (`transparency`, `vitrine`) harvest adds once the Board sealed the policy. */
export function eventPresentation({ live, policyNote }) {
  return {
    i18n: {
      en: { name: "Municipal elections 2026", description: "Vote online in the municipal elections." },
      es: { name: "Elecciones municipales 2026", description: "Vote por Internet en las elecciones municipales." },
    },
    language_conf: { enabled_language_codes: ["en", "es"], default_language_code: "en" },
    logo_url: LOGO_URL,
    skip_election_list: false,
    show_user_profile: true,
    show_cast_vote_logs: "hide-logs-tab",
    // The ballot box receives each ballot at review and signs it; voters sign their ballots.
    voter_signing_policy: "with-signature",
    receipts: { policy: "signed-by-ballot-box", checks_period_policy: "unlimited" },
    transparency: "public",
    vitrine: {
      policy_id: live.policyId,
      policy: policyNote,
      logs: {
        ballots: { url: live.ballots.url, origin: live.ballots.origin },
        events: { url: live.events.url, origin: live.events.origin },
      },
    },
  };
}

/** The digests the config statement commits to, and the statement's `styles`. */
export function styleDigests(text) {
  const digests = styleList([styleDigest(text)]);
  return { digests, hash: stylesHash(digests), key: sha256(Buffer.from(ELECTION_PUBLIC_KEY, "base64")) };
}

const NOW = () => new Date().toISOString();

/** The rows the stand-in serves, in Hasura's tables. */
export function tables({ live, electionId, policyNote, text }) {
  const ts = NOW();
  const base = { tenant_id: TENANT_ID, created_at: ts, last_updated_at: ts, labels: null, annotations: null };
  const opened = new Date(live.opens * 1000).toISOString();
  const closes = new Date(live.closes * 1000).toISOString();
  const status = {
    is_published: true,
    voting_status: "OPEN",
    kiosk_voting_status: "NOT_STARTED",
    early_voting_status: "NOT_STARTED",
    telephone_voting_status: "NOT_STARTED",
    voting_period_dates: { first_started_at: opened, last_stopped_at: null, first_paused_at: null, last_paused_at: null },
    kiosk_voting_period_dates: {},
    early_voting_period_dates: {},
    telephone_voting_period_dates: {},
  };
  return {
    sequent_backend_election_event: [
      {
        ...base,
        id: live.eventId,
        name: "Municipal elections 2026",
        description: "Vote online in the municipal elections.",
        alias: "municipal-2026",
        presentation: eventPresentation({ live, policyNote }),
        status: { voting_status: "OPEN", is_published: true, kiosk_voting_status: "NOT_STARTED", early_voting_status: "NOT_STARTED", telephone_voting_status: "NOT_STARTED", voting_period_dates: {}, kiosk_voting_period_dates: {} },
        is_archived: false,
        is_audit: null,
        bulletin_board_reference: null,
        encryption_protocol: "RSA256",
        public_key: ELECTION_PUBLIC_KEY,
        voting_channels: { online: true, kiosk: false },
        dates: { start_date: opened, end_date: closes },
        statistics: null,
      },
    ],
    sequent_backend_election: [
      {
        ...base,
        id: electionId,
        election_event_id: live.eventId,
        name: "Municipal council election",
        alias: "council",
        description: "Choose who chairs the municipal council.",
        presentation: { i18n: i18n("Municipal council election", "Choose who chairs the municipal council."), dates: { start_date: opened, end_date: closes } },
        status,
        voting_channels: { online: true, kiosk: false, early_voting: false, telephone: false },
        is_consolidated_ballot_encoding: false,
        spoil_ballot_option: false,
        num_allowed_revotes: 0,
        dates: { start_date: opened, end_date: closes },
        eml: null,
        statistics: null,
        permission_label: null,
      },
    ],
    sequent_backend_ballot_style: [
      {
        ...base,
        id: `${electionId}-style`,
        election_id: electionId,
        election_event_id: live.eventId,
        area_id: AREA_ID,
        status: "published",
        ballot_eml: text,
        ballot_signature: null,
        deleted_at: null,
      },
    ],
    sequent_backend_area: [{ ...base, id: AREA_ID, election_event_id: live.eventId, name: "Municipality", description: "The whole municipality", type: null, parent_id: null }],
    sequent_backend_support_material: [],
    sequent_backend_cast_vote: [],
    sequent_backend_tenant: [{ id: TENANT_ID, slug: "step", settings: null, voting_channels: { online: true }, is_active: true, annotations: null, labels: null, created_at: ts, updated_at: ts }],
  };
}
