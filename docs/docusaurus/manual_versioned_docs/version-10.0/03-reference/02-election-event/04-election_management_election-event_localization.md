---
id: election_management_election_event_localization
title: Localization
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

This section allows you to override portal text per language. Election-event overrides can
target the Voting Portal, Ballot Verifier, Results Portal, or every public portal.

### Steps

- **Select Language**  
  Choose the language for which the localization will be applied.

- **Add**
  - **Portal scope**: Select the portal that should receive the override. Select **Global**
    only when the same key and wording should be used by every public portal.
  - **Key**: Specify the translation key to customize, without a portal prefix.
  - **Value**: Enter the text to be displayed for the selected element.

### Actions

- **Edit** a Localization element or change its portal scope.
- **Delete** a Localization element.

### Presentation fields

Portal scopes also apply to translated presentation fields such as an election-event, election,
contest, or candidate name, alias, and description. Each application automatically uses its own
scope. For example, the Voting Portal reads `votingPortal:description`, while the Ballot Verifier
reads `ballotVerifier:description`.

For the selected language, presentation fields are resolved in this order:

1. The current portal's value, such as `votingPortal:description`.
2. The value for all portals, such as `global:description`.
3. The unprefixed presentation value, such as `description`.
4. The legacy database field, when present, for compatibility with older election events.

If no value exists in the active language, the same order is applied to the configured default
language before using the legacy field.

### Existing overrides

Overrides created before portal scopes were introduced are shown as **Legacy (Voting portal)**.
They continue to affect only the Voting Portal, exactly as before. Editing and saving a legacy
override promotes it to the explicit scope selected in the editor; no migration is required.

At the storage/API level, explicit scopes use a colon prefix, for example
`resultsPortal:resultsPortal.pageTitle`. The Admin Portal adds this prefix automatically. Known
prefixes are `global`, `votingPortal`, `ballotVerifier`, `resultsPortal`, and `adminPortal`.
Unprefixed election-event keys remain Voting Portal-only for backward compatibility. The Ballot
Verifier and Results Portal ignore unprefixed keys.

Results Portal overrides are part of the published results snapshot. Republish the results after
changing an override for it to appear on the published site.
