---
id: release-8.7.2
title: Release Notes v8.7.2
---
<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

## Strategic Takeaways

This release continues Sequent’s investment in admin usability, compliance readiness, and performance at scale. Stronger controls on identity verification and communication increase trust and fraud prevention. Many of the changes signal Sequent’s maturity in serving both regulated union elections and public sector voting.

## Key enhancements

### 1. Security & Compliance

- 2FA Enhancements: Email and mobile OTP support, including configuration for max users per phone/email. Crucial for preventing fraud and vote manipulation.

- Country Restrictions for SMS: More control over where voting communications go. Useful for geo-limited elections or union bylaws.

- Email Verification Import: Supports importing *email\_verified- status, improving voter onboarding accuracy.

### 2. Admin Experience

- Added scheduled events support: Supported importing scheduled events in the JSON file, as in previous version.

- Key Ceremony Trustee Sorting: Minor UX polish, but helpful in large-scale elections with multiple stakeholders.

- Create/Edit Area Consistency: Fixes inconsistencies in the admin portal, reducing setup confusion.

- Tally Performance Boost: Faster vote counting and eligibility validation vital for real-time reporting.

### 3. Voting Experience

- Support for ‘Galego’ Language: Enhances inclusivity for Galician-speaking populations, especially important in multilingual contexts.

- Null Vote Handling Fix: Fixes an issue with “null vote” selection. Key for validity in ranked or optional elections.

## Bug fixes

- Tenant Access Page Fix: Ensures that tenant page is accessed correctly after login out.

- Ballot Receipt Download Fix: Ensures voters can access receipts critical for auditability and trust.

- Voter List Refresh Fix: Auto-refreshes after import, small but major for admin confidence and data flow.

- Table Sorting Crashes Resolved: Stability improvements during data reviews.

- Tally Duplication Bug: Eliminates duplicate entries. Avoids post-election disputes.

