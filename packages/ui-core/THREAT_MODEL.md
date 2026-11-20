<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# ui-core threat model

ui-core (`@sequentech/ui-core`) is a TypeScript library that webpack bundles into an ES module (`dist/index.js`, entry `src/index.tsx`). voting-portal, admin-portal, ballot-verifier, results-portal and ui-essentials import it, so it runs in the voter's browser (or on a kiosk device) and in the election manager's browser. It provides the HTML sanitizer for election-manager text (`stringToHtml`), i18next setup and translation overrides, translation lookup in presentation objects, thin wrappers around the sequent-core WASM ballot functions (encrypt, hash, sign, verify, decode), candidate categorisation and ordering, download and cookie helpers, and the shared configuration types. It makes no network requests of its own. It matters to an election for two reasons: its sanitizer is what stops content authored in admin-portal from running script in a voter's session, and its wrappers sit on the path a ballot takes from the voter's selections to the ciphertext. See the [system threat model](../../THREAT_MODEL.md).

## Assets

- **Portal origin and sessions**: the voter's and the admin's Keycloak tokens, held by the consuming portal in the same origin. Confidentiality: script injected through rendered content can read them.
- **Ballot selections and auditable ballot**: the data passed through `src/services/wasm.ts` to sequent-core for encoding, encryption, hashing and signing. Integrity (cast as intended) and confidentiality.
- **Election content shown to voters**: instructions, candidate names and descriptions, translations and overrides. Integrity: voters act on what they read.
- **sequent-core WASM package** (`rust/sequent-core-0.1.0.tgz`, the peer dependency every portal bundles): the code that encrypts ballots in the browser. Integrity.
- **Candidate, contest and category order**: integrity, because ballot position must not be biased when an election uses random order.
- **Language preference** (`USER_LANGUAGE` cookie, `src/utils/cookies.ts`). Low value. Integrity.

## Entry points and trust boundaries

| Entry point | Who can reach it | Trust | Checks in place |
|---|---|---|---|
| `src/services/stringToHtml.ts` `stringToHtml`, `stringToText` (presentation text, candidate descriptions, translation overrides) | Election managers through admin-portal or election event import; portal translation files | authenticated admin | `escapeStrayAngleBrackets`, then sanitize-html 2.17.5 with `SANITIZE_OPTIONS` (default tag list; attributes limited to `lang`/`dir`, `a` link attributes and table-cell attributes); output parsed by html-react-parser into React elements |
| `src/services/stringToHtml.ts` `translateHtml`, `escapeTranslationValues` | Values interpolated into translations (ballot IDs, names, counts) | untrusted | `escapeHtml` on every value that is not a number, boolean, `null` or `undefined`; the `TranslationValue` type excludes objects and arrays at compile time |
| Language selection: the `language` argument of `src/services/i18n.ts` `initializeLanguages` (portals pass `?lang=`), the `USER_LANGUAGE` cookie read by `applyConfigurationLanguagePolicy`, and i18next-browser-languagedetector with its default sources when no language is given | Anyone who crafts a link or sets the language cookie | untrusted | The page's `lang` attribute is set through `setAttribute` |
| `src/services/i18n.ts` `overwriteTranslations` (election event, tenant or admin `i18n` overrides) | Election managers and tenant admins | authenticated admin | Scoped overrides: `src/services/translationScopes.ts` `filterTranslationOverrides` (per-portal scopes), stored with i18next `addResource` |
| `src/services/translate.ts` `translate`, `translateFromPresentation` | Configuration from Hasura or the public bucket | authenticated admin | Type checks on presentation data: `isRecord`, and `getTranslatedValue` returns only non-empty strings |
| `src/services/wasm.ts` `encryptBallotSelection`, `hashBallot`, `signHashableBallot`, `interpretContestSelection` and their multi-ballot forms | The voter's own selections and the ballot style from Hasura | authenticated voter | Pass-through to sequent-core; errors are rethrown |
| `src/services/wasm.ts` `decodeAuditableBallot`, `verifyBallotSignature`, `verifyAuditableBallotCiphertext` and multi-ballot forms | An auditable ballot pasted or uploaded into ballot-verifier | untrusted | Parsing and checks in sequent-core; errors are rethrown |
| `src/services/categoryService.ts` `categorizeCandidates`, `getShuffledCategories`; `src/services/presentationOrder.ts` `sortByPresentationOrder` | Contest and candidate configuration | authenticated admin | `parseEntityPresentation` (JSON parse, object check) and a finite-number check on `sort_order` |
| `src/services/downloadBlob.ts` `downloadUrl`, `downloadBlob` | URLs from harvest `fetchDocument` (every current caller in admin-portal and voting-portal); blobs built in the browser | internal service | `downloadBlob` revokes its object URL after the click |
| `src/services/sanitizeFilename.ts` `sanitizeFilename`; `src/services/cssClassNameFormatter.ts` `toValidClassName`, `getContestClassName`; `src/services/normalizeWriteInText.ts` `normalizeWriteInText` | Admin-authored names; the voter's write-in text | authenticated admin / authenticated voter | Character allowlists; length limits on file and class names (write-in length is checked by sequent-core through `getWriteInAvailableCharacters`) |
| `src/services/votingPortalDateTime.ts` `parseVotingPortalDateTimePattern`, `formatVotingPortalDateTime` | Election managers setting a custom date format | authenticated admin | Pattern must contain a known token and no misused token; output is a plain string |
| `rust/sequent-core-0.1.0.tgz` | Maintainers committing a new build | operator | Maintainer build process (see Assumptions) |

