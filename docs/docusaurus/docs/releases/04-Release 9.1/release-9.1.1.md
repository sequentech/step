---
id: release-9.1.1
title: Release Notes 9.1.1
---
<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

## 🐞 Further translation issues

In the Voting Portal, the lang HTML tag is set to English/en and it doesn't change even when changing the
language. This fixes the issue, which was triggering unwanted automatic translations, for example
translating to German pages that were already in German.

- Issue: [#8470](https://github.com/sequentech/meta/issues/8470)