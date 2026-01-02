---
id: release-9.4.0-rc.1
title: Release 9.4.0-rc.1
---
<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->
# Release 9.4.0-rc.1

## 📋 All Changes

### 🚀 Features

- 🐞 Instant-runoff Voting (IRV/RCV) Test fix ([sequentech/step#2191](https://github.com/sequentech/step/pull/2191))
  by @BelSequent

- ✨ Don't build/push images on main ([sequentech/step#2184](https://github.com/sequentech/step/pull/2184))
  by @Findeton

- ✨ Voting Portal > Nightwatch voting with no revotes ([sequentech/step#2125](https://github.com/sequentech/step/pull/2125))
  by @Findeton

- ✨ Automatic Launch of E2E tests for environments and during release process ([sequentech/step#2121](https://github.com/sequentech/step/pull/2121))
  by @Findeton


### 🛠 Bug Fixes

- 🐞 Admin Portal > Can't send message to voters ([sequentech/step#2247](https://github.com/sequentech/step/pull/2247))
  by @Findeton

- 🐞 Errors editing forms ([sequentech/step#2244](https://github.com/sequentech/step/pull/2244))
  by @Findeton

- 🐞 Keycloak's custom event listener is not working ([sequentech/step#2236](https://github.com/sequentech/step/pull/2236))
  by @Findeton

- 🐞 Fix graphql typescript issues ([sequentech/step#2223](https://github.com/sequentech/step/pull/2223))
  by @Findeton

- 🐞 Fixes after dependency updates ([sequentech/step#2171](https://github.com/sequentech/step/pull/2171))
  by @Findeton

- 📖 [doc] Adding a section: Reference/Third-Party Libraries ([sequentech/step#2153](https://github.com/sequentech/step/pull/2153))
  by @Findeton, @edulix

- 🐞 Admin Portal > Sidebar: Can't select active events tab if all Events are archived ([sequentech/step#2132](https://github.com/sequentech/step/pull/2132))
  by @Findeton

- 🐞 Default Invalid vote policy mismatch ([sequentech/step#2126](https://github.com/sequentech/step/pull/2126))
  by @Findeton

- 🐞 Can't see Election Lists ([sequentech/step#2104](https://github.com/sequentech/step/pull/2104))
  by @Findeton

- 🐞 Failed scheduled event ([sequentech/step#2089](https://github.com/sequentech/step/pull/2089))
  by @Findeton


### 📖 Documentation

- ✨ Prepare Release 9.3 ([sequentech/step#2185](https://github.com/sequentech/step/pull/2185))
  by @Findeton

- ✨ Prepare Release 9.2.0 ([sequentech/step#2169](https://github.com/sequentech/step/pull/2169))
  by @Findeton

- 📖 [doc] v9.0.2 documentation ([sequentech/step#2167](https://github.com/sequentech/step/pull/2167))
  by @Findeton

- ✨ Prepare Release v9.0.1 ([sequentech/step#2114](https://github.com/sequentech/step/pull/2114))
  by @Findeton


### Other Changes

- ✨ Generalize `release-tool` for semver & release-flow: using release-bot ([sequentech/step#2302](https://github.com/sequentech/step/pull/2302))
  by @edulix

- 🐞 Voting Portal > Grace Period not applied if no scheduled event ([sequentech/step#2283](https://github.com/sequentech/step/pull/2283))
  by @BelSequent

- 🐞 New immudb column ballot_id is not backwards compatible ([sequentech/step#2256](https://github.com/sequentech/step/pull/2256))
  by @BelSequent

- 🐞 Admin Portal > Reports > Timezone shown is not showing timezone ([sequentech/step#2291](https://github.com/sequentech/step/pull/2291))
  by @xalsina-sequent

- 🐞 Tally > Contests are not in order when using multi-contest encoding ([sequentech/step#2290](https://github.com/sequentech/step/pull/2290))
  by @xalsina-sequent

- 🐞 Multi-Tenant login doesn't work ([sequentech/step#2277](https://github.com/sequentech/step/pull/2277))
  by @Findeton

- 🐞 Tally > Export option can't be read correctly if title is too long ([sequentech/step#2110](https://github.com/sequentech/step/pull/2110))
  by @yuvalkom-M

- 🐞 Tally > "No Results" while loading the results ([sequentech/step#2096](https://github.com/sequentech/step/pull/2096))
  by @yuvalkom-M

- 🐞 Tally UI shows manual and executes automatic after policy switch ([sequentech/step#2042](https://github.com/sequentech/step/pull/2042))
  by @yuvalkom-M

- 🐞 Contest result extended metrics are 0 ([sequentech/step#2057](https://github.com/sequentech/step/pull/2057))
  by @BelSequent

- 🐞 Error with tenants and templates in Admin portal. ([sequentech/step#2221](https://github.com/sequentech/step/pull/2221))
  by @xalsina-sequent

- 🐞 Keys Ceremony > State not cleared when switching Election Events ([sequentech/step#2112](https://github.com/sequentech/step/pull/2112))
  by @yuvalkom-M

- 🐞 Can't filter voter logs by username (Add release notes) ([sequentech/step#2220](https://github.com/sequentech/step/pull/2220))
  by @xalsina-sequent, @yuvalkom-M

- ✨ Admin Portal > Tasks: add support for tenant level tasks, like creating a new tenant ([sequentech/step#1831](https://github.com/sequentech/step/pull/1831))
  by @yuvalkom-M

- ✨ Delegate voting with imports ([sequentech/step#2141](https://github.com/sequentech/step/pull/2141))
  by @xalsina-sequent

- 🐞 Tally > State not cleared when switching events ([sequentech/step#2109](https://github.com/sequentech/step/pull/2109))
  by @yuvalkom-M

- 🐞 Username is shown after an attempted login with a valid username ([sequentech/step#2037](https://github.com/sequentech/step/pull/2037))
  by @yuvalkom-M

- 🛡 Security updates: ring ([sequentech/step#2158](https://github.com/sequentech/step/pull/2158))
  by @Findeton

- Use arc-runner-set for Java test workflow ([sequentech/step#2151](https://github.com/sequentech/step/pull/2151))
  by @oded-eid-sequentech

- ✨ Publicly Open Source Preparations: SAML SSO fixes ([sequentech/step#2165](https://github.com/sequentech/step/pull/2165))
  by @edulix

- ✨ IdP-initiated SAML SSO authentication flow support ([sequentech/step#2083](https://github.com/sequentech/step/pull/2083))
  by @xalsina-sequent

- ✨ Move voter signature to the voting portal ([sequentech/step#1969](https://github.com/sequentech/step/pull/1969))
  by @xalsina-sequent

- 🐞 Windmill > Can't create Ballot Images on ARM ([sequentech/step#2144](https://github.com/sequentech/step/pull/2144))
  by @xalsina-sequent

- 🐞 Voter log errors ([sequentech/step#2039](https://github.com/sequentech/step/pull/2039))
  by @yuvalkom-M

- ✨ Voting Portal > Logs: Show Message column, to ensure signature shown ([sequentech/step#1964](https://github.com/sequentech/step/pull/1964))
  by @BelSequent

- 🐞 Admin Portal > "Something went wrong" error when switching between diferent elections/questions ([sequentech/step#2097](https://github.com/sequentech/step/pull/2097))
  by @yuvalkom-M

- ✨ Videoconference links from Admin Portal ([sequentech/step#2024](https://github.com/sequentech/step/pull/2024))
  by @BelSequent

