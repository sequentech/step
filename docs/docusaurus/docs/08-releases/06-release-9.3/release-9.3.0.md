---
id: release-v9.3.0
title: Release v9.3.0
---
<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->
# Release v9.3.0

## 🔄 Migrations
### Keycloak upgrade from 24 to 26.4
Following the new keycloak version 26.4 presented in v9.3.0, the healthcheck probes moved to different ports. Changes:
- Variable `KC_HOSTNAME` now needs `https://` at the beginning and `/auth` at the end
- Added `"--http-enabled=true", "--health-enabled=true"`, to the start command
- For nginx implementation, added:
    ``` 
        proxy-buffer-size: "128k"
        backend-protocol: HTTP
        ssl-passthrough: "false"
    ```
- `Live` and `ready` probes are now available only in port 9000, and with scheme HTTPS


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

## 📝 Highlights

### 📖 [doc] Adding a section: `Reference/Third-Party Libraries`
This issue tracks the comprehensive update of Rust toolchain, dependencies, and documentation for the Step Repository. The goal is to standardize on **Rust stable 1.90.0** across all environments (Nix, GitHub Actions, Dockerfiles) and update all Rust crates and their dependencies to their latest compatible versions.
Additionally, this includes creating developer documentation for managing Rust versions and third-party dependencies, plus implementing tooling for dependency auditing and reporting.
### Main PRs
- https://github.com/sequentech/step/pull/1988
- https://github.com/sequentech/step/pull/2150
- https://github.com/sequentech/step/pull/2153
### Stable PRs
- [x] Mark if not applicable
See [sequentech/meta#7996](https://github.com/sequentech/meta/issues/7996) for details.

## 📋 All Changes

### 🚀 Features

- ✨ Publicly Open Source Preparations ([sequentech/meta#9060](https://github.com/sequentech/meta/issues/9060))
  by @edulix


- ✨ IdP-initiated SAML SSO authentication flow support ([sequentech/meta#8213](https://github.com/sequentech/meta/issues/8213))
  by @xalsina-sequent


- ✨ Move voter signature to the voting portal ([sequentech/meta#5518](https://github.com/sequentech/meta/issues/5518))
  by @xalsina-sequent


- ✨ Voting Portal > Nightwatch voting with no revotes ([sequentech/meta#8624](https://github.com/sequentech/meta/issues/8624))
  by @Findeton


- ✨ Voting Portal > Logs: Show Message column, to ensure signature shown ([sequentech/meta#7669](https://github.com/sequentech/meta/issues/7669))
  by @BelSequent


- ✨ Videoconference links from Admin Portal ([sequentech/meta#8189](https://github.com/sequentech/meta/issues/8189))
  by @BelSequent



### 🛠 Bug Fixes

- 🐞 Error loading activity logs ([sequentech/step#2317](https://github.com/sequentech/step/pull/2317))
  by @Findeton


- 🐞 Voting Portal > Grace Period not applied if no scheduled event ([sequentech/meta#9091](https://github.com/sequentech/meta/issues/9091))
  by @BelSequent


- 🐞 Admin Portal > Reports > Timezone shown is not showing timezone ([sequentech/meta#6191](https://github.com/sequentech/meta/issues/6191))
  by @yuvalkom-M

  **Migration:** In order to see the change the default receipt template ballot_receipt_user.hbs needs some changes.
From
```
<span class="value" class="timestamp-content">{{timestamp}}</span>
```
To
```
<span class="value" class="timestamp-content">{{datetime from_rfc3339=timestamp output_format="%B %d, %Y %H:%M GMT %:z"}}</span>
```

- 🐞 Tally > Contests are not in order when using multi-contest encoding ([sequentech/meta#8678](https://github.com/sequentech/meta/issues/8678))
  by @yuvalkom-M


- 🐞 Admin Portal > Can't send message to voters ([sequentech/meta#9721](https://github.com/sequentech/meta/issues/9721))
  by @Findeton


- 🐞 Tally > Election aliases not used ([sequentech/meta#8426](https://github.com/sequentech/meta/issues/8426))
  by @yuvalkom-M


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


- 🐞 Can't filter voter logs by username ([sequentech/meta#7751](https://github.com/sequentech/meta/issues/7751))
  by @yuvalkom-M


- 🐞 Keys Ceremony > State not cleared when switching Election Events ([sequentech/meta#8675](https://github.com/sequentech/meta/issues/8675))
  by @yuvalkom-M


- 🐞 Fixes after dependency updates ([sequentech/meta#9132](https://github.com/sequentech/meta/issues/9132))
  by @Findeton


- 📖 [doc] Adding a section: `Reference/Third-Party Libraries` ([sequentech/meta#7996](https://github.com/sequentech/meta/issues/7996))
  by @Findeton, @edulix

  This issue tracks the comprehensive update of Rust toolchain, dependencies, and documentation for the Step Repository. The goal is to standardize on **Rust stable 1.90.0** across all environments (Nix, GitHub Actions, Dockerfiles) and update all Rust crates and their dependencies to their latest compatible versions.
Additionally, this includes creating developer documentation for managing Rust versions and third-party dependencies, plus implementing tooling for dependency auditing and reporting.
### Main PRs
- https://github.com/sequentech/step/pull/1988
- https://github.com/sequentech/step/pull/2150
- https://github.com/sequentech/step/pull/2153
### Stable PRs
- [x] Mark if not applicable

  **Migration:** ### For Developers
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

- 🐞 Tally > State not cleared when switching events ([sequentech/meta#8674](https://github.com/sequentech/meta/issues/8674))
  by @yuvalkom-M


- 🐞 Username is shown after an attempted login with a valid username ([sequentech/meta#6476](https://github.com/sequentech/meta/issues/6476))
  by @Findeton


- 🐞 Windmill > Can't create Ballot Images on ARM ([sequentech/meta#8621](https://github.com/sequentech/meta/issues/8621))
  by @xalsina-sequent


- 🐞 Voter log errors ([sequentech/meta#8097](https://github.com/sequentech/meta/issues/8097))
  by @yuvalkom-M


- 🐞 Admin Portal > Sidebar: Can't select active events tab if all Events are archived ([sequentech/meta#8876](https://github.com/sequentech/meta/issues/8876))
  by @Findeton


- 🐞 Default Invalid vote policy mismatch ([sequentech/meta#8855](https://github.com/sequentech/meta/issues/8855))
  by @Findeton


- 🐞 Can't see Election Lists ([sequentech/meta#8735](https://github.com/sequentech/meta/issues/8735))
  by @Findeton


- 🐞 Admin Portal > "Something went wrong" error when switching between diferent elections/questions ([sequentech/meta#8725](https://github.com/sequentech/meta/issues/8725))
  by @yuvalkom-M


- 🐞 Failed scheduled event ([sequentech/meta#8681](https://github.com/sequentech/meta/issues/8681))
  by @Findeton



### 📖 Documentation

- 📖 [doc] v9.0.2 documentation ([sequentech/meta#9156](https://github.com/sequentech/meta/issues/9156))
  by @Findeton



### 🛡 Security Updates

- 🛡 Security updates: ring ([sequentech/meta#9133](https://github.com/sequentech/meta/issues/9133))
  by @Findeton



### Other Changes

- ✨ Investigating costs increase in infra cluster (GHA) ([sequentech/meta#9293](https://github.com/sequentech/meta/issues/9293))
  by @oded-eid-sequentech


- 🐞 New immudb column ballot_id is not backwards compatible ([sequentech/step#2259](https://github.com/sequentech/step/pull/2259))
  by @BelSequent


- 🐞 Multi-Tenant login doesn't work ([sequentech/step#2276](https://github.com/sequentech/step/pull/2276))
  by @Findeton


