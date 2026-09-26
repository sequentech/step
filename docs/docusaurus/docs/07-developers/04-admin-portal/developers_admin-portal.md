---
id: developers_admin_portal
title: Developing the Admin Portal
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->
Use the [fast feedback guide](../03-development-environment/fast-feedback.md)
for shared-source hot reload, fixture-backed screens and focused tests.

```sh
BROWSER=none yarn --cwd packages/admin-portal start
scripts/dev/step-dev test admin-portal
```

Runtime service URLs and portal settings come from `/global-settings.json`.
Webpack embeds only these public build settings from the environment or the
portal's `.env` file:

| Setting | Default | Purpose |
| --- | --- | --- |
| `PUBLIC_URL` | empty | Prefix for editor and phone-input assets |
| `MAX_DIFF_LINES` | 500 | Maximum lines shown in publication comparisons |
| `SECONDS_TO_SHOW_COUNTDOWN` | 60 | Election-event countdown threshold |
| `SECONDS_TO_SHOW_ALERT` | 180 | Election-event alert threshold |

Numeric defaults are applied by the consuming UI when a setting is absent or
empty. Webpack mode supplies `NODE_ENV`. Other build-process variables are not
embedded in the browser bundle. Restart the dev server after changing build
settings; source edits reload automatically.
