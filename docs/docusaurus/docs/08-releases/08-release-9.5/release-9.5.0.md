---
id: release-9.5.0
title: Release 9.5.0
---
<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->
# Release 9.5.0

## 📝 Highlights

### 🐞 Windmill and Harvest fail to start on production deployments with AWS S3
In AWS deployments, `windmill` and `harvest` can fail during startup while initializing the plugin manager. The failure happens when the backend tries to list plugin objects under `public/plugins/` using the current S3 abstraction, where:
- `AWS_S3_PRIVATE_URI` and `AWS_S3_PUBLIC_URI` point to a real AWS bucket URL.
- `AWS_S3_BUCKET` and `AWS_S3_PUBLIC_BUCKET` are treated as logical prefixes like `election-event-documents` and `public`.
This works against MinIO, but it breaks on AWS because `list_objects_v2` must address the real bucket and use the logical bucket as part of the prefix.
See [sequentech/meta#11570](https://github.com/sequentech/meta/issues/11570) for details.

## 📋 All Changes

### 🚀 Features

- ✨ Keycloak - Map multiple IdP users to a single user via custom multi-value attribute ([sequentech/step#2633](https://github.com/sequentech/step/pull/2633))
  by @Findeton

- ✨ Voting Portal: Expand/Collapse all lists ([sequentech/meta#11765](https://github.com/sequentech/meta/issues/11765))
  by @xalsina-sequent

- ✨ Policy to disable browser language detection and force the configured default language ([sequentech/meta#11799](https://github.com/sequentech/meta/issues/11799))
  by @Findeton

- ✨ Keycloak: Configuration to support multiple certificates in Election Event ([sequentech/meta#11110](https://github.com/sequentech/meta/issues/11110))
  by @BelSequent, @Findeton

- ✨ Update all Envs to use 2FA/Passkeys ([sequentech/meta#10975](https://github.com/sequentech/meta/issues/10975))
  by @Findeton

- ✨ IRV: Support external hat-procedure for ties ([sequentech/meta#10596](https://github.com/sequentech/meta/issues/10596))
  by @xalsina-sequent

- ✨ IRV: Implicit Invalid vote handling policies ([sequentech/meta#10597](https://github.com/sequentech/meta/issues/10597))
  by @BelSequent

- ✨ Velvet: Single PDF with Election results for all Areas ([sequentech/meta#10595](https://github.com/sequentech/meta/issues/10595))
  by @yuvalkom-M

- ✨ IRV Voting Experience - Display ([sequentech/meta#10594](https://github.com/sequentech/meta/issues/10594))
  by @BelSequent

- ✨ API for preview election Event ([sequentech/meta#9991](https://github.com/sequentech/meta/issues/9991))
  by @yuvalkom-M

- ✨ Add versioning to election event configs ([sequentech/meta#6333](https://github.com/sequentech/meta/issues/6333))
  by @omri81

- ✨ CSS per election: election alias ([sequentech/meta#9992](https://github.com/sequentech/meta/issues/9992))
  by @yuvalkom-M

- ✨ Implement happy path for CLI ([sequentech/meta#6680](https://github.com/sequentech/meta/issues/6680))
  by @yuvalkom-M, @omri81

- ✨ Step-CLI Generate voters - Allow email_verified and authorized_election_count ([sequentech/meta#7254](https://github.com/sequentech/meta/issues/7254))
  by @xalsina-sequent


### 🛠 Bug Fixes

- 🐞 Step-Cli not compiling ([sequentech/step#2647](https://github.com/sequentech/step/pull/2647))
  by @Findeton

- 🐞 Windmill and Harvest fail to start on production deployments with AWS S3 ([sequentech/meta#11570](https://github.com/sequentech/meta/issues/11570))
  by @Findeton

- 🐞 Windmill: Github action keeps failing ([sequentech/meta#10956](https://github.com/sequentech/meta/issues/10956))
  by @oded-eid-sequentech, @Findeton

- 🐞 Broken sequent-core WASM package ([sequentech/meta#11089](https://github.com/sequentech/meta/issues/11089))
  by @Findeton

- 🐞 Admin Portal > Electoral Logs >Timestamp filter does not work ([sequentech/meta#9995](https://github.com/sequentech/meta/issues/9995))
  by @BelSequent


### Other Changes

- 🐞 Voting portal - Event localization override not working for alerts and errors if ballot styles are missing ([sequentech/step#2618](https://github.com/sequentech/step/pull/2618))
  by @xalsina-sequent

- ✨ Add OpenID4VP Wallet Extension Plugin to Keycloak image ([sequentech/step#2615](https://github.com/sequentech/step/pull/2615))
  by @BelSequent

- ✨ Prepare Release 9.5 ([sequentech/step#2639](https://github.com/sequentech/step/pull/2639))
  by @Findeton

- ✨ Keycloak - Add Catalan translations for OTP ([sequentech/step#2591](https://github.com/sequentech/step/pull/2591))
  by @xalsina-sequent

