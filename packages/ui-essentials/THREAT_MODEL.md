<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# ui-essentials threat model

ui-essentials (`@sequentech/ui-essentials`) is the shared React component library that voting-portal, admin-portal, ballot-verifier and results-portal import. webpack builds it from `src/index.tsx` into `dist/index.js`. It has no server side. It calls no API and uses no browser storage. It runs in the browser of a voter, election manager, trustee or auditor, inside the origin of whichever portal imports it, so the portal's tokens are within its reach. It draws the candidate rows voters select from, the decoded ballot that ballot-verifier shows, the ballot ID and its QR code, the election list, the header with the signed-in identity, the file drop zone used for trustee key shares and auditable ballots, and the published results charts. A rendering or script-injection flaw here affects every portal at once. See the [system threat model](../../THREAT_MODEL.md).

## Assets

- **Portal origin and session**: components run in the same origin as the importing portal's Keycloak tokens and GraphQL client. Confidentiality and integrity: any script a component lets run acts as the voter or admin.
- **Selection and review display**: candidate checkboxes, preference positions and write-in fields (`Candidate`, `CandidatesList`), and the decoded ballot view (`PlaintextVoteContest`). Integrity: cast-as-intended and the Benaloh audit depend on the voter seeing exactly the state the caller holds.
- **Ballot ID and tracker link**: `BallotHash`, `BallotHashCopyButton`, `QRCode`. Integrity.
- **Files passed through the drop zone**: trustee private key shares, auditable ballots, CA certificates and import files (`DropFile`, `CustomDropFile`). Confidentiality and integrity. The package only forwards them.
- **Signed-in identity**: name, username and email shown by `Header` and `ProfileMenu`. Confidentiality on shared and kiosk devices.
- **Published results presentation**: `ResultsAndParticipation` and its tables and charts in results-portal and admin-portal. Integrity.
- **Built bundle** `dist/index.js`: integrity, since all four portals ship it.

## Entry points and trust boundaries

The package has no trust boundary of its own. Every input arrives as a prop from the importing portal, and trust is that of the data's original author.

| Entry point | Who can reach it | Trust | Checks in place |
|---|---|---|---|
| `src/components/Candidate/Candidate.tsx` `Candidate` (`title`, `description`, `url`, image children, write-in `TextField`) | Election managers through configuration; voters typing a write-in | authenticated admin / authenticated voter | Strings rendered as React children; checked state, write-in value and position controlled by the caller |
| `src/components/PlaintextVoteContest/PlaintextVoteContest.tsx` `PlaintextVoteContest` (`question`, `questionPlaintext`) | Anyone who gives an auditable ballot file to a ballot-verifier user | untrusted | Strings as React children; acclamation description through ui-core `stringToHtml` (sanitize-html allowlist); `normalizeMessageMap` on error parameters |
| `src/components/SelectElection/SelectElection.tsx` `SelectElection` (`title`, `electionHomeUrl`, `resultsUrl`, `electionDates`, `formatDateTime`) | Election managers, through voting-portal's election list | authenticated admin | Strings as React children; vote and ballot-locator actions are caller callbacks |
| `src/components/Header/Header.tsx` `Header`, `src/components/ProfileMenu/ProfileMenu.tsx` `ProfileMenu` (`userProfile`, `logoUrl`, `logoLink`, `expiry`) | The signed-in user's own profile; election managers (logo) | authenticated voter / authenticated admin | Name rendered through react-i18next `Trans`; username and email as text |
| `src/components/CustomDropFile/CustomDropFile.tsx` `CustomDropFile`, `src/components/DropFile/DropFile.tsx` `DropFile` | Local user picking or dropping a file (admin-portal key ceremony, tally trustees, imports, CA certificates, candidate images; ballot-verifier) | untrusted | Files handed to the caller unread; only the file name is displayed |
| `src/components/TallyResults/` `ResultsAndParticipation` and its parts | Published results; public in results-portal, and admin-portal tally pages | internal service (text authored by election managers) | `utils.ts` `toFiniteNumber`; `constants.ts` `MAX_CANDIDATES_REPRESENTED` |
| `src/components/BallotHash/BallotHash.tsx` `copyBallotHash`, `src/components/QRCode/QRCode.tsx` `QRCode` | Ballot ID and tracker URL computed by voting-portal | internal service | `copyBallotHash` writes only the `hash` string on an explicit click and reports failure instead of throwing |
| `src/components/LanguageMenu/LanguageMenu.tsx` `LanguageMenu` (`languagesList`) | Election managers through configuration, via `Header` `languagesList`; the user picks one entry | authenticated admin | Entries rendered as text through `t("language", {lng})` and passed to `i18n.changeLanguage`; `LanguageSetter` is used only in stories |
| `src/components/ReviewChangesTable/ReviewChangesTable.tsx`, `src/components/CustomAutocompleteArrayInput/CustomAutocompleteArrayInput.tsx` | admin-portal user and report forms | authenticated admin | Strings as React children; labels split on whitespace and de-duplicated |

## Threats

