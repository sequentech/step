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

## 🐞 Tally > Contests are not in order when using multi-contest encoding

Sort contest shows on tally results tabs based on the contest-order 
 field in the election presentation.

- Issue: [#8678](https://github.com/sequentech/meta/issues/8678)
