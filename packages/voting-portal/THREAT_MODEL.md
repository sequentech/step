<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# voting-portal threat model

voting-portal is the React single-page app voters use to log in, read support materials, fill in a ballot, encrypt it, cast it, audit it and get a receipt. It runs in the voter's browser, or on a kiosk device, and nginx serves it as static files (`../Dockerfile.prod`, `../default.conf`). The sequent-core WebAssembly build encodes, encrypts, hashes and optionally signs the ballot in the browser, so this is the code that decides which ciphertext leaves the voter's device. Election data comes from Hasura GraphQL (role `user`), from presigned published JSON and from the public bucket. Login uses the Keycloak realm of the election event. Authorization belongs on the server; the portal's own checks are user experience only. See the [system threat model](../../THREAT_MODEL.md).

## Assets

- **Voter access and refresh tokens**: the event-realm JWTs that let the holder read data and cast as the voter. Confidentiality.
- **Plaintext selections and write-ins**: the voter's choices before encryption. Confidentiality (ballot secrecy).
- **Auditable ballot**: ciphertexts together with plaintext and encryption randomness per contest. Confidentiality; revealing it for a cast ballot proves how the voter voted.
- **Ephemeral voter signing key**: created and used inside the WASM call. Confidentiality and integrity of the ballot signature.
- **Cast ballot and ballot ID**: the hashable ballot sent to `insert_cast_vote` and its hash. Integrity (cast as intended, recorded as cast).
- **Ballot definition and election public key**: the published ballot style the voter encrypts to. Integrity; a substituted key breaks secrecy.
- **sequent-core WASM** (`rust/sequent-core-0.1.0.tgz`): the code that encrypts ballots in the browser. Integrity.
- **Voter personal data**: Keycloak profile (name, username, email) and `login_hint__*` values. Confidentiality.
- **Receipts, cast-vote log entries and preview documents**: what the voter is shown about their own and other ballots. Confidentiality of other voters' data, integrity of the receipt.
- **Runtime configuration** (`/global-settings.json`): service URLs, client IDs and `DISABLE_AUTH`. Integrity.

## Entry points and trust boundaries