| ID | STRIDE | Threat | Controls in the code | Status |
|---|---|---|---|---|
| ui-essentials-T1 | Elevation of privilege | Script injection through election text, voter-supplied text or translations rendered by the package's own code, acting with the portal's tokens | No `dangerouslySetInnerHTML`, `innerHTML`, `eval` or `new Function` in `src/`; strings rendered as React children; the only HTML sink is `PlaintextVoteContest` through ui-core `stringToHtml` (sanitize-html allowlist, no script or event attributes) | Mitigated |
| ui-essentials-T2 | Elevation of privilege | Script injection through text that a component hands to a third-party UI library | Text is passed as props or data, not HTML; the libraries' own rendering was not fully reviewed | Not verified |
| ui-essentials-T3 | Tampering | Reverse tabnabbing: a page opened from a `target=_blank` link without `rel=noopener` in `Candidate`, `Header` or `SelectElection` sends the portal tab to a look-alike page | `src/components/Footer/Footer.tsx` `CustomLink` sets `rel="noopener noreferrer"` and the `SelectElection` results button sets `rel="noreferrer"`; the other links set none | Open on release/10.0 (fix in sequentech/step#3522) |
| ui-essentials-T4 | Spoofing | Links and images whose targets come from election configuration send voters to unsafe schemes or hostile sites, or track them | React 19 (`react-dom@19.1.1`) blocks `javascript:` URLs in `href`; links inside sanitized HTML are limited to the sanitize-html default schemes | Partial |
| ui-essentials-T5 | Tampering | Selection controls show a state different from the one the caller encodes, so the voter casts something other than intended | `Candidate` and `CandidatesList` are controlled by the caller's `checked` and `selectedPosition`; `shouldDisable` gates clicks; no tests cover toggling | Partial |
| ui-essentials-T6 | Tampering | The decoded-ballot view hides or misstates selections, invalid or blank markers, so a Benaloh audit passes on a ballot that does not match the voter's choices | `PlaintextVoteContest` lists the selected candidates, the explicit invalid, blank and decline-to-vote markers, and the `invalid_errors` of non-acclaimed contests; acclaimed contests list all eligible candidates; `PlaintextVoteContest.test.tsx` covers acclaimed and normal contests only | Partial |
| ui-essentials-T7 | Information disclosure | Components leak ballot content, ballot IDs or identity to third parties, logs or storage | No storage API or logging of props in `src/` (`useTemplate` logs a constant only); `copyBallotHash` writes only the hash | Partial |
| ui-essentials-T8 | Tampering | Files dropped or picked (trustee key shares, auditable ballots, certificates, imports) have an unexpected type, size or content | `CustomDropFile` hands files to the caller unread and shows only their name; document `dragover`/`drop` handlers stop the browser from opening a dropped file in the tab; file validation belongs to callers | Partial |
| ui-essentials-T9 | Denial of service | Malformed election, ballot or results data throws during render and blanks a voter-facing screen | `TallyResults/utils.ts` `toFiniteNumber` turns non-numeric counts into `-`; `MAX_CANDIDATES_REPRESENTED` caps chart slices; `normalizeMessageMap` accepts only maps, entry arrays or plain objects | Partial |
| ui-essentials-T10 | Information disclosure | Version and build hash are shown in the header to anyone, including unauthenticated visitors | `Header` renders `appVersion` and `appHash` through `src/components/Version/Version.tsx` `Version` | Accepted (voters and auditors need to identify the deployed build; the source is public) |
| ui-essentials-T11 | Spoofing | Election status, "voted" markers, dates or the session countdown mislead the user | Rendered only from caller props (`isOpen`, `hasVoted`, `electionDates`, `expiry`) | Accepted (presentational; voting periods and eligibility are enforced server-side) |
| ui-essentials-T12 | Tampering | A compromised dependency or tampered `dist/` bundle runs in every portal origin | `../yarn.lock` pins versions with integrity hashes; `package.json` `resolutions` force patched transitive versions; `dist/` is git-ignored and rebuilt in CI (`../../.github/workflows/tests.yml`, `lint_prettify.yml`); `webpack.config.cjs` `externals` leave React, MUI, the other UI libraries and ui-core to each portal | Partial |

## Assumptions

- Portals pass HTML only through ui-core `stringToHtml` or `translateHtml`, and the ui-core sanitizer allowlist is sound (ui-core).
- The third-party UI libraries that portals install render the text they receive as text.
- React escapes text children and blocks `javascript:` URLs (react-dom 19). ui-core initializes i18next with `escapeValue: false` on that basis.
- Ballot encoding, hashing and validation happen in sequent-core WASM inside the caller. The caller maps component events to ballot state (voting-portal `Answer`, ballot-verifier).
- Callers validate files received from `DropFile` and `CustomDropFile` (admin-portal key ceremony and tally trustee steps, imports, CA certificates, candidate images; ballot-verifier).
- Election managers are trusted to configure link targets and logos.
- harvest, windmill and Hasura authorize every action that components trigger through caller callbacks (vote, ballot locator, logout).
- The ingress serves each portal with a CSP, frame-ancestors, nosniff and HSTS.

## Review focus

1. Ballot selection and decoded-ballot rendering (`Candidate`, `CandidatesList`, `PlaintextVoteContest`) compared with the state the caller encodes. Cast-as-intended and the audit rest on it.
2. Every path where externally authored text reaches a third-party library or an attribute instead of a React text node.
3. Link handling across components and URLs that come from configuration.
4. File intake used for key shares and auditable ballots, read together with the callers that parse the files.
5. Render robustness of voter-facing screens against malformed election, ballot and results data.
6. Dependency pinning and the CI build of the `dist/` bundle that all portals consume.
