<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# results-portal threat model

results-portal is the React single-page app that shows published tally results at `/:electionEventId` and `/:electionEventId/elections/:electionId`. It runs in the browsers of the public, voters and observers, and nginx serves it as static files (`../Dockerfile.prod`). It reads a discovery index, a manifest and a SQLite results database that windmill publishes to object storage. It opens the database in the browser with sql.js (WebAssembly) and renders it with ui-essentials components. For publications with `authenticated` access, the viewer signs in to the election event's Keycloak realm with the `results-portal` client. The portal then calls the Hasura actions `resolveResultsPublication` and `fetchResultsArtifact`, which windmill authorizes. The portal enforces no access rule itself. This is where the public reads the outcome of an election, so what it shows must match the tally, and results meant only for signed-in voters or for one area must not reach anyone else. See the [system threat model](../../THREAT_MODEL.md).

## Assets

- **Published results**: the SQLite database (`results_*` tables, names, areas) and the manifest's contest list. Integrity and availability.
- **Discovery index and manifest**: they decide what the portal shows. Integrity.
- **Restricted results**: the manifest and SQLite of `authenticated` publications, per-area artefacts and the presigned URLs to them. Confidentiality.
- **Viewer access token**: the event-realm JWT issued to the `results-portal` client. Confidentiality.
- **Viewer profile**: name, username and email from the token, shown in the header. Confidentiality.
- **Runtime configuration** (`/global-settings.json`): Keycloak, Hasura and bucket URLs and the client ID. Integrity.
- **Portal bundle and `sql-wasm.wasm`**: the code that parses and renders results. Integrity.

## Entry points and trust boundaries

| Entry point | Who can reach it | Trust | Checks in place |
|---|---|---|---|
| Routes `/:eeId` and `/:eeId/elections/:electionId` (`src/index.tsx`, `src/routes/ResultsRoute.tsx`) | Anyone with a link | untrusted | Signed-in readers resolved server-side by windmill |
| Fallback route `*` and `errorElement` (`src/index.tsx`) | Anyone | untrusted | Static state messages only |
| Query parameter `lang` and the language cookie (`src/services/i18n.ts` `getLanguageFromURL`, `src/App.tsx` `AppHeader`) | Anyone crafting a link | untrusted | Used only as an i18next language key; a language outside the manifest's list falls back to the default locale (`AppHeader`) |
| `/global-settings.json` (`src/providers/SettingsContextProvider.tsx` `SettingsWrapper`) | Deployment | operator | None; templated from the environment by `../nginx-entrypoint.sh` |
| Public discovery index (`src/services/publicationDiscovery.ts`) | Written by windmill, fetched by every viewer | internal service | `src/types/results.ts` `parseResultsPublicationIndex` (required fields, `access` and scope enums); 404 means not published |
| Manifest from the bucket or the resolver (`src/routes/ResultsRoute.tsx`) | Written by windmill | internal service | `src/types/results.ts` `parseResultsManifest` (required fields and enums) |
| SQLite artefact (`src/services/sqliteResults.ts` `loadSqliteDatabase`, `readResultsDataset`) | Public bucket or presigned URL | internal service | sql.js in WebAssembly; fixed `SELECT` per known table; a missing table yields no rows (`queryTable`) |
| Hasura actions `resolveResultsPublication`, `fetchResultsArtifact` (`src/queries/resultsPublication.ts`, `src/services/graphql.ts` `graphqlFetch`), handled by harvest and windmill | Holders of an event-realm or admin token | authenticated voter | `../../hasura/metadata/actions.yaml` roles `user`, `admin-user`; `../windmill/src/services/results_publication.rs` `resolve_results_publication_request`, `fetch_results_artifact_request`, `authorize_results_reader` |
| OIDC redirect, token refresh, logout and account console link (`src/hooks/useAuthenticatedResults.ts`) | Keycloak, signed-in viewer | internal service | keycloak-js standard flow; redirect URIs validated by the Keycloak client; logout returns to the current page without its fragment |
| Admin-authored content inside a publication: custom CSS, translation overrides, election, contest, candidate and area names (`src/services/customCss.ts`, `src/services/resultsOrdering.ts`, `src/services/resultLabels.ts`) | Election managers through admin-portal | authenticated admin | Rendered through React components; custom CSS as `<style>` text |

## Threats