| Entry point | Who can reach it | Trust | Checks in place |
|---|---|---|---|
| `/tenant/:tenantId/event/:eventId/login`, `/enroll` (`src/index.tsx`, `src/App.tsx` `setupTenantEvent`) | Anyone with the link | untrusted | Keycloak login or registration in realm `tenant-{tenantId}-event-{eventId}` (`src/providers/AuthContextProvider.tsx` `createKeycloakConfig`); who may enroll or vote is decided by Keycloak |
| `election-chooser`, `materials` (`src/index.tsx`); `election/:electionId/{start,vote,review,confirmation,audit}`, `election/:electionId/ballot-locator/:ballotId?` (`src/routes/electionRoutes.tsx` `electionRoutes`) | Logged-in voter | authenticated voter | `src/providers/ApolloContextProvider.tsx` `ApolloWrapper` (mounted in `src/App.tsx`) renders no route and creates no client without a token unless `DISABLE_AUTH` is set; `src/routes/PublishedBallot.tsx` renders election screens only when the loaded style matches the URL tenant, event and election; data authorized server-side |
| `/preview/:tenantId/:documentId/:areaId/:publicationId` (`src/routes/PreviewPublicationEvent.tsx` `PreviewPublicationEvent`) | Anyone with the link | untrusted | `src/providers/SettingsContextProvider.tsx` forces `DISABLE_AUTH` on preview paths; casts simulated locally (`src/routes/ReviewScreen.tsx` `useAddFakeCastVote`); `getBallotStyleConfigurationError` on each style |
| `/`, `/cert-auth-error` (`src/index.tsx` `ThrowCertAuthError`, `src/App.tsx`) | Anyone | untrusted | Error pages only (`VotingPortalError`) |
| Query parameters `login_hint__*`, `kiosk`, `lang` | Anyone crafting a link | untrusted | `src/utils/loginHints.ts` `parseLoginHints`, `routeAcceptsLoginHints`, `appendLoginHints`; `src/utils/kioskUrls.ts` `getKioskPortalRedirectUrl`, `getKioskAwareUrl` (configured URLs only) |
| OIDC callback, step-up and logout (`AuthContextProvider.tsx` `initializeKeycloak`, `reauthWithGold`, `logout`) | Keycloak redirect, the voter | internal service | keycloak-js authorization code flow, `checkLoginIframe: false`, token in memory, `onTokenExpired` logs out |
| sessionStorage `ballotData`, `ballotDataExpiration`, `isDemo`, `areaId`, `documentId`, `publicationId` | Earlier navigation in the tab, same-origin script | untrusted | Expiry, guarded `JSON.parse` and shape checks (`src/routes/ReviewScreen.tsx` `getBallotDataFromSessionStorage`); the server re-validates every cast |
| `/global-settings.json` (`SettingsContextProvider.tsx`) | Deployment | operator | None in the client; templated from the environment by `../nginx-entrypoint.sh` |
| Public bucket: `election_event_config.json` (`src/services/ElectionEventConfig.ts` `createElectionEventConfigLoader`), candidate images, support materials, receipts (`src/hooks/public-document-url.ts` `useGetPublicDocumentUrl`) | Written by election managers and windmill, read anonymously | authenticated admin | Fixed `PUBLIC_BUCKET_URL` prefix; rich text through `stringToHtml`; integrity relies on object-storage access control |
| Published ballot JSON from presigned URLs (`src/services/PublishedBallots.ts` `fetchPublicationJson`, `loadPublicationList`, `loadSelectedBallot`; `src/hooks/useVoterContext.ts`) | Object storage | internal service | Event, election and style IDs compared with the `get_ballot_files_urls` references; `credentials: "omit"`, `referrerPolicy: "no-referrer"` |
| GraphQL in `src/queries/` and the `user`-role actions in `../../hasura/metadata/actions.yaml` (`insert_cast_vote`, `get_ballot_files_urls`, `create_ballot_receipt`, `list_cast_vote_messages`, `acknowledge_support_materials`, `get_support_materials_acknowledgment`; `fetchResultsArtifact` and `resolveResultsPublication` are also open to `user` though the portal does not call them) | Anyone holding a voter token, with or without the portal | authenticated voter | Hasura `user` row filters; harvest handlers call `authorize_voter_election` / `authorize_voter_event` (`../sequent-core/src/services/authorization.rs`) |

## Threats

