---
id: voting_portal_custom_css
title: Voting Portal CSS hooks
sidebar_position: 9
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

Use the stable classes below in **Data → Ballot Design → Custom CSS**. Prefer a
screen or component class over generated Emotion classes, translated text, element
positions, or an assumed HTML tag. A navigation action may be a link styled as a
button. Existing classes remain supported; the hooks below identify logical
components, not every layout element.

## Screens and shared structure

| Screen         | Root selector                                                          |
| -------------- | ---------------------------------------------------------------------- |
| Ballot List    | `.election-selection-screen`                                           |
| Instructions   | `.start-screen`                                                        |
| Vote           | `.voting-screen`                                                       |
| Review         | `.review-screen` (unavailable-ballot fallback: `.review-error-screen`) |
| Confirmation   | `.confirmation-screen`                                                 |
| Ballot Locator | `.ballot-locator-screen`                                               |

The six screen roots retain `.screen`. Within the applicable screen, use
`.screen-title`, `.screen-description`, `.stepper-box`, `.screen-help-button` and
`.actions-container`. The Instructions screen has no page-help button. Optional
sections only render when enabled by the election configuration.

| Area                 | Supported hooks                                                                                                                                                                          |
| -------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Header               | `.header-class`, `.header-logo-link`, `.header-logo`, `.header-actions`, `.app-version` (version and hash)                                                                               |
| Language menu        | `.language-selector`, `.language-selector-button`, `.language-selector-menu`, `.language-option`                                                                                         |
| Profile menu         | `.profile-menu-container`, `.profile-menu-button`, `.profile-menu`, `.user-details`, `.profile`, `.logout-button`                                                                        |
| Footer               | `.footer-class`, `.footer-link`                                                                                                                                                          |
| Progress             | `.step-container`, `.step-number`, `.step-separator`, `.selected`, `.not-selected` (scope these to `.step-container`)                                                                    |
| Dialog structure     | `.dialog`, `.dialog-title`, `.dialog-title-text`, `.dialog-content`, `.dialog-error`, `.dialog-actions`, `.dialog-close-button`, `.dialog-expand-button`, `.ok-button`, `.cancel-button` |
| Shared icon controls | `.icon-button`; feature-specific button classes below target the control, not its SVG                                                                                                    |

Dialogs and menus render in portals outside the screen root. Target their own
classes directly, not as descendants of a screen. Page-help dialogs have
`.screen-help-dialog` plus one of `.election-selection-help-dialog`,
`.voting-help-dialog`, `.review-help-dialog`, `.confirmation-help-dialog` or
`.ballot-locator-help-dialog`. Other dialogs include `.demo-dialog`,
`.decline-to-vote-dialog`, `.ballot-validation-dialog`, `.audit-ballot-dialog`,
`.confirm-cast-ballot-dialog`, `.review-ballot-id-help-dialog`, `.logout-dialog`
and `.session-expiry-dialog`.

## Screen-specific components

| Area                         | Supported hooks                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| ---------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Ballot cards                 | `.elections-list`, `.election-list-item` (complete card), `.election-item`, `.election-info`, `.election-title`, `.election-vote-status`, `.election-open-status`, `.election-dates`, `.election-open-date`, `.election-close-date`, `.election-website-link`, `.election-countdown`, `.election-actions`                                                                                                                                                                                    |
| Ballot List actions/messages | `.click-to-vote-button`, `.locate-ballot-button`, `.election-results-button`, `.election-event-results-button`, `.support-materials-button`, `.materials-gate-banner`, `.election-selection-warning`, `.elections-empty`                                                                                                                                                                                                                                                                     |
| Instructions                 | `.instructions-title`, `.instructions-description`, `.instructions-steps`, `.instructions-step`, `.instructions-select-step`, `.instructions-review-step`, `.instructions-cast-step`                                                                                                                                                                                                                                                                                                         |
| Eligibility declaration      | `.security-confirmation` (complete declaration), `.security-confirmation-checkbox`, `.security-confirmation-label`                                                                                                                                                                                                                                                                                                                                                                           |
| Start actions                | `.start-voting-button`, `.decline-to-vote-button`                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| Contests and choices         | `.contest`, `.contest-container`, `.contest-title`, `.contest-description`, `.contest-options-toggle`, `.candidates-container`, `.candidates-list`, `.candidates-list-title`, `.candidates-list-toggle`, `.candidates-list-checkbox`, `.candidates-selected-count`, `.candidate-item`, `.candidate-title`, `.candidate-description`, `.candidate-checkbox`, `.candidate-input`, `.candidate-writein-textfield`, `.candidate-position-select`, `.candidate-position-label`, `.candidate-link` |
| Vote/Review actions          | `.back-button`, `.clear-selection-button`, `.next-button`, `.edit-ballot-button`, `.audit-button`, `.cast-ballot-button`, `.cast-ballot-error`                                                                                                                                                                                                                                                                                                                                               |
| Review tracker               | `.hash-container`, `.hash-text`, `.hash-actions`, `.hash-copy-button`, `.hash-help-button`, `.hash-copy-status`                                                                                                                                                                                                                                                                                                                                                                              |
| Locator search/result        | `.ballot-locator-content`, `.ballot-locator-search`, `.ballot-locator-result`, `.ballot-id-field` (whole field), `.ballot-id-input`, `.ballot-id-error`, `.ballot-lookup-status`, `.ballot-content-description`, `.ballot-content`, `.locate-ballot-button`, `.locate-again-button`, `.back-button`                                                                                                                                                                                          |
| Locator tabs/logs            | `.ballot-locator-tabs`, `.ballot-lookup-tab`, `.ballot-lookup-panel`, `.cast-vote-logs-tab`, `.cast-vote-logs-panel`, `.cast-vote-logs-title`, `.cast-vote-logs-table`, `.cast-vote-logs-pagination`, `.cast-vote-log-copy-button`                                                                                                                                                                                                                                                           |

Validation messages also retain their [message-specific CSS hooks](./08-ballot-errors-custom-css.md).

## Confirmation Ballot ID

| Component                                                | Selector within `.confirmation-screen`                                            |
| -------------------------------------------------------- | --------------------------------------------------------------------------------- |
| Complete Ballot ID row, including its label and controls | `.ballot-id-container`                                                            |
| Bordered value group                                     | `.ballot-id-border`                                                               |
| Label / status icon                                      | `.ballot-id-label` / `.ballot-id-status-icon`                                     |
| Both responsive values                                   | `.ballot-id-value`                                                                |
| Individual responsive variants                           | `.ballot-id-value-desktop`, `.ballot-id-value-mobile`                             |
| Help control                                             | `.ballot-id-help-button`                                                          |
| Verification heading / description / QR                  | `.ballot-verification-title`, `.ballot-verification-description`, `.qr-container` |
| Receipt / finish actions                                 | `.print-receipt-button`, `.finish-button`                                         |

Confirmation dialogs use `.ballot-id-help-dialog`, `.demo-ballot-id-help-dialog`,
`.demo-ballot-url-dialog`, `.demo-print-receipt-dialog` and `.print-receipt-error-dialog`.

To hide **only the complete Ballot ID row**, on desktop and mobile:

```css
.confirmation-screen .ballot-id-container {
  display: none;
}
```

This rule also hides the row's help control and responsive value links, so they do
not leave keyboard stops behind. It does not hide the verification description,
QR code, receipt action or Finish button. No section is hidden by default, and
CSS does not change ballot encryption, casting or verification. Fully acclaimed
elections already omit the Ballot ID and QR because no ballot is cast.

Keep labels and their controls together when hiding other sections; do not hide
an accessible-name or description target while leaving its control visible.
Recheck keyboard focus, contrast and responsive layout after applying tenant CSS.
Rebuild `ui-essentials` before verifying source changes to shared components.
