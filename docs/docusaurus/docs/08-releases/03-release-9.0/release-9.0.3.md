---
id: release-9.0.3
title: Release Notes v9.0.3
---
<!--
SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Release 9.0.3

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

## 📋 All Changes

### 🛠 Bug Fixes


- 🐞 Ballot Verifier > Custom CSS not properly applied ([sequentech/meta#8476](https://github.com/sequentech/meta/issues/8476))
  by @yuvalkom-M


- 🐞 Admin Portal: Can't allow write-ins ([sequentech/step#2396](https://github.com/sequentech/step/pull/2396))
  by @Findeton


- 🐞 Voting Portal > Candidate images broken after export then import the event ([sequentech/meta#9087](https://github.com/sequentech/meta/issues/9087))
  by @yuvalkom-M


- 🐞 Admin Portal: Scheduled Repeatable Reports is not working ([sequentech/meta#5412](https://github.com/sequentech/meta/issues/5412))
  by @yuvalkom-M


- 🐞 Tally > Ballot Image fails on second time: duplicate ACM key ([sequentech/meta#8679](https://github.com/sequentech/meta/issues/8679))
  by @BelSequent

- 🐞 Tally: arbitrary votes instead of the last one on voter re-votes ([sequentech/step#2488](https://github.com/sequentech/step/pull/2488))
  by @Findeton


- 🐞 sequentech-bot is not in the allow list for CLA ([sequentech/step#2313](https://github.com/sequentech/step/pull/2313))
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


- 🐞 Tally > Election aliases not used ([sequentech/meta#8426](https://github.com/sequentech/meta/issues/8426))
  by @yuvalkom-M


- 🐞 Tally > Export option can't be read correctly if title is too long ([sequentech/meta#8676](https://github.com/sequentech/meta/issues/8676))
  by @yuvalkom-M


- 🐞 Tally > "No Results" while loading the results ([sequentech/meta#8677](https://github.com/sequentech/meta/issues/8677))
  by @yuvalkom-M


- 🐞 Contest result extended metrics are 0 ([sequentech/meta#8573](https://github.com/sequentech/meta/issues/8573))
  by @BelSequent


- 🐞 Fix graphql typescript issues ([sequentech/meta#9540](https://github.com/sequentech/meta/issues/9540))
  by @Findeton


- 🐞 Can't filter voter logs by username ([sequentech/meta#7751](https://github.com/sequentech/meta/issues/7751))
  by @yuvalkom-M


- 🐞 Keys Ceremony > State not cleared when switching Election Events ([sequentech/meta#8675](https://github.com/sequentech/meta/issues/8675))
  by @yuvalkom-M


- 🐞 Tally > State not cleared when switching events ([sequentech/meta#8674](https://github.com/sequentech/meta/issues/8674))
  by @yuvalkom-M


- 🐞 Username is shown after an attempted login with a valid username ([sequentech/meta#6476](https://github.com/sequentech/meta/issues/6476))
  by @Findeton



### 📖 Documentation

- ✨ Add AI Agent Documentation Structure ([sequentech/step#2417](https://github.com/sequentech/step/pull/2417))
  by @Findeton


- 📖 [doc] v9.0.2 documentation ([sequentech/meta#9156](https://github.com/sequentech/meta/issues/9156))
  by @Findeton



### Other

- ✨ Prepare Release 9.0.3 ([sequentech/step#2489](https://github.com/sequentech/step/pull/2489))
  by @Findeton


- 🐞 Admin portal - Election Event Localization Tab shows no "No results found" when localizations are present ([sequentech/step#2481](https://github.com/sequentech/step/pull/2481))
  by @xalsina-sequent


- 🐞 Admin Portal: Heterogeneus use of name/alias ([sequentech/step#2422](https://github.com/sequentech/step/pull/2422))
  by @yuvalkom-M


- 🐞 Windmill - Duplicate Key error caused by race condition while Logging Electoral logs in process_electoral_log_events_batch task ([sequentech/step#2445](https://github.com/sequentech/step/pull/2445))
  by @xalsina-sequent


- 🐞 Unneeded "Or Sign With"/simplesaml in Keycloak Login ([sequentech/step#2330](https://github.com/sequentech/step/pull/2330))
  by @yuvalkom-M


- 🐞 fix release yml output version release/9.0 ([sequentech/step#2413](https://github.com/sequentech/step/pull/2413))
  by @oded-eid-sequentech


- 🛡 Security updates ([sequentech/step#2343](https://github.com/sequentech/step/pull/2343))
  by @yuvalkom-M


- 🐞 Tally Multi Contest area votes not included in tally if area does not have the first contest assigned ([sequentech/step#2265](https://github.com/sequentech/step/pull/2265))
  by @xalsina-sequent

