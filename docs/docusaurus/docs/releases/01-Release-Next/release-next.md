---
id: release-next
title: Release Notes next
---
<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

## ✨ Delegate voting with imports

Implemented a simple version of delegate voting. Voters can delegate their vote 
on other voters, and initially the state of delegation will be imported. 
Then at the tally level if a voter is a delegate with N weight, we'll insert the 
ballot N times.

- Issue: [#7683](https://github.com/sequentech/meta/issues/7683)
