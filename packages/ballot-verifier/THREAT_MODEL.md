<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# ballot-verifier threat model

ballot-verifier is the cast-or-cancel verifier: a React single-page app that runs in the voter's browser and lets a voter audit a ballot they spoiled in voting-portal (Benaloh challenge). The voter logs in to the election event's Keycloak realm, loads the auditable ballot file downloaded from voting-portal, and types the ballot ID the portal showed before the cast-or-audit choice. The app checks with the sequent-core WASM (through `@sequentech/ui-core`) that the ballot's ciphertext encrypts its plaintext, compares the ballot hash with the typed ID, and shows the decoded selections. The operator serves the static build with nginx (`../Dockerfile.prod`, `../default.conf`). The verifier is how a voter detects a voting client that encrypts something other than the voter's choice, so its verdict is part of the cast-as-intended guarantee. See the [system threat model](../../THREAT_MODEL.md).

## Assets

- **Audited ballot file**: plaintext choices, encryption randomness, ciphertexts and an optional ephemeral voter signature. Confidentiality: it shows how the voter was going to vote. Integrity: it is the evidence under test.
- **Verification verdict**: the "ballot ID matches" and "ciphertext reproduces" results and the decoded selections on screen. Integrity: a wrong verdict hides a dishonest voting client.
- **Typed ballot ID**: the ID the voter recorded from voting-portal. Integrity.
- **Voter access token**: the Keycloak token for the event realm, sent to Hasura. Confidentiality.
- **Published election content**: ballot styles from Hasura, `election_event_config.json` from the public bucket, CSS and translation overrides. Integrity.
- **Verifier code**: the JS bundle, the sequent-core WASM and `global-settings.json`. Integrity: the voter has to trust the code that runs the check.

## Entry points and trust boundaries

| Entry point | Who can reach it | Trust | Checks in place |
|---|---|---|---|
| Routes `/`, `/tenant/:tenantId/event/:eventId/login`, `/start`, `/confirmation` (`src/App.tsx`, `src/screens/LoginScreen.tsx`) and `?lang=` (`src/providers/AuthContextProvider.tsx` `getLanguageFromURL`) | Anyone with a link | untrusted | `/` redirects to `DEFAULT_TENANT_ID`/`DEFAULT_EVENT_ID`; route params select the realm `tenant-<t>-event-<e>`; Keycloak `onLoad: "login-required"`; `lang` passed through `toBCP47` as the login locale |
| Ballot file drop on `/start` (`src/screens/HomeScreen.tsx` `handleFiles`) | Logged-in voter | untrusted content, authenticated voter | `JSON.parse`; typed serde deserialization in WASM; fail-closed `checkAuditableBallotCiphertext` (`src/services/ballotCiphertextVerification.ts`); when the file carries a voter signature, a failed `verifyBallotSignature` / `verifyMultiBallotSignature` rejects it |
| Ballot ID text field (`src/screens/HomeScreen.tsx`) | Logged-in voter | authenticated voter | Exact string comparison with the computed hash (`src/screens/ConfirmationScreen.tsx` `isMatchingBallotIds`) |
| "Use sample" button (`src/screens/HomeScreen.tsx` `onUseSampleClick`) | Logged-in voter | authenticated voter | Generates a demo ballot locally with `generateSampleAuditableBallot` |
| `GetBallotStyles` query to Hasura (`src/queries/GetBallotStyles.ts`, `src/services/BallotStyles.ts` `updateBallotStyleAndSelection`) | Hasura with the voter JWT | internal service, admin-authored content | Bearer token; Hasura `user` role filters by tenant, event, area and authorized elections (`../../hasura/metadata/databases/backend-db/tables/sequent_backend_ballot_style.yaml`); only styles of publications with `published_at` set are kept |
| `election_event_config.json` from `PUBLIC_BUCKET_URL` (`src/providers/ApolloContextProvider.tsx` `setupLogin`) | Public bucket | admin-published content | Read for the language policy only; errors fall back to no default locale |
| `/global-settings.json` (`src/providers/SettingsContextProvider.tsx` `loadSettings`): service URLs and login settings | Same origin | operator | None in the app; `../nginx-entrypoint.sh` overwrites its keys from the container environment |
| Header: language switch, profile link, logout (`src/App.tsx` `HeaderWithContext`) | Logged-in voter | authenticated voter | Language stored in a cookie through `setCookie`; profile opens Keycloak `accountManagement`; logo URL comes from the published ballot style |

## Threats

| ID | STRIDE | Threat | Controls in the code | Status |
|---|---|---|---|---|
| ballot-verifier-T1 | Tampering | A ballot file whose ciphertext is not the encryption of its plaintext and randomness is reported as verified | `checkAuditableBallotCiphertext` (`src/services/ballotCiphertextVerification.ts`) lets only `VERIFIED` through and keeps `MISMATCH` apart from `NOT_VERIFIABLE`; sequent-core compares both ciphertext components; any failure clears the loaded ballot; tests in `src/services/ballotCiphertextVerification.test.ts` | Mitigated |
| ballot-verifier-T2 | Tampering | The verdict is computed against election parameters that are not the authentic published ones | — | Not verified |
| ballot-verifier-T3 | Tampering | The audited ballot is not the one whose ID the portal showed before the cast-or-audit choice | Ballot hash computed in WASM; `src/screens/ConfirmationScreen.tsx` `isMatchingBallotIds` exact match, decoded selections hidden on mismatch | Mitigated |
| ballot-verifier-T4 | Tampering | Presentation content changes how the verdict or the selections read | HTML through ui-core `stringToHtml` (sanitize-html allowlist); class names through `toValidClassName` (`src/screens/hooks/useElectionClassName.ts`) | Partial |
| ballot-verifier-T5 | Repudiation | A voter who detects a dishonest ballot cannot prove to a third party that the file came from the platform, and the platform can deny it | `verifyBallotSignature` / `verifyMultiBallotSignature` check an ephemeral voter signature, not a platform signature; `window.print` in `src/screens/ConfirmationScreen.tsx` is the only record | Accepted (by design: the Benaloh audit gives the voter individual assurance; disputes rely on the electoral log and on many voters auditing) |
| ballot-verifier-T6 | Information disclosure | The audited ballot file (plaintext and randomness) is sent out of the voter's browser | The file is read locally; decoding, re-encryption and hashing run in WASM; no request carries the file | Mitigated |
| ballot-verifier-T7 | Information disclosure | Side channels reveal the decoded selections | TLS (deployment) | Partial |
| ballot-verifier-T8 | Elevation of privilege | A script on the verifier origin takes the voter's access token and misuses it | keycloak-js code flow; `logout` ends the Keycloak session; token lifetime set in Keycloak (deployment) | Partial |
| ballot-verifier-T9 | Denial of service | A crafted ballot file aborts the WASM decoder: sequent-core `decode_array_to_vec` panicked on an out-of-range length byte (`../sequent-core/src/ballot_codec/vec.rs`), reached from `decodeAuditableBallot`; the impact is limited to the voter's tab | `src/screens/HomeScreen.tsx` `handleFiles` catches exceptions | Open on release/10.0 (fix in sequentech/step#3522) |
| ballot-verifier-T10 | Denial of service | The operator's services are down or withhold access, so the voter cannot audit | The cryptographic check itself runs locally in WASM | Partial |
| ballot-verifier-T11 | Tampering | The served bundle or WASM differs from the reviewed source | Dependencies pinned by `yarn.lock` and installed with `--frozen-lockfile` (`../Dockerfile.prod`) | Partial |
| ballot-verifier-T12 | Spoofing | A look-alike or framed verifier shows a fake verdict | Security headers at the ingress (deployment) | Not verified |
| ballot-verifier-T13 | Spoofing | A crafted verifier link sends the voter to another event's realm | Login to the realm the route names; the voter needs an account there; `src/App.tsx` clears the loaded ballot when the route's event changes | Partial |
| ballot-verifier-T14 | Spoofing | Reverse tabnabbing: ui-essentials `Header` (used in `src/App.tsx`) and `Candidate` (rendered through `PlaintextVoteContest` in `src/screens/ConfirmationScreen.tsx`) open `target=_blank` links without `rel=noopener` | None in this package | Open on release/10.0 (fix in sequentech/step#3522) |

## Assumptions

- voting-portal shows the ballot ID before the cast-or-audit choice and keeps cast and audited ballots separate; the voter records the ID outside the voting device.
- Keycloak: each event realm has the client the verifier is configured to log in with, and issues short-lived access tokens.
- Hasura keeps the `user` role limited to the ballot styles and publications of the voter's tenant, event, area and authorized elections.
- Election admins are trusted for the presentation content they publish; the admin-portal and Hasura control who can publish it.
- The operator serves the reviewed build over TLS, keeps `global-settings.json` correct, serves the verifier from an origin that runs no other untrusted content, and sets CSP, HSTS and `frame-ancestors` at the ingress.
- The sequent-core WASM is built from reviewed source; strand's Ristretto ElGamal is correct.
- Voters who do not trust the operator need an independently built and hosted copy of the verifier; the electoral log and immudb record cast ballots for disputes.

## Review focus

1. Authenticity of the election parameters the verifier checks against.
2. Handling of untrusted ballot files across JS and WASM: parsing, decoding and error paths.
3. Presentation content that can change what the confirmation screen says.
4. Authentication and token handling in the browser.
5. Privacy of the decoded selections in the browser.
6. Supply chain of the bundle and the WASM.
7. Deployment: security headers and origin isolation.
