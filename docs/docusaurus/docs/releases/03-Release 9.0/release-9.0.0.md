---
id: release-9.0.0
title: Release Notes v9.0.0
---
<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

## Strategic takeaways

This upgrade strengthens security, improves admin workflows, and boosts performance for large-scale elections. It fixes several issues in the Admin and Voting portals, provides clearer admin voting guidance, and speeds up vote processing—simplifying event setup.

## Key enhancements

### 1. Security & compliance
- Improved Voter Action Logging: enhanced entries in the electoral log for better auditability and tracking.

### 2. Performance & reliability
- Faster vote counting: optimized processing logic significantly improves end-to-end tally speed.
- Lower memory usage: reduced consumption, especially for large elections with multiple concurrent elections/contests.

### 3. Admin experience
- Improved Demo Mode: easy access from the Admin Portal’s “Publish” tab via the “Preview” button.

## Bug fixes

- Fixed an issue where manually created (non-imported) elections failed to cast votes after clicking “Cast your Ballot” on the Review Ballot step.
- Fixed voter login error after login when using tenants other than the default.
- Fixed empty Tally results download menu for Contest and Areas when using the “Actions” menu.
- Fixed redirect: accessing a tenant URL after logging out now correctly returns to the tenant selection page.
- Fixed intermittent errors when loading preview after login in the Voting Portal.
- Added missing Election Name on Admin Portal Tally under the “Results & Participation” menu.