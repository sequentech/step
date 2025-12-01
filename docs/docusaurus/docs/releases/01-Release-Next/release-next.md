---
id: release-next
title: Release Notes next
---
<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->


## ✨ Instant-runoff Voting (IRV/RCV) System support

Support for Instant-runoff elections. Adaptations and implementations were added:
- Velvet tally and make the tally operation configurable.
- Admin portal, selectable counting algorithm at contest level, UI tally results and results report.
- Voting portal and ballot verifier for preferential order.
- Ballot codec for preferential order.
- Layout documents in docusaurus of the Velvet crate for developers.
- Description on how to configure the tally operation in annotations.
- Document in election management at contest level.

- Issue: [#8214](https://github.com/sequentech/meta/issues/8214)

## ✨ Delegate voting with imports

Implemented a simple version of delegate voting. Voters can delegate their vote 
on other voters, and initially the state of delegation will be imported. 
Then at the tally level if a voter is a delegate with N weight, we'll insert the 
ballot N times.

- Issue: [#7683](https://github.com/sequentech/meta/issues/7683)