## Threats

| ID | STRIDE | Threat | Controls in the code | Status |
|---|---|---|---|---|
| ui-core-T1 | Elevation of privilege | Script in election-manager text or translations runs in the voter's or admin's session and steals the token or changes the ballot | `src/services/stringToHtml.ts` `stringToHtml` (sanitize-html allowlist, no event handlers, no `style`, no `script`); `escapeStrayAngleBrackets` runs before the sanitizer and cannot widen it; `src/services/stringToHtml.test.ts` covers scripts, `onclick` and `javascript:` links; tests run in CI (`.github/workflows/tests.yml`) | Mitigated |
| ui-core-T2 | Tampering | A value interpolated into a translation that is rendered as HTML becomes markup, for example a link inside an error message | `translateHtml` and `escapeTranslationValues` escape every string value, including under i18next's `{{- }}` syntax; tests cover nested values. `initializeLanguages` sets `escapeValue: false`, so safety depends on consumers calling `translateHtml` | Partial |
| ui-core-T3 | Spoofing | Allowed markup in election-manager text imitates portal controls or links voters to another site | `SANITIZE_OPTIONS` allows no global `id`, `role` or `aria-*`, so content cannot relabel the start-screen confirmation or hide text from assistive technology; links use sanitize-html's default URL schemes | Partial |
| ui-core-T4 | Denial of service | Malformed configuration or input breaks page rendering in the portals | Parsing against known values in `src/services/translationScopes.ts` `parseTranslationOverrideKey` and `src/services/presentationOrder.ts` `parseEntityPresentation` | Partial |
| ui-core-T5 | Information disclosure | The language cookie is shared with every host under the same parent domain | `src/utils/cookies.ts` `setCookie`: `Path=/`, `SameSite=Lax`, `Secure` on HTTPS, URI-encoded value | Accepted (portals on different subdomains share the voter's language choice; the cookie carries no secret) |
| ui-core-T6 | Tampering | A wrapper alters the selections it passes to sequent-core, or a verification wrapper fails open and the portal shows an unverified ballot as valid | `src/services/wasm.ts`: every wrapper passes its arguments unchanged; encryption, hashing, signing, decoding and all `verify*` wrappers rethrow on error; only display helpers (`checkIsBlank`, `getLayoutProperties`, `getPoints`, `generateSampleAuditableBallot`) return `null`. Acting on a `false` verification result is left to ballot-verifier | Mitigated |
| ui-core-T7 | Tampering | The WASM package that encrypts ballots in every portal is tampered with or does not come from the reviewed sequent-core source | Maintainer build process (see Assumptions) | Not verified |
| ui-core-T8 | Information disclosure | Ballot data or internal state reaches the browser console on a shared device | Error messages come from sequent-core; whether any contains selections is not confirmed | Not verified |
| ui-core-T9 | Tampering | Random candidate, contest or category order is predictable or biased | `src/utils/array.ts` `shuffle` (moderndash 4.0.0) used by `getShuffledCategories`; `sortElectionList`, `sortContestList`, `sortCandidatesInContest` delegate to sequent-core. The randomness source of `shuffle` is not confirmed | Not verified |
| ui-core-T10 | Elevation of privilege | A download helper opens a URL that is not a server-issued document URL or a local object URL | Callers pass presigned URLs from harvest `fetchDocument` or object URLs from `downloadBlob` | Partial |
| ui-core-T11 | Tampering | Admin-authored names injected into download file names or CSS class names escape their context | `src/services/sanitizeFilename.ts` `sanitizeFilename` (drops path separators, characters reserved in file names and control characters, then trailing dots and spaces; length limit); `src/services/cssClassNameFormatter.ts` `toValidClassName` (`[a-zA-Z0-9-_]`, prefix, 40 characters) | Mitigated |
| ui-core-T12 | Tampering | A known vulnerability in sanitize-html, html-react-parser or i18next reaches the portals | Exact pins for `sanitize-html` and `html-react-parser` in `package.json`, `resolutions` for transitive packages | Not verified |
| ui-core-T13 | Information disclosure | Admin-authored image or link URLs make the voter's browser contact a third-party host, revealing the voter's IP address and when they view the ballot | None in ui-core: `src/services/candidatePresentation.ts` `getImageUrl` returns the configured URL; depends on the portals' Content Security Policy | Not verified |

## Assumptions

- **Consuming portals** (voting-portal, admin-portal, ballot-verifier, results-portal, ui-essentials) render election-manager text only through `stringToHtml` or `translateHtml`, never through `dangerouslySetInnerHTML`, and call `translateHtml` whenever a translation rendered as HTML interpolates values.
- **sequent-core** does all ballot encoding, encryption, hashing, signing and proof checking; ui-core adds no cryptography (see the sequent-core and strand models).
- **Hasura** permissions decide which configuration data reaches each portal.
- **harvest** `fetchDocument` returns only HTTPS presigned object-storage URLs, which callers pass to `downloadUrl`.
- **admin-portal and Keycloak** limit who can edit presentation, translations and candidate data. Election managers are trusted to author content, not to run script.
- **The ingress** serves the portals with a Content Security Policy that also limits which hosts images and links can load from; ui-core sets none. Portals inject tenant and election custom CSS themselves; ui-core only builds the class names it targets.
- **Maintainers** build `rust/sequent-core-0.1.0.tgz` from reviewed sequent-core source and commit the same file to every portal.

## Review focus

1. `src/services/stringToHtml.ts`: the sanitize-html allowlist and its interaction with html-react-parser. It is the only script barrier for election-manager content in every portal, and any change to tags, attributes or schemes widens it.
2. The translation pipeline in `src/services/i18n.ts`, `translationScopes.ts` and `translate.ts`.
3. Consumer call sites that render translations as HTML with interpolated values, since i18next runs with `escapeValue: false`.
4. The provenance of `rust/sequent-core-0.1.0.tgz`.
5. `src/services/wasm.ts` on the ballot path: fail-closed error handling, and what reaches the console.
6. Candidate, contest and category ordering, including the randomness behind shuffled order.
7. Browser helpers for downloads and cookies.
