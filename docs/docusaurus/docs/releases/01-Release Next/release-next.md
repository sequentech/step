---
id: release-next
title: Release Notes next
---
<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

## 🐞 Default Invalid vote policy mismatch

The default Invalid Vote Policy was different in the backend and the front-end.
As a result, the UI displayed the policy as `warn-explicit-and-implicit`, even
though the actual default (when not predefined) was `allowed`.

- Issue: [#8855](https://github.com/sequentech/meta/issues/8855)
