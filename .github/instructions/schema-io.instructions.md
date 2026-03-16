---
applyTo: "hasura/migrations/**/*.sql,hasura/metadata/**/*.yaml,packages/windmill/src/services/import/**/*.rs,packages/windmill/src/services/export/**/*.rs,packages/sequent-core/**/*.rs,packages/admin-portal/src/types/**/*.ts,packages/voting-portal/src/types/**/*.ts"
---
<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Schema Import Export Review

- Review schema, migration, import, and export changes for backwards compatibility with existing election events and generated GraphQL types.
- New fields should usually be optional or have safe defaults. Flag changes that make old data, fixtures, or previously exported packages unreadable.
- Hasura metadata, migrations, and generated types must stay in sync. Flag manual schema drift and missing regeneration steps.
- Changes to CSV, JSON, or report export formats must preserve stable identifiers, deterministic ordering where required, and documented semantics.
- For tally and results import or export, verify the same latest-ballot and multi-area semantics used by the backend counting paths.
- For migrations, look for irreversible or unsafe transformations, silent truncation, and changes that can break tenant isolation or permission checks.
- Expect tests or fixtures that cover old event data, invalid input, missing optional fields, and round trips through export then import.