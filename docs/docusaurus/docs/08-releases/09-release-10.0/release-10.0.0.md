---
id: release-10.0.0
title: Release 10.0.0
---
<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->
# Release 10.0.0

Sequent 10.0 adds telephone voting, a dedicated results website, reviewed entry and import of paper-ballot results, and tools to reconcile voter records with Datafix. It also makes voter records easier to manage, expands login and ballot options, and improves accessibility.

These notes cover **v9.5.0 → v10.0.0**, including changes introduced during the v10 release candidates. Optional features require configuration and, in some cases, additional services. See [Before upgrading](#before-upgrading) for the actions needed to adopt this release.

## Voting options

### Telephone voting and shared kiosks

Voters can authenticate, hear their ballot choices, make selections and cast a vote by telephone using the IVR (interactive voice response) integration. Election staff can customize spoken prompts, manage blocked phone numbers for each event, see the event's voting number and test the experience with an emulator. Date-of-birth plus PIN login and whole blank ballots are supported. Telephone voting requires a separately deployed telephone service. [Voting flow](https://github.com/sequentech/step/pull/2791), [administration](https://github.com/sequentech/step/pull/2826), [emulator](https://github.com/sequentech/step/pull/2906).

For shared kiosks, administrators can configure the voting and login addresses and return voters to the kiosk login when they sign out. An optional setting clears a previous voter's remaining login session so that the next voter can sign in. [Kiosk settings](https://github.com/sequentech/step/pull/3128), [successive voter logins](https://github.com/sequentech/step/pull/3117).

### Clearer ballot choices and rules

- **Acclaimed contests:** show candidates elected without a vote, while allowing voters to complete the other contests. If the entire ballot is acclaimed, the voter sees the outcome without casting a ballot or receiving a voting receipt. [Acclaimed contests](https://github.com/sequentech/step/pull/3077).
- **Blank and invalid choices:** record a deliberate “blank vote” separately from a contest left unanswered. Rules for deliberately marking a vote invalid are applied consistently, and that choice can be configured to exclude other selections. [Blank choices](https://github.com/sequentech/step/pull/2853), [invalid-vote rules](https://github.com/sequentech/step/pull/2750), [exclusive selections](https://github.com/sequentech/step/pull/2949).
- **Whole blank ballots:** an optional setting lets voters confirm that they intend to leave the entire ballot blank. Results report these separately at election level. This requires multiple-contest encryption and remains distinct from Decline to Vote. [Whole blank ballots](https://github.com/sequentech/step/pull/3068).
- **Overvotes on multi-contest ballots:** when the configured rules allow voters to select more candidates than permitted for a valid vote, the ballot can preserve those selections without an encoding failure. The selections still count as an overvote under the election's rules. Publication checks the ballot's size and explains which election and area exceed the supported limit. [Overvote support and publication checks](https://github.com/sequentech/step/pull/2927).
- **Weighted voting:** supported elections can assign different voting weights to voters, with overrides for individual contests—for example, one voter's ballot can have a weight of three. Review the [restrictions and privacy implications](#review-optional-feature-limits) before enabling this feature. [Weighted voting](https://github.com/sequentech/step/pull/3005).

## Election management and reporting

### Enter, import and review external results

Election staff can enter paper or postal voting totals as tally sheets for each contest, area and voting channel. Corrections create a new version, preserving who entered and reviewed the figures. **Only approved versions are included in the tally**, alongside electronic votes. [Tally-sheet entry and review](https://github.com/sequentech/step/pull/2820).

For larger imports, staff can upload the supported CSV format or ES&S Enhanced XML, preview errors and changed figures, and submit them for approval. Imports retain their source files and detect conflicts if the records being reviewed have changed. Corrections also improve ES&S imports for contests where voters may select several candidates. [File imports](https://github.com/sequentech/step/pull/2843), [ES&S corrections](https://github.com/sequentech/step/pull/3069).

### Publish results to a dedicated website

Authorized staff can choose which completed tally results to publish, inspect publication history and withdraw a publication. Results can be public for the whole event, or require login and show either the whole event or the voter's area. The website supports election branding and translations and presents the selected results without exposing the full internal tally records. [Results website](https://github.com/sequentech/step/pull/2866).

### Participation reports and clearer totals

A new participation report covers an entire event or a single election. It shows the eligible voter count, how many distinct voters have voted, the participation percentage, the number of recorded valid votes, and when the report was generated. Voters and votes are counted separately: a voter who votes in two elections counts once in the event's voter total. [Participation report](https://github.com/sequentech/step/pull/2652).

Dashboards, tally results and reports also gain participation breakdowns by voting channel. The votes-over-time chart lets staff choose a time range and view activity by minute, hour or day. The default electoral-results PDF template adds page numbering. [Channel breakdowns](https://github.com/sequentech/step/pull/2923), [PDF page numbers](https://github.com/sequentech/step/pull/2197).

### Easier voter-record management

- **Review changes before saving.** Staff can inspect the proposed changes to a voter before confirming them. [Change confirmation](https://github.com/sequentech/step/pull/2868).
- **More useful forms.** Configurable field order, grouping and layout keep related information together. Fields respect configured character limits, and dropdowns show readable descriptions alongside stored values. [Form layout](https://github.com/sequentech/step/pull/3046), [field validation and labels](https://github.com/sequentech/step/pull/3078).
- **Clearer save errors.** When a save is rejected immediately, the editor preserves the entered values and explains the error. Datafix updates processed as background tasks report their outcome through Tasks. [Save errors](https://github.com/sequentech/step/pull/3078).
- **Useful sorting.** Voter lists support sorting by voting channel and election. Sortable columns across administration screens cycle through ascending, descending and no sorting. [List sorting](https://github.com/sequentech/step/pull/3053).
- **Track bulk deletion.** Deleting many voters runs as a background task whose progress staff can follow. [Bulk deletion](https://github.com/sequentech/step/pull/3064).

### Voting schedules and trustee workflows

Start/end schedules can target online, kiosk, early or telephone voting, applying only to channels enabled for each election. Tally checks now consistently consider the selected elections, publication status and whether the relevant voting periods have ended. An election that has stopped voting can still be published. Rejections give staff clearer explanations. [Scheduling, publication and tally checks](https://github.com/sequentech/step/pull/3219).

Trustees can re-check their saved private keys after key generation is complete. Completed ceremonies have a clearer finished view, and actions are offered only when the trustee can perform them. Uploading a key that has already been accepted is reported as already restored, rather than incorrectly labelled invalid. [Backup checks](https://github.com/sequentech/step/pull/3120), [completed ceremonies](https://github.com/sequentech/step/pull/3122), [tally participation](https://github.com/sequentech/step/pull/3124), [repeat uploads](https://github.com/sequentech/step/pull/3125).

## Voter login and experience

### More flexible login forms

Administrators can configure login using date of birth and a PIN without requiring a username. PIN/password fields support grouped entry patterns and configurable placeholder characters. Attribute-based login forms now use the configured field types, help text, choice lists and phone-number controls; optional fields can be enabled explicitly. [Date of birth and PIN](https://github.com/sequentech/step/pull/2919), [grouped entry](https://github.com/sequentech/step/pull/2912), [placeholders](https://github.com/sequentech/step/pull/3045), [consistent forms](https://github.com/sequentech/step/pull/3081).

Notification links can prefill supported login and registration fields, reducing retyping. Prefilled information still goes through the configured authentication and validation checks. An opt-in setting also lets eligible imported voters use one-time codes without a separate credential-provisioning step. SmartLink gains the second-generation integration and can use an organization's own event identifier in its links. [Prefilled fields](https://github.com/sequentech/step/pull/2907), [one-time-code setup](https://github.com/sequentech/step/pull/2918), [SmartLink](https://github.com/sequentech/step/pull/3138).

### Voter letters and protected information

Staff can generate voter information letters as password-protected, encrypted PDFs. Authorized staff retrieve the password through the document task. Event-level password policies are also configurable. [Voter letters](https://github.com/sequentech/step/pull/2926).

Designated custom voter fields can be encrypted in storage, with separate permissions to read or change them. Authorized workflows can use them in letters, communications and exports, or as a login credential when explicitly configured. Upgrading does not automatically encrypt existing plaintext fields. [Encrypted voter information](https://github.com/sequentech/step/commit/cf1413d97deb70b92aaa37feda12cb9cfaa937b2).

### Accessibility, presentation and supporting documents

The Voting Portal improves keyboard navigation, visible focus, contrast, headings, error messages and dialogs. Election cards remain clickable across the card while keeping accessible controls. These improvements address specific accessibility issues; they do not constitute a complete accessibility certification. [Accessibility improvements](https://github.com/sequentech/step/pull/3097), [review and error handling](https://github.com/sequentech/step/pull/3221), [election cards](https://github.com/sequentech/step/pull/3302).

Voters also get shorter Ballot IDs with a copy button. For closed-list elections, review and verification screens show the candidates belonging to the selected list. Administrators can configure date/time formats and customize translations separately for each portal; designers gain more reliable controls for styling individual interface elements. [Ballot IDs](https://github.com/sequentech/step/pull/2857), [copy button](https://github.com/sequentech/step/pull/3060), [closed lists](https://github.com/sequentech/step/pull/2753), [date/time formats](https://github.com/sequentech/step/pull/2867), [portal translations](https://github.com/sequentech/step/pull/3075).

Events can require voters to open supporting documents and acknowledge them before voting. Existing events need their support-materials setting reviewed during the upgrade; see the checklist below. [Required supporting documents](https://github.com/sequentech/step/pull/3101).

## Datafix integration

A new reconciliation workflow compares Sequent's voter records with a Datafix CSV. Staff review the differences, download the changes that need to be applied in Datafix, and then upload a fresh comparison file before applying the remaining changes in Sequent. This provides a deliberate, reviewable process for resolving mismatched records. [Voter reconciliation](https://github.com/sequentech/step/pull/2917).

The live integration also improves:

- **Vote-status updates:** communication with Datafix after a vote is cast moves to background processing. [Background updates](https://github.com/sequentech/step/pull/2904).
- **Election lookup:** fix an error that prevented Datafix from retrieving election events after identifier fields changed. [Election lookup](https://github.com/sequentech/step/pull/2813).
- **PIN replacement:** a replacement PIN is no longer incorrectly marked as temporary. [PIN correction](https://github.com/sequentech/step/pull/3134).
- **Audit records:** incoming Datafix requests record more useful details about voter changes. [Electoral logs](https://github.com/sequentech/step/pull/3166).
- **Area matching:** the poll component is always `000`, while the ward and any school-support component are retained—for example, `00-P-000`. Differences in the incoming poll number no longer cause an area mismatch; the ward and school-support values must still match an existing area. [Area matching](https://github.com/sequentech/step/pull/3245).
- **Data exposure:** Datafix configuration is removed from the ballot data delivered to voters. [Publication filtering](https://github.com/sequentech/step/pull/3306).

## Other fixes

### Voting and login

- Fix cases where going Next and then Back prevented further progress, and prevent advancing while a required contest has not been validated. Clearing selections now also clears an explicit blank choice. [Back/Next navigation](https://github.com/sequentech/step/pull/2882), [contest validation](https://github.com/sequentech/step/pull/3089), [clearing selections](https://github.com/sequentech/step/pull/2645).
- Correct Decline to Vote navigation and selection limits, and remove the duplicate ballot-audit action. [Decline navigation](https://github.com/sequentech/step/pull/2827), [selection limits](https://github.com/sequentech/step/pull/2891), [audit action](https://github.com/sequentech/step/pull/3095).
- Restore configured external sign-in providers on deferred login forms; keep login messages visible when a default language is enforced; fix one-time-code screens when older configurations omit newer settings. [External sign-in](https://github.com/sequentech/step/pull/2667), [login messages](https://github.com/sequentech/step/pull/2658), [code screens](https://github.com/sequentech/step/pull/2950).
- Improve English and French wording, translated content and formatted messages. Correct mobile login-header display and date fields accepting years longer than four digits. [Translations](https://github.com/sequentech/step/pull/2934), [formatted messages](https://github.com/sequentech/step/pull/3076), [mobile header](https://github.com/sequentech/step/pull/3055), [date entry](https://github.com/sequentech/step/pull/3000).

### Administration, tallying and reliability

- Improve imports of large election events and allow shared email addresses during voter import when the event's identity settings permit them. [Large imports](https://github.com/sequentech/step/pull/2737), [shared email addresses](https://github.com/sequentech/step/pull/2691).
- Correct dashboard eligible-voter counts and refresh them after voter changes; exclude service accounts from those counts. [Dashboard totals](https://github.com/sequentech/step/pull/3220), [service accounts](https://github.com/sequentech/step/pull/2955).
- Restore missing candidate labels when reordering candidates, improve image removal, and keep usernames visible to administrators even when hidden on voter-facing forms. Certificate imports give clearer feedback about accepted, skipped and rejected certificates. [Candidate labels](https://github.com/sequentech/step/pull/2646), [image removal](https://github.com/sequentech/step/pull/2679), [usernames](https://github.com/sequentech/step/pull/3088), [certificate imports](https://github.com/sequentech/step/pull/2738).
- Restore recount execution and correct result percentages for contests allowing several selections. Preserve the configured order in results and reports, and fix tallying of acclaimed contests with single-contest encryption. [Recounts](https://github.com/sequentech/step/pull/3028), [percentages](https://github.com/sequentech/step/pull/3062), [ordering](https://github.com/sequentech/step/pull/3066), [acclaimed contests](https://github.com/sequentech/step/pull/3114).
- Restore activity-log and S3-backed event exports. Fix missing automatic downloads and duplicate PDF downloads. [Activity logs](https://github.com/sequentech/step/pull/3023), [event exports](https://github.com/sequentech/step/pull/3021), [automatic downloads](https://github.com/sequentech/step/pull/2733), [duplicate downloads](https://github.com/sequentech/step/pull/2728).
- Improve trustee key-upload handling and messages when a key-download page is no longer current. Correct the Ballot Verifier's reconstruction of ballots opened for audit and make its contest order match the configured ballot order. [File handling](https://github.com/sequentech/step/pull/3118), [expired download steps](https://github.com/sequentech/step/pull/3307), [ballot verification](https://github.com/sequentech/step/pull/3127), [contest order](https://github.com/sequentech/step/pull/2718).
- Fix a login-service connection leak that could exhaust messaging connections, honor the configured SMS sending number, and update vulnerable software dependencies. Default settings for new organizations also revise two-factor authentication. Existing organizations need their settings reviewed separately. [Connection leak](https://github.com/sequentech/step/pull/2878), [SMS sender](https://github.com/sequentech/step/pull/3136), [dependency updates](https://github.com/sequentech/step/pull/3150), [authentication defaults](https://github.com/sequentech/step/pull/3170).

## Platform and developer improvements

**Ballot delivery:** published ballot content is served from private S3 storage through temporary authorized links, while the portal requests only the live voter status it needs. This reduces repeated database work during voting. Backend changes also narrow the checks made when a vote is cast and coordinate simultaneous submissions. Existing publications without these files must be replaced with newly generated publications before switching to this delivery method. Event backups also include the private publication files, and imports validate them before restoring publication status. [Backend](https://github.com/sequentech/step/pull/3160), [Voting Portal](https://github.com/sequentech/step/pull/3161).

**Load testing:** `step-cli load` can prepare synthetic elections, simulate distinct voters using k6 or Chromium, and report response times, throughput and outcomes. Local, Docker and Kubernetes execution configurations are included. Reported validation covered local and Docker execution; live Kubernetes execution was not verified. Test results must be measured for the intended deployment rather than treated as a guaranteed capacity. [Load-testing tools](https://github.com/sequentech/step/pull/3152).

**Extensions:** an initial WebAssembly plugin framework lets developers add integrations through defined application hooks, API routes and background tasks. Plugins are loaded from configured storage and require development and deployment work. [Extension framework](https://github.com/sequentech/step/pull/1879).

**Self-hosting documentation:** expanded Docker Compose deployment scripts, example configuration and a standalone deployment guide help technical teams set up an internet-accessible development, testing or demonstration environment. This single-server guide is not a production deployment specification. [Deployment tooling and guide](https://github.com/sequentech/step/pull/2257).

## Before upgrading

This major release needs a coordinated upgrade. Election managers and deployment teams should agree on the following before rollout:

- Schedule the upgrade outside an active election and use matching application components.
- Apply the database and identity-configuration changes, and regenerate publications that lack the required ballot files.
- Review permissions before enabling telephone voting, results publication, encrypted voter information or new report workflows.
- Reconfigure existing support materials explicitly; otherwise they default to hidden in v10.
- Plan migration separately from export/import: the standard importer rejects v9.5 election-event exports in v10.

The following checklist is intended for deployment teams.

### Coordinate the application and database upgrade

This is a major-version upgrade. Ballot encoding and signed presentation structures change, including explicit blank/invalid handling and acclamation. Plan the rollout outside an active election; deploy matching backend, portal, verifier and tally components and generate publications using the intended v10 configuration. Do not assume previously signed ballots can be reinterpreted using the new structures.

Apply the release's **18 new `backend-db` migrations and matching Hasura metadata**. These include tally-sheet versioning and imports, results publications, phone blacklists, cast validation/indexing and voting schedules. The name/alias migration renames database `alias` columns to `external_id` and moves names into presentation data; review custom GraphQL clients, SQL queries and reports that address those columns. [Schema migration](https://github.com/sequentech/step/blob/v10.0.0/hasura/migrations/backend-db/1772358027729_alias_to_external_id_and_move_name_to_presentation/up.sql).

Event import now requires the same major version and does not accept a newer minor version into an older one. A **v9.5 event export is rejected by v10.0** through the normal importer. Export/import is therefore not a substitute for a separately planned database/configuration upgrade. [Compatibility check](https://github.com/sequentech/step/blob/v10.0.0/packages/sequent-core/src/util/version.rs).

### Regenerate S3 publications before switching the portal

On populated databases, run `scripts/postgres/ballot_style_voter_reference_index.sql` against each writer **before Hasura migrations**, outside a transaction. Also run `scripts/postgres/cast_vote_covering_index.sql` separately outside a transaction. Allow time for backfills/index creation and resolve invalid schedules before retrying failed migrations.

Deploy the backend and action metadata, then generate and publish a **new publication** for each event whose existing publication lacks the prepared private files, before switching the Voting Portal. The `prepare_ballot_files` example uses the normal generation path; it cannot retrofit an already-generated publication without files. New publication generation prepares the objects automatically, and publications without them cannot be used by the new portal. Configure the private bucket, a browser-reachable `AWS_S3_PUBLIC_URI`, and the correct portal CORS policy. Retain active S3 objects and database ballot styles. [Tagged rollout instructions](https://github.com/sequentech/step/blob/v10.0.0/scripts/postgres/README.voting-publication.md).

If an earlier development version of migration `1788765000002` or `1788909000000` was applied, follow the documented rollback/reapply procedure using the original migration files. Those migrations changed before release; replacing files does not rerun an already-recorded migration. The final release uses schedule indexes and validation, not the earlier voting-window projection.

### Update templates, realm defaults and feature permissions

Make the v10 report templates available in the deployment, including the new participation and voter-information-letter templates. Review any customized templates before replacing them; the supplied electoral-results templates include the new participation sections and PDF page numbers.

Default tenant and election-event realms are now read from S3. Upload the matching templates and configure `KEYCLOAK_TENANT_REALM_CONFIG_S3_KEY` and `KEYCLOAK_ELECTION_EVENT_REALM_CONFIG_S3_KEY`; local-file configuration alone no longer supplies them. The default templates include revised tenant 2FA/required-action settings, conditional passwordless WebAuthn, and `no-reply@sequent.vote` with SMTP host `localhost`. Verify the actual email transport and sender authorization for each deployment. Updating templates does **not** update existing realms. [S3 defaults](https://github.com/sequentech/step/pull/2855), [authentication and email defaults](https://github.com/sequentech/step/pull/3304).

Review existing realm roles before enabling the new features. Relevant additions include `publish-results-read/write`, `tally-sheet-import-view/create/review`, `tally-recount-execute`, `voter-information-letter`, `document-password-read`, the reconciliation and IVR permissions, and `voter-secret-attribute-read/write`. Mandatory support materials require `ack-support-materials` in the event realm and its voter-group mapping. Grant each capability to its intended operators or voters.

For existing events with support materials, explicitly set `presentation.materials.policy` to `optional` or `mandatory_for_voting` as intended and regenerate the publication before voting. In this tag, an absent policy resolves to `off`; the legacy `materials.activated` flag does not preserve visibility. [Policy resolution](https://github.com/sequentech/step/blob/v10.0.0/packages/ui-core/src/types/ElectionEventPresentation.ts).

For encrypted-attribute login, supply the same existing `MASTER_SECRET` to Keycloak and the backend services, and explicitly configure the secret-attribute credential policy. For results publication, configure `RESULTS_PORTAL_URL`, deploy the results portal, and provision its dedicated Keycloak client. For IVR and HMAC SmartLink, use the Beyond revision pinned by this release rather than an unrelated checkout.

### Review optional-feature limits

- **Voter-weighted voting** supports plurality-at-large (selecting candidates without ranking them) and must not be combined with conflicting area weights. Voting weights are visible in the election audit data, and small or distinguishable weight groups can expose voter choices. The implementation caps individual weights at 100,000 and summed weights at 1,000,000 per contest area; the summed limit is checked during tally preparation, so validate it before voting. Do not use it where voting weights must remain secret. [Implementation and tradeoffs](https://github.com/sequentech/step/pull/3005).
- **Whole blank ballots** require multiple-contest encryption and an enabled per-election policy. They are distinct from explicit blank selections within a contest and from Decline to Vote. [Blank-ballot behavior](https://github.com/sequentech/step/blob/v10.0.0/docs/docusaurus/docs/02-election_managers/02-reference/09-blank-ballots.md).
- **Tally-sheet imports** require unambiguous contest external IDs within the event and matching area/candidate identifiers. Verify these mappings before importing external results. [Import format](https://github.com/sequentech/step/blob/v10.0.0/docs/docusaurus/docs/02-election_managers/02-reference/02-election-event/08-03-election_management_election-event_tally-sheet-imports.md).
- **Existing voting schedules** with omitted, null or empty channel lists retain the online-and-kiosk default. Review those defaults when enabling early or telephone voting. [Schedule compatibility](https://github.com/sequentech/step/pull/3219).

Self-hosted Compose deployments also switch the MinIO and `mc` image sources to Quay because the previous Docker Hub repositories were no longer publicly pullable. Keycloak remains **26.6.1**, the same base version used by v9.5.0. [Container fix](https://github.com/sequentech/step/pull/3280).

[Compare the release tags](https://github.com/sequentech/step/compare/v9.5.0...v10.0.0).