| ID | STRIDE | Threat | Controls in the code | Status |
|---|---|---|---|---|
| results-portal-T1 | Spoofing | The portal renders an index, manifest or SQLite that did not come from the platform's publication, under the portal's own origin | Shape checks in `src/types/results.ts`; storage origin and bucket write control (deployment) | Partial |
| results-portal-T2 | Tampering | Published artefacts are altered in object storage or in transit, so the page differs from the tally | Shape and enum checks only (`src/types/results.ts`); integrity rests on bucket write control and TLS; the page shows the publication version (`src/components/ResultsPageContent.tsx`) | Not verified |
| results-portal-T3 | Tampering | Revoked or superseded results stay visible | Index, manifest and SQLite fetched with `cache: "no-store"`; windmill deletes revoked and superseded artefacts and rewrites the index; caches between bucket and browser are outside this package | Partial |
| results-portal-T4 | Information disclosure | Through the results actions, a viewer without access obtains an `authenticated` publication's manifest or SQLite, an election outside their token or another area's artefact | No manifest path in the index and private storage for `authenticated` publications' artefacts (`../windmill/src/services/results_publication.rs` `refresh_public_results_index`, `publish_private_artifacts`); `authorize_results_reader` requires, for the `results-portal` client (`is_results_portal_client`), the event realm in `iss` and an `authorized_election_ids` match, and for other clients a `publish-results-read` or `publish-results-write` role, with the tenant taken from the token; `manifest_for_reader` and `fetch_results_artifact_request` return only the token's `area_id` artefact for area-based publications; the `access` branch in `src/routes/ResultsRoute.tsx` is UX only | Mitigated |
| results-portal-T5 | Spoofing | Sign-in is hijacked: the authorization code or tokens are intercepted, the redirect goes elsewhere, or the viewer signs in to the wrong realm | keycloak-js standard flow with `responseMode: "fragment"` and a fragment-free redirect URI; Keycloak client redirect URIs limited to `RESULTS_PORTAL_URL/*` | Partial |
| results-portal-T6 | Elevation of privilege | The access token held by the portal is used beyond reading results, after script injection or a leak | Tokens stay in keycloak-js memory; no web storage in `src/`; the Bearer token goes only to `HASURA_URL` (`src/services/graphql.ts` `graphqlFetch`), never to the bucket | Partial |
| results-portal-T7 | Elevation of privilege | Script injection through published names, translation overrides or labels runs on the portal origin | Names rendered through React; overrides applied through i18next; class names from IDs pass `src/services/cssClassNames.ts` `cssClassToken`; custom CSS is `<style>` text and cannot add markup | Not verified |
| results-portal-T8 | Tampering | Admin-authored styling or wording misrepresents the figures, or makes viewers' browsers contact third parties | CSS and translation overrides are frozen into the publication at publish time, which needs `publish-results-write` and is logged by windmill; CSP from the ingress (deployment) | Partial |
| results-portal-T9 | Information disclosure | Published results let someone infer how individual voters voted, or reveal data beyond results | windmill copies only allow-listed columns of the selected contests into the published SQLite; the portal shows what the publication contains | Not verified |
| results-portal-T10 | Information disclosure | The public index shows that non-public publications exist, with tenant, event, election and publication IDs | Index entries for `authenticated` publications carry no manifest path (windmill `refresh_public_results_index`) | Accepted (by design: the index holds no figures) |
| results-portal-T11 | Repudiation | A disputed page cannot be tied to the publication and tally it came from | The manifest carries `publication_id`, `version`, `tally_session_id`, `tally_session_execution_id` and `results_event_id` (`src/types/results.ts` `ResultsManifest`); the page shows the version; windmill logs publish and revoke actions to the electoral log (`post_results_publication_action`) | Partial |
| results-portal-T12 | Denial of service | Large or malformed artefacts, or results-night load, leave the page unusable | sql.js runs in a WebAssembly sandbox; fixed queries and per-table fallback (`queryTable`); load failures show a generic message without server error text (`ResultsRoute`); capacity is left to object storage and the CDN | Partial |
| results-portal-T13 | Tampering | Clickjacking, or other attacks on the portal origin that browser security headers limit | Security headers set at the ingress (deployment) | Not verified |
| results-portal-T14 | Tampering | Reverse tabnabbing from the `target=_blank` logo link without `rel=noopener` in ui-essentials `Header`, rendered by `src/App.tsx` `AppHeader` | None on this branch; the link target is fixed to `https://sequentech.io` | Open on release/10.0 (fix in sequentech/step#3522) |
| results-portal-T15 | Tampering | A compromised dependency or `sql-wasm.wasm` ships in the bundle | `yarn install --frozen-lockfile` in `../Dockerfile.prod`; `sql-wasm.js` and `sql-wasm.wasm` copied from the locked `sql.js` package and served from the portal origin (`webpack.config.cjs`, `src/services/sqliteResults.ts` `locateFile`) | Partial |

## Assumptions

- windmill is the only writer of `results-index/`, manifests and public SQLite files. It publishes only allow-listed columns, keeps `authenticated` artefacts in private storage behind short-lived presigned URLs, and deletes revoked and superseded artefacts (windmill).
- Hasura validates JWT signatures and issuers, and exposes the two results actions only to the `user` and `admin-user` roles. windmill `authorize_results_reader` is the access decision (hasura, harvest, windmill).
- Hasura permissions limit what a `results-portal` token can read (hasura).
- The event realm's `results-portal` client allows only `RESULTS_PORTAL_URL` redirects and grants no more than reading results needs (keycloak).
- The ingress serves the portal over TLS with browser security headers, including a CSP that limits scripts, styles, images and connections to the portal, Hasura, Keycloak and bucket origins. The public bucket is served on an origin separate from the portal.
- Object storage and any CDN in front of it carry results-night load, honour the `no-store` index and manifest, and allow the portal origin through CORS.
- Election managers with presentation and `publish-results-write` rights are trusted not to misrepresent results through styling or wording. Their publish actions are in the electoral log (windmill).
- Operators protect `/global-settings.json` and the build pipeline.

## Review focus

1. Origin, integrity and freshness of published artefacts: bucket write access, revocation, intermediate caches, and whether a reader can tie a page to a tally.
2. Server-side authorization of `resolveResultsPublication` and `fetchResultsArtifact` in windmill: realm and client binding, election and area filtering, tenant scoping of admin tokens and presigned URL lifetime.
3. Scope of the `results-portal` Keycloak client and of its token.
4. Admin-authored content on the public origin: custom CSS, translation overrides and names.
5. Statistical disclosure in published results.
6. Availability under results-night load.
