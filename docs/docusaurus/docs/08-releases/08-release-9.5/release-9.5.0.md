---
id: release-9.5.0
title: Release 9.5.0
---
<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->
# Release 9.5.0

## 🔄 Migrations

### ✨ Keycloak upgraded from 26.4 to 26.6.1
The Keycloak base image has been bumped from `26.4` to `26.6.1` as part of the [OpenID4VP Wallet Extension Plugin](https://github.com/sequentech/meta/issues/11928) integration. The `oid4vp` extension v0.6.2 requires Keycloak 26.6.1 or later (it references `UserAuthenticationIdentityProvider`, introduced in that version).

For deployments upgrading from `9.4.x`:

1. **Keycloak image**: all Keycloak Dockerfiles now use `quay.io/keycloak/keycloak:26.6.1`. Verify your Helm/compose overrides reference the updated image.
2. **New SPI module**: the `idp-linking-authenticator` Keycloak SPI module is included for [multi-IdP user linking](https://github.com/sequentech/meta/issues/11932). If you maintain custom Keycloak provider configurations, review the new provider ID `idp-linking-authenticator`.
3. **Multiple certificates**: Election Events now support [multiple certificates](https://github.com/sequentech/meta/issues/11110). Existing single-certificate configurations continue to work without changes.

See [sequentech/meta#11928](https://github.com/sequentech/meta/issues/11928) for details.

### ✨ Import versioning constraints
Events can no longer be imported into a different major version. If you rely on cross-version imports, review [sequentech/meta#11114](https://github.com/sequentech/meta/issues/11114) before upgrading.

## 📝 Highlights

### ✨ Keycloak and authentication enhancements
Release `9.5.0` adds [OpenID4VP Wallet Extension Plugin support](https://github.com/sequentech/meta/issues/11928), including a Keycloak version bump from `26.4` to `26.6.1`. It introduces [multi-IdP user linking via custom attributes](https://github.com/sequentech/meta/issues/11932), [multiple certificate support per Election Event](https://github.com/sequentech/meta/issues/11110), and a [policy to disable browser language detection](https://github.com/sequentech/meta/issues/11799). Keycloak also gains [Catalan OTP translations](https://github.com/sequentech/meta/issues/11905) and [2FA/Passkeys across all environments](https://github.com/sequentech/meta/issues/10975).

### ✨ Voting and election-management improvements
IRV support continues to expand with an [external hat-procedure for ties](https://github.com/sequentech/meta/issues/10596), [implicit invalid vote handling policies](https://github.com/sequentech/meta/issues/10597), and [improved IRV voting experience display](https://github.com/sequentech/meta/issues/10594). The Voting Portal adds [expand/collapse all lists](https://github.com/sequentech/meta/issues/11765), [selected-candidate indicators in collapsed lists](https://github.com/sequentech/meta/issues/12099), and [voter-facing text improvements](https://github.com/sequentech/meta/issues/11924). New capabilities include a [participation report](https://github.com/sequentech/meta/issues/12102), an [election preview API](https://github.com/sequentech/meta/issues/9991), [single PDF with all-areas election results](https://github.com/sequentech/meta/issues/10595), and [paginated electoral results reports](https://github.com/sequentech/meta/issues/9535).

### 🛠 Platform and developer updates
This release [removes unsafe Rust code](https://github.com/sequentech/meta/issues/7111), introduces the [initial extension system with WebAssembly Components](https://github.com/sequentech/meta/issues/578), and implements the [step-cli happy path](https://github.com/sequentech/meta/issues/6680) with [voter generation improvements](https://github.com/sequentech/meta/issues/7254). It also adds [event config versioning](https://github.com/sequentech/meta/issues/6333) and [import constraints](https://github.com/sequentech/meta/issues/11114), along with extensive [Election Managers documentation](https://github.com/sequentech/meta/issues/10085) and [IVR telephone voting design](https://github.com/sequentech/meta/issues/11800).

### 🐞 Windmill and Harvest fail to start on production deployments with AWS S3
In AWS deployments, `windmill` and `harvest` can fail during startup while initializing the plugin manager. The failure happens when the backend tries to list plugin objects under `public/plugins/` using the current S3 abstraction, where:
- `AWS_S3_PRIVATE_URI` and `AWS_S3_PUBLIC_URI` point to a real AWS bucket URL.
- `AWS_S3_BUCKET` and `AWS_S3_PUBLIC_BUCKET` are treated as logical prefixes like `election-event-documents` and `public`.
This works against MinIO, but it breaks on AWS because `list_objects_v2` must address the real bucket and use the logical bucket as part of the prefix.
See [sequentech/meta#11570](https://github.com/sequentech/meta/issues/11570) for details.

## 📋 All Changes

### 🚀 Features

- ✨ Add OpenID4VP Wallet Extension Plugin to Keycloak image ([sequentech/meta#11928](https://github.com/sequentech/meta/issues/11928))
  by @BelSequent

- ✨ Keycloak - Map multiple IdP users to a single user via custom multi-value attribute ([sequentech/meta#11932](https://github.com/sequentech/meta/issues/11932))
  by @Findeton

- ✨ Keycloak: Configuration to support multiple certificates in Election Event ([sequentech/meta#11110](https://github.com/sequentech/meta/issues/11110))
  by @BelSequent, @Findeton

- ✨ Keycloak - Add Catalan translations for OTP ([sequentech/meta#11905](https://github.com/sequentech/meta/issues/11905))
  by @xalsina-sequent

- ✨ Policy to disable browser language detection and force the configured default language ([sequentech/meta#11799](https://github.com/sequentech/meta/issues/11799))
  by @Findeton

- ✨ Update all Envs to use 2FA/Passkeys ([sequentech/meta#10975](https://github.com/sequentech/meta/issues/10975))
  by @Findeton

- ✨ IRV: Support external hat-procedure for ties ([sequentech/meta#10596](https://github.com/sequentech/meta/issues/10596))
  by @xalsina-sequent

- ✨ IRV: Implicit Invalid vote handling policies ([sequentech/meta#10597](https://github.com/sequentech/meta/issues/10597))
  by @BelSequent

- ✨ IRV Voting Experience - Display ([sequentech/meta#10594](https://github.com/sequentech/meta/issues/10594))
  by @BelSequent

- ✨ Voting Portal: Expand/Collapse all lists ([sequentech/meta#11765](https://github.com/sequentech/meta/issues/11765))
  by @xalsina-sequent

- ✨ Voting Portal > Ballot: Indicator for selected candidates in collapsed lists ([sequentech/meta#12099](https://github.com/sequentech/meta/issues/12099))
  by @BelSequent

- ✨ Voting Portal: Voter Facing Text Improvements ([sequentech/meta#11924](https://github.com/sequentech/meta/issues/11924))
  by @GuyZequent, @BelSequent

- ✨ Add Participation Report ([sequentech/meta#12102](https://github.com/sequentech/meta/issues/12102))
  by @xalsina-sequent

- ✨ API for preview election Event ([sequentech/meta#9991](https://github.com/sequentech/meta/issues/9991))
  by @yuvalkom-M

- ✨ Velvet: Single PDF with Election results for all Areas ([sequentech/meta#10595](https://github.com/sequentech/meta/issues/10595))
  by @yuvalkom-M

- ✨ Reports > Add pagination to the electoral results report ([sequentech/meta#9535](https://github.com/sequentech/meta/issues/9535))
  by @xalsina-sequent, @Findeton

- ✨ Remove unsafe rust code ([sequentech/meta#7111](https://github.com/sequentech/meta/issues/7111))
  by @Findeton, @BelSequent

- ✨ Initial extension system support with WebAssembly Components ([sequentech/meta#578](https://github.com/sequentech/meta/issues/578))
  by @xalsina-sequent, @Findeton

- ✨ Implement happy path for CLI ([sequentech/meta#6680](https://github.com/sequentech/meta/issues/6680))
  by @yuvalkom-M, @omri81

- ✨ Step-CLI Generate voters - Allow email_verified and authorized_election_count ([sequentech/meta#7254](https://github.com/sequentech/meta/issues/7254))
  by @xalsina-sequent

- ✨ Add versioning to election event configs ([sequentech/meta#6333](https://github.com/sequentech/meta/issues/6333))
  by @omri81

- ✨ Constraint importing event into a different major version ([sequentech/meta#11114](https://github.com/sequentech/meta/issues/11114))
  by @BelSequent, @GuyZequent

- ✨ CSS per election: election alias ([sequentech/meta#9992](https://github.com/sequentech/meta/issues/9992))
  by @yuvalkom-M

- ✨ Tally > Ballot Images: add election_alias ([sequentech/meta#8680](https://github.com/sequentech/meta/issues/8680))
  by @xalsina-sequent, @BelSequent

- ✨ IVR: Move to beyond ([sequentech/meta#11109](https://github.com/sequentech/meta/issues/11109))
  by @oded-eid-sequentech

- ✨ Create deployment suite and tutorial for running step in docker compose env ([sequentech/meta#9292](https://github.com/sequentech/meta/issues/9292))
  by @oded-eid-sequentech, @BelSequent


### 🛠 Bug Fixes

- 🐞 Admin portal - Reorder candidates UI does not show the candidates text ([sequentech/meta#12050](https://github.com/sequentech/meta/issues/12050))
  by @BelSequent

- 🐞 Keycloak - When Force Default language is enabled, login screen messages disappear too quickly ([sequentech/meta#12120](https://github.com/sequentech/meta/issues/12120))
  by @Findeton

- 🐞 Keycloak: Doesn't show SSO providers in deferred login mode ([sequentech/meta#12125](https://github.com/sequentech/meta/issues/12125))
  by @xalsina-sequent

- 🐞 Voting Portal - Overvote policy not disabling other list from being selected ([sequentech/meta#12048](https://github.com/sequentech/meta/issues/12048))
  by @BelSequent

- 🐞 Voting portal - Event localization override not working for alerts and errors if ballot styles are missing ([sequentech/meta#11925](https://github.com/sequentech/meta/issues/11925))
  by @xalsina-sequent

- 🐞 Step-Cli not compiling ([sequentech/step#2647](https://github.com/sequentech/step/pull/2647))
  by @Findeton

- 🐞 Windmill and Harvest fail to start on production deployments with AWS S3 ([sequentech/meta#11570](https://github.com/sequentech/meta/issues/11570))
  by @Findeton

- 🐞 Windmill: Github action keeps failing ([sequentech/meta#10956](https://github.com/sequentech/meta/issues/10956))
  by @oded-eid-sequentech, @Findeton

- 🐞 Broken sequent-core WASM package ([sequentech/meta#11089](https://github.com/sequentech/meta/issues/11089))
  by @Findeton

- 🐞 Admin Portal > Electoral Logs > Timestamp filter does not work ([sequentech/meta#9995](https://github.com/sequentech/meta/issues/9995))
  by @BelSequent

- 🐞 Tally > Election aliases not used ([sequentech/meta#8426](https://github.com/sequentech/meta/issues/8426))
  by @xalsina-sequent, @BelSequent


### 📖 Documentation

- 📖 [doc] Design IVR Telephone Voting system ([sequentech/meta#11800](https://github.com/sequentech/meta/issues/11800))
  by @Findeton

- ✨ Docusaurus: Election Managers guides ([sequentech/meta#10085](https://github.com/sequentech/meta/issues/10085))
  by @GuyZequent

- ✨ Docusaurus: Information about import versioning constraints ([sequentech/meta#11114](https://github.com/sequentech/meta/issues/11114))
  by @GuyZequent, @BelSequent

- ✨ Documentation: Voter Guides Enhanced ([sequentech/meta#9998](https://github.com/sequentech/meta/issues/9998))
  by @GuyZequent

- ✨ Documentation: Structural Changes ([sequentech/meta#8672](https://github.com/sequentech/meta/issues/8672))
  by @GuyZequent, @xalsina-sequent

- 📖 [doc] Document new release flow and schedule ([sequentech/meta#10139](https://github.com/sequentech/meta/issues/10139))
  by @Findeton


### Other Changes

- Fix CLA Assistant false negatives for Copilot-attributed committers ([sequentech/meta#11927](https://github.com/sequentech/meta/issues/11927))
  by @oded-eid-sequentech

- ✨ Prepare Release 9.5 ([sequentech/step#2639](https://github.com/sequentech/step/pull/2639))
  by @Findeton