| ID | STRIDE | Threat | Controls in the code | Status |
|---|---|---|---|---|
| voting-portal-T1 | Spoofing | An unauthenticated visitor reads voter data or casts a ballot | `src/providers/ApolloContextProvider.tsx` `ApolloWrapper` waits for `isAuthContextInitialized` and a token, and its link sends the Bearer token only to `HASURA_URL`; Hasura `user` row filters use the JWT claims, and the role has no insert on `cast_vote`; casting goes through harvest `authorize_voter_election` (`../sequent-core/src/services/authorization.rs`) | Mitigated |
| voting-portal-T2 | Elevation of privilege | A voter token used directly against GraphQL or actions reaches data or operations outside the voter's own event, area and elections | Hasura `user` row filters use the JWT claims; `authorize_voter_event` binds the event to the claims for `get_ballot_files_urls` | Partial |
| voting-portal-T3 | Tampering | Election policies applied in the portal are skipped by calling the voter API directly | Server-side checks on every cast (`../windmill/src/services/insert_cast_vote.rs` `try_insert_cast_vote`); revote limit in a database trigger (`../../hasura/migrations/backend-db/1783088256806_add_cast_vote_status_indexes_and_revote_limit`); the portal's own gates are UX, not controls | Partial |
| voting-portal-T4 | Spoofing | The kiosk channel is used from an unmanaged device | Kiosk sessions use their own Keycloak client (`AuthContextProvider.tsx` `getClientId`, `src/utils/kioskUrls.ts`); device restriction is a deployment control | Not verified |
| voting-portal-T5 | Tampering | The ballot held across step-up re-authentication is cast under another identity, replayed or altered | `src/routes/ReviewScreen.tsx` `storeBallotDataAndReauth` stores only the hashable ballot (no plaintext or randomness) with a 5-minute expiry; `getBallotDataFromSessionStorage` checks expiry and shape; `clearSessionStorageBallotData` (`src/store/castVotes/sessionBallotData.ts`) after use or failure; the server re-authorizes and re-hashes the cast | Partial |
| voting-portal-T6 | Information disclosure | Plaintext selections, randomness or another voter's state leak from the browser, including on shared or kiosk devices | Selections live only in memory; encryption, hashing and ephemeral signing run in WASM (`src/hooks/useEncryptBallotForReview.ts` `encryptAndStoreBallot`); randomness from `crypto.getRandomValues`; only the public key and signature leave `sign_hashable_ballot_with_ephemeral_voter_signing_key`; `src/utils/voterSessionScope.ts` `voterSessionScope` plus `src/store/store.ts` `clearVoterSession` reset Redux and Apollo when subject, client, session, `acr` or Hasura claims change; no telemetry sink (`src/index.tsx` calls `reportWebVitals()` without a callback) | Partial |
| voting-portal-T7 | Information disclosure | Cast-or-audit separation fails, breaking ballot secrecy or giving a coercion receipt | Audit is voter-initiated and confirmed through `AuditBallotHelpDialog` (`src/routes/ReviewScreen.tsx`); ballots are encrypted in WASM with fresh randomness (`encryptAndStoreBallot`) | Partial |
| voting-portal-T8 | Tampering | A substituted ballot definition or election public key makes voters encrypt to the wrong key or see altered options | `src/services/PublishedBallots.ts` `loadPublicationList`, `loadSelectedBallot` compare event, election and style IDs with the authorized references; `src/hooks/useVoterContext.ts` rejects an event mismatch; `src/services/BallotStyles.ts` `getBallotStyleConfigurationError` | Partial |
| voting-portal-T9 | Tampering | The shipped WASM does not match the reviewed sequent-core and strand source | The tarball is a committed build artifact referenced from `package.json`; CI keeps every package's copy identical | Partial |
| voting-portal-T10 | Elevation of privilege | Script injection through election text, translations or interpolated values steals the token or alters the ballot | `../ui-core/src/services/stringToHtml.ts` `stringToHtml` (sanitize-html allowlist) at every HTML sink; `translateHtml` / `escapeTranslationValues` where values are interpolated (i18next runs with `escapeValue: false`, so this depends on call sites); no `dangerouslySetInnerHTML`, `innerHTML` or `eval` in `src/`; React 19 blocks `javascript:` URLs in JSX | Partial |
| voting-portal-T11 | Information disclosure | Content authored or uploaded by election managers leaks voter information to another origin, misleads voters or runs with the portal's origin | HTML sanitized as in T10; documents load from `PUBLIC_BUCKET_URL` (`src/hooks/public-document-url.ts` `useGetPublicDocumentUrl`, used by `src/components/SupportMaterial/SupportMaterial.tsx`); default logout target is same-origin (`src/utils/logoutRedirect.ts` `getLogoutRedirectUrl`); kiosk portal redirect goes only to the configured `KIOSK_VOTING_PORTAL_URL` (`src/utils/kioskUrls.ts` `getKioskPortalRedirectUrl`); a CSP is expected from the ingress | Partial |
| voting-portal-T12 | Tampering | Reverse tabnabbing from `target=_blank` links without `rel=noopener` in `../ui-essentials/src/components/Candidate/Candidate.tsx`, `Header/Header.tsx` and `SelectElection/SelectElection.tsx`, all rendered by the portal | None on this branch | Open on release/10.0 (fix in sequentech/step#3522) |
| voting-portal-T13 | Tampering | Clickjacking or missing browser hardening (CSP, frame-ancestors, nosniff, HSTS) | Security headers in `../default.conf` are commented out and expected from the ingress | Not verified |
| voting-portal-T14 | Information disclosure | Personal data in login-hint URLs leaks through browser history | `src/utils/loginHints.ts` `parseLoginHints` (name pattern, count and length limits, strict decoding), `removeLoginHintsFromSearch`, `appendLoginHints` re-validates; hints accepted only on `/login` and `/enroll` (`routeAcceptsLoginHints`); `src/App.tsx` replaces the history entry. Copies sent to Keycloak are accepted by design, since Keycloak needs them | Mitigated |
| voting-portal-T15 | Information disclosure | Voter-facing views show other voters' data | Ballot lookup (`src/routes/BallotLocator.tsx`, `GET_CAST_VOTE`) reads the voter's own cast votes through the Hasura `cast_vote` filter on `voter_id_string`; voter log views are off by default (election policy) | Partial |
| voting-portal-T16 | Spoofing | A preview link is mistaken for the real election or interferes with a real voting session | Preview casts are simulated (`useAddFakeCastVote`); demo watermark while the tab is in preview (`src/store/ballotStyles/ballotStylesSlice.ts` `showDemo`, `src/components/WaterMark/Watermark.tsx`) | Partial |
| voting-portal-T17 | Repudiation | The voter cannot show that the ballot was recorded as cast, or the receipt does not match the cast ballot | Ballot ID computed in WASM and shown on `src/routes/ConfirmationScreen.tsx`; the server rejects a cast whose hash differs from the ballot ID (`../windmill/src/services/insert_cast_vote.rs`); ballot locator (`src/routes/BallotLocator.tsx`); receipts generated only for a ballot ID the caller cast; inclusion on the bulletin board is outside this package | Partial |
| voting-portal-T18 | Denial of service | The voter cannot load the ballot or cast because publication downloads fail or presigned URLs expire | `src/hooks/useVoterContext.ts` renews URLs once on 401/403 and offers a retry; `mapPublicationFiles` caps parallel downloads at 4; `fetchPublicationJson` shares in-flight requests per Apollo client | Partial |
| voting-portal-T19 | Elevation of privilege | Arguments of voter actions are used by server-side services beyond what the voter may do | harvest voter actions authorize the election or event against the claims (`authorize_voter_election`, `authorize_voter_event`) | Not verified |
| voting-portal-T20 | Information disclosure | Receipts, previews and other generated documents in the public bucket are read by people other than their owner | Object-storage access control (deployment) | Not verified |
| voting-portal-T21 | Denial of service | Voter-callable actions that start expensive server work are called in bulk with a voter token | None in the portal; rate limiting expected from the ingress and Hasura | Not verified |

## Assumptions

- Hasura validates JWT signatures and issuer per realm, the `user` role sees only rows and columns scoped to the caller's tenant, event, area and elections, and the `unauthorized` role sees no voter data or unpublished content (hasura).
- harvest and windmill enforce every election policy the portal shows, and validate every voter action argument and bind it to the JWT claims.
- Keycloak event realms configure the voter clients securely and apply brute-force protection (keycloak-extensions).
- The ingress serves the portal over TLS with a CSP, frame-ancestors, nosniff and HSTS, rate-limits GraphQL, and serves `PUBLIC_BUCKET_URL` from an origin separate from the portal.
- Only windmill writes publication objects and the public bucket, and the bucket does not allow listing.
- Kiosk devices are managed hardware, the deployment restricts the kiosk channel to them, and kiosk sessions end with logout.
- The committed WASM tarball is rebuilt from reviewed source whenever sequent-core or strand change.
- Operators protect `global-settings.json`; `DISABLE_AUTH` stays `false` outside preview, and no secret is copied into it.

## Review focus

1. Server-side enforcement of every election policy the portal applies.
2. Hasura `user` permissions, used without the portal.
3. Handling of ballot secrets in the browser, including on shared and kiosk devices.
4. Server-side validation of voter action arguments.
5. Election-manager content in the voter page, and the CSP and headers at the ingress.
6. Preview mode and its isolation from real voting sessions.
7. Integrity of the published ballot data and of the shipped WASM.
8. Keycloak configuration of the event realms, including the kiosk channel.
