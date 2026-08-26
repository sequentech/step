---
id: release-9.4.0-rc.25
title: Release 9.4.0-rc.25
---
<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->
# Release 9.4.0-rc.25

## 📝 Highlights

### 📖 [doc] Adding a section: `Reference/Third-Party Libraries`
This issue tracks the comprehensive update of Rust toolchain, dependencies, and documentation for the Step Repository. The goal is to standardize on **Rust stable 1.90.0** across all environments (Nix, GitHub Actions, Dockerfiles) and update all Rust crates and their dependencies to their latest compatible versions.
Additionally, this includes creating developer documentation for managing Rust versions and third-party dependencies, plus implementing tooling for dependency auditing and reporting.

See [sequentech/meta#7996](https://github.com/sequentech/meta/issues/7996) for details.

## 📋 All Changes

### 🚀 Features

- ✨ Generalize `release-tool` for semver & release-flow ([sequentech/meta#9230](https://github.com/sequentech/meta/issues/9230))
  by @edulix


- ✨ Admin Portal > Tasks: add support for tenant level tasks, like creating a new tenant ([sequentech/meta#2348](https://github.com/sequentech/meta/issues/2348))
  by @yuvalkom-M


- ✨ Instant-runoff Voting (IRV/RCV) System support ([sequentech/meta#8214](https://github.com/sequentech/meta/issues/8214))
  by @BelSequent


- ✨ Delegate voting with imports ([sequentech/meta#7683](https://github.com/sequentech/meta/issues/7683))
  by @xalsina-sequent


- ✨ Don't build/push images on main ([sequentech/meta#9291](https://github.com/sequentech/meta/issues/9291))
  by @Findeton


- ✨ Publicly Open Source Preparations ([sequentech/meta#9060](https://github.com/sequentech/meta/issues/9060))
  by @edulix


- ✨ IdP-initiated SAML SSO authentication flow support ([sequentech/meta#8213](https://github.com/sequentech/meta/issues/8213))
  by @xalsina-sequent


- ✨ Move voter signature to the voting portal ([sequentech/meta#5518](https://github.com/sequentech/meta/issues/5518))
  by @xalsina-sequent


- ✨ Voting Portal > Nightwatch voting with no revotes ([sequentech/meta#8624](https://github.com/sequentech/meta/issues/8624))
  by @Findeton


- ✨ Automatic Launch of E2E tests for environments and during release process ([sequentech/meta#7004](https://github.com/sequentech/meta/issues/7004))
  by @Findeton

- ✨ Prepare Release 10.0 ([sequentech/step#2474](https://github.com/sequentech/step/pull/2474))
  by @Findeton

- ✨ Docusaurus - Tutorial on how to make calls to hasura/graphql ([sequentech/step#2429](https://github.com/sequentech/step/pull/2429))
  by @xalsina-sequent

- ✨ Move From Self-Hosted Runners using Runs-On in AWS to using GHA controller in a Google Cloud Project ([sequentech/meta#6511](https://github.com/sequentech/meta/issues/6511))
  by @oded-eid-sequentech


### 🛠 Bug Fixes

- 🐞 Ballot Verifier > Custom CSS not properly applied ([sequentech/meta#8476](https://github.com/sequentech/meta/issues/8476))
  by @yuvalkom-M


- 🐞 NullPointerException on voter login after KC24 → KC26.4 upgrade ([sequentech/step#2426](https://github.com/sequentech/step/pull/2426))
  by @Findeton


- 🐞 Can't send emails from keycloak ([sequentech/step#2353](https://github.com/sequentech/step/pull/2353))
  by @Findeton


- 🐞 Admin Portal: Can't allow write-ins ([sequentech/step#2393](https://github.com/sequentech/step/pull/2393))
  by @Findeton


- 🐞 Voting Portal > Candidate images broken after export then import the event ([sequentech/meta#9087](https://github.com/sequentech/meta/issues/9087))
  by @yuvalkom-M


- 🐞 Admin Portal: Scheduled Repeatable Reports is not working ([sequentech/meta#5412](https://github.com/sequentech/meta/issues/5412))
  by @yuvalkom-M


- 🐞 Tally > Ballot Image fails on second time: duplicate ACM key ([sequentech/meta#8679](https://github.com/sequentech/meta/issues/8679))
  by @BelSequent


- 🐞 sequentech-bot is not in the allow list for CLA ([sequentech/step#2314](https://github.com/sequentech/step/pull/2314))
  by @Findeton


- 🐞 Voting Portal > Grace Period not applied if no scheduled event ([sequentech/meta#9091](https://github.com/sequentech/meta/issues/9091))
  by @BelSequent


- 🐞 Admin Portal > Reports > Timezone shown is not showing timezone ([sequentech/meta#6191](https://github.com/sequentech/meta/issues/6191))
  by @xalsina-sequent

- 🐞 Tally > Contests are not in order when using multi-contest encoding ([sequentech/meta#8678](https://github.com/sequentech/meta/issues/8678))
  by @xalsina-sequent


- 🐞 Admin Portal > Can't send message to voters ([sequentech/meta#9721](https://github.com/sequentech/meta/issues/9721))
  by @Findeton


- 🐞 Errors editing forms ([sequentech/meta#9572](https://github.com/sequentech/meta/issues/9572))
  by @Findeton


- 🐞 Keycloak's custom event listener is not working ([sequentech/meta#9574](https://github.com/sequentech/meta/issues/9574))
  by @Findeton


- 🐞 Tally > Export option can't be read correctly if title is too long ([sequentech/meta#8676](https://github.com/sequentech/meta/issues/8676))
  by @yuvalkom-M


- 🐞 Tally > "No Results" while loading the results ([sequentech/meta#8677](https://github.com/sequentech/meta/issues/8677))
  by @yuvalkom-M


- 🐞 Tally UI shows manual and executes automatic after policy switch ([sequentech/meta#8472](https://github.com/sequentech/meta/issues/8472))
  by @yuvalkom-M


- 🐞 Contest result extended metrics are 0 ([sequentech/meta#8573](https://github.com/sequentech/meta/issues/8573))
  by @BelSequent


- 🐞 Error with tenants and templates in Admin portal. ([sequentech/meta#9539](https://github.com/sequentech/meta/issues/9539))
  by @xalsina-sequent


- 🐞 Fix graphql typescript issues ([sequentech/meta#9540](https://github.com/sequentech/meta/issues/9540))
  by @Findeton


- 🐞 Keys Ceremony > State not cleared when switching Election Events ([sequentech/meta#8675](https://github.com/sequentech/meta/issues/8675))
  by @yuvalkom-M


- 🐞 Can't filter voter logs by username ([sequentech/meta#7751](https://github.com/sequentech/meta/issues/7751))
  by @xalsina-sequent, @yuvalkom-M


- 🐞 Tally > State not cleared when switching events ([sequentech/meta#8674](https://github.com/sequentech/meta/issues/8674))
  by @yuvalkom-M


- 🐞 Username is shown after an attempted login with a valid username ([sequentech/meta#6476](https://github.com/sequentech/meta/issues/6476))
  by @yuvalkom-M


- 🐞 Fixes after dependency updates ([sequentech/meta#9132](https://github.com/sequentech/meta/issues/9132))
  by @Findeton

- 🐞 Windmill > Can't create Ballot Images on ARM ([sequentech/meta#8621](https://github.com/sequentech/meta/issues/8621))
  by @xalsina-sequent


- 🐞 Voter log errors ([sequentech/meta#8097](https://github.com/sequentech/meta/issues/8097))
  by @yuvalkom-M


- 🐞 Admin Portal > Sidebar: Can't select active events tab if all Events are archived ([sequentech/meta#8876](https://github.com/sequentech/meta/issues/8876))
  by @Findeton


- 🐞 Default Invalid vote policy mismatch ([sequentech/meta#8855](https://github.com/sequentech/meta/issues/8855))
  by @Findeton

- 🐞 Admin Portal: Heterogeneus use of name/alias ([sequentech/step#2440](https://github.com/sequentech/step/pull/2440))
  by @yuvalkom-M

- 🐞 Windmill - Duplicate Key error caused by race condition while Logging Electoral logs in process_electoral_log_events_batch task ([sequentech/step#2448](https://github.com/sequentech/step/pull/2448))
  by @xalsina-sequent


- 🐞 IRV: Default tally operation processes the candidate results in each area ([sequentech/step#2459](https://github.com/sequentech/step/pull/2459))
  by @BelSequent


- 🐞 Unneeded "Or Sign With"/simplesaml in Keycloak Login ([sequentech/step#2333](https://github.com/sequentech/step/pull/2333))
  by @yuvalkom-M


- 🐞 Voting Portal > IRV > Limit in ranking positions is not consistent with max_votes constraint ([sequentech/step#2328](https://github.com/sequentech/step/pull/2328))
  by @BelSequent


- 🐞 Admin Portal: Can't do report actions ([sequentech/step#2387](https://github.com/sequentech/step/pull/2387))
  by @yuvalkom-M


- 🐞 Tally Multi Contest area votes not included in tally if area does not have the first contest assigned ([sequentech/step#2274](https://github.com/sequentech/step/pull/2274))
  by @xalsina-sequent


- 🐞 New immudb column ballot_id is not backwards compatible ([sequentech/step#2256](https://github.com/sequentech/step/pull/2256))
  by @BelSequent


- 🐞 Multi-Tenant login doesn't work ([sequentech/step#2277](https://github.com/sequentech/step/pull/2277))
  by @Findeton


### 📖 Documentation

- 📖 Add AI Agent Documentation Structure ([sequentech/step#2417](https://github.com/sequentech/step/pull/2417))
  by @Findeton


- 📖 [doc] v9.0.2 documentation ([sequentech/meta#9156](https://github.com/sequentech/meta/issues/9156))
  by @Findeton


### 🛡 Security Updates

- 🛡 Security updates: ring ([sequentech/meta#9133](https://github.com/sequentech/meta/issues/9133))
  by @Findeton

- 🛡 Security updates ([sequentech/step#2379](https://github.com/sequentech/step/pull/2379))
  by @yuvalkom-M

## 🔄 Migrations

### 🐞 Admin Portal > Reports > Timezone shown is not showing timezone
In order to see the change the default receipt template ballot_receipt_user.hbs needs some changes.
From
```
<span class="value" class="timestamp-content">{{timestamp}}</span>
```
To
```
<span class="value" class="timestamp-content">{{datetime from_rfc3339=timestamp output_format="%B %d, %Y %H:%M GMT %:z"}}</span>
```
See [sequentech/meta#6191](https://github.com/sequentech/meta/issues/6191) for details.

### 📖 [doc] Adding a section: `Reference/Third-Party Libraries`
### For Developers
1. **Rust Version**: All developers must use Rust 1.90.0. Run `rustc --version` in `devenv shell` to verify.
2. **Dependency Updates**: After pulling this branch, run:
```bash
devenv shell
cd packages/
cargo build
```
3. **Frontend Updates**: For frontend work:
```bash
cd packages/
yarn install
```
4. **ESLint Migration**: Projects now use flat config (`eslint.config.js`). Old `.eslintrc.json` files have been removed.
5. **New Documentation**: Review the new developer documentation:
- `docs/docusaurus/docs/developers/11-Updates/updating-rust-version.md`
- `docs/docusaurus/docs/reference/third_party_deps/third_party_deps.md`
- `docs/docusaurus/docs/developers/03-Development-Environment/nvd-api-key-setup.md`

See [sequentech/meta#7996](https://github.com/sequentech/meta/issues/7996) for details.
