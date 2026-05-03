---
id: admin_portal_tally_results
title: Tally Results
description: Developer notes for the Admin Portal tally results UI, including the Participation Summary widget and its CSS selectors.
---

<!--
SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

## Overview

The Admin Portal **Tally Results** view presents the finalized election results after a tally
session has completed. It combines summary charts and tables with candidate-level results so
administrators and integrators can review the outcome and apply supported UI customizations.

## Tally Results layout

The results screen is organized into widgets. In the example below, the **Participation Summary**
widget appears above **Candidate Results** and next to the participation chart.

![Participation Summary widget inside the Admin Portal Tally Results screen](https://github.com/user-attachments/assets/86e5d174-be07-435b-9169-2c237af54b1f)

## Participation Summary widget

The **Participation Summary** widget is the table on the right side of the results screen. It
shows the main turnout and ballot classification metrics for the currently selected tally scope
(global or area results).

### Available row selectors

Each row in the widget exposes a stable row-level CSS class so custom Admin Portal styling can
target complete rows instead of individual cells.

- `.participation-summary-row` — shared class added to all rows in the widget
- `.eligible-voters`
- `.total-auditable-votes`
- `.total-votes-counted`
- `.total-valid-votes`
- `.total-invalid-votes`
- `.explicitly-invalid-votes`
- `.implicitly-invalid-votes`
- `.blank-votes`
- `.weight` — rendered only when weighted voting is shown for area results

### Example customization

```css
.total-valid-votes td,
.total-valid-votes th {
    font-weight: 700;
}
```

### Notes

- These selectors are applied to the table row (`<tr>`) for each metric.
- The shared `.participation-summary-row` class can be used for styles common to the whole widget.
- The metric-specific classes are intended to stay stable so integrators can safely reference them
  from custom CSS overrides.
