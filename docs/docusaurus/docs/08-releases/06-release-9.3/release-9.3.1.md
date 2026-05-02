---
id: release-v9.3.1
title: Release v9.3.1
---
<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->
# Release v9.3.1

## 📝 Highlights

### 🛡 Security updates
[Address dependabot security alerts in step repo.](https://github.com/sequentech/step/security/dependabot)
For all maintained branches.
See [sequentech/meta#10222](https://github.com/sequentech/meta/issues/10222) for details.

## 📋 All Changes

### 💥 Breaking Changes

- 🐞 Admin Portal: Heterogeneus use of name/alias ([sequentech/meta#10552](https://github.com/sequentech/meta/issues/10552))
  by @BelSequent, @yuvalkom-M


### 🚀 Features

- ✨ Update all Envs to use 2FA/Passkeys ([sequentech/meta#10975](https://github.com/sequentech/meta/issues/10975))
  by @Findeton

- ✨ Docusaurus - Tutorial on how to make calls to hasura/graphql ([sequentech/meta#10959](https://github.com/sequentech/meta/issues/10959))
  by @xalsina-sequent

- ✨ Add AI Agent Documentation Structure ([sequentech/meta#10577](https://github.com/sequentech/meta/issues/10577))
  by @Findeton


### 🛠 Bug Fixes

- 🐞 Admin Portal > Electoral Logs >Timestamp filter does not work ([sequentech/meta#9995](https://github.com/sequentech/meta/issues/9995))
  by @BelSequent

- 🐞 Admin Portal: Voter Date input loses focus while typing ([sequentech/meta#11804](https://github.com/sequentech/meta/issues/11804))
  by @Findeton

- 🐞 Windmill: SQL parameters sanitization ([sequentech/meta#11608](https://github.com/sequentech/meta/issues/11608))
  by @Findeton

- 🐞 Voting Portal: Ballot Locator redirect not working ([sequentech/meta#11699](https://github.com/sequentech/meta/issues/11699))
  by @Findeton

- 🐞 Voting portal: Can't login on kiosk channel ([sequentech/meta#11638](https://github.com/sequentech/meta/issues/11638))
  by @Findeton

- 🐞 Admin Portal > Electoral Logs > Export CSV issues ([sequentech/meta#10960](https://github.com/sequentech/meta/issues/10960))
  by @BelSequent

- 🐞 Admin Portal: Sidebar tree does not refresh after creating an election event ([sequentech/meta#11587](https://github.com/sequentech/meta/issues/11587))
  by @Findeton

- 🐞 Tally: arbitrary votes instead of the last one on voter re-votes ([sequentech/meta#11342](https://github.com/sequentech/meta/issues/11342))
  by @xalsina-sequent

- 🐞 Admin portal - Election Event Localization Tab shows no "No results found" when localizations are present ([sequentech/meta#11128](https://github.com/sequentech/meta/issues/11128))
  by @xalsina-sequent

- 🐞 Admin Portal: Ballot images PDF missing contests beyond the second one ([sequentech/meta#11451](https://github.com/sequentech/meta/issues/11451))
  by @Findeton

- 🐞 Windmill - Duplicate Key error caused by race condition while Logging Electoral logs in process_electoral_log_events_batch task ([sequentech/meta#11108](https://github.com/sequentech/meta/issues/11108))
  by @xalsina-sequent

- 🐞 Ballot Verifier > Custom CSS not properly applied ([sequentech/meta#8476](https://github.com/sequentech/meta/issues/8476))
  by @yuvalkom-M

- 🐞 Unneeded "Or Sign With"/simplesaml in Keycloak Login ([sequentech/meta#10141](https://github.com/sequentech/meta/issues/10141))
  by @yuvalkom-M

- 🐞 NullPointerException on voter login after KC24 → KC26.4 upgrade ([sequentech/meta#10972](https://github.com/sequentech/meta/issues/10972))
  by @Findeton

- 🐞 Can't send emails from keycloak ([sequentech/meta#10143](https://github.com/sequentech/meta/issues/10143))
  by @Findeton

- 🐞 Admin Portal: Can't allow write-ins ([sequentech/meta#10631](https://github.com/sequentech/meta/issues/10631))
  by @Findeton

- 🐞 Admin Portal: Can't do report actions ([sequentech/meta#10549](https://github.com/sequentech/meta/issues/10549))
  by @yuvalkom-M

- 🐞 Voting Portal > Candidate images broken after export then import the event ([sequentech/meta#9087](https://github.com/sequentech/meta/issues/9087))
  by @yuvalkom-M

- 🐞 Admin Portal: Scheduled Repeatable Reports is not working ([sequentech/meta#5412](https://github.com/sequentech/meta/issues/5412))
  by @yuvalkom-M

- 🐞 Tally > Ballot Image fails on second time: duplicate ACM key ([sequentech/meta#8679](https://github.com/sequentech/meta/issues/8679))
  by @BelSequent


### 🛡 Security Updates

- 🛡 Security updates ([sequentech/meta#10222](https://github.com/sequentech/meta/issues/10222))
  by @yuvalkom-M


### Other Changes

- ✨ Sunday Deployment - 8-Feb-2026 ([sequentech/meta#10603](https://github.com/sequentech/meta/issues/10603))
  by @oded-eid-sequentech

