---
id: developers_admin_portal
title: Developing the Admin Portal
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->




## Participation Summary CSS selectors

The Participation Summary table in the tally results view exposes stable row-level CSS classes so
custom Admin Portal styling can target complete rows instead of individual cells.

- `.participation-summary-row` — shared class applied to every Participation Summary row
- `.eligible-voters`
- `.total-auditable-votes`
- `.total-votes-counted`
- `.total-valid-votes`
- `.total-invalid-votes`
- `.explicitly-invalid-votes`
- `.implicitly-invalid-votes`
- `.blank-votes`
- `.weight` — only rendered when weighted voting is shown for area results

Example:

```css
.total-valid-votes td,
.total-valid-votes th {
    font-weight: 700;
}
```
