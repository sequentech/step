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
button. Existing classes remain supported. Authored layout elements, labels,
icons, loading indicators and controls have descriptive hooks as well as their
containing sections. React fragments, providers and routing components do not
produce their own DOM elements; their rendered children carry the hooks.
Library-generated internals still use the library's public classes, such as
`.MuiCheckbox-root` and `.MuiInputLabel-root`, scoped to the relevant component.

## Screens and shared structure

| Screen               | Root selector                                                          |
| -------------------- | ---------------------------------------------------------------------- |
| Ballot List          | `.election-selection-screen`                                           |
| Instructions         | `.start-screen`                                                        |
| Vote                 | `.voting-screen`                                                       |
| Review               | `.review-screen` (unavailable-ballot fallback: `.review-error-screen`) |
| Confirmation         | `.confirmation-screen`                                                 |
| Ballot Tracking      | `.ballot-locator-screen`                                               |
| Audit                | `.audit-screen`                                                        |
| Support materials    | `.support-materials-screen`                                            |
| Error                | `.error-screen`                                                        |
| Login / registration | `.login-screen`, `.register-screen`                                    |
| Publication preview  | `.preview-publication-screen`                                          |

Screen roots also have `.screen`. Within the applicable screen, use
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
classes directly from a global stylesheet, not as descendants of a screen.
The election's Custom CSS is currently scoped to the app wrapper and does not
reach those portals; adding hooks does not change that stylesheet scope.
Page-help dialogs have
`.screen-help-dialog` plus one of `.election-selection-help-dialog`,
`.voting-help-dialog`, `.review-help-dialog`, `.confirmation-help-dialog` or
`.ballot-locator-help-dialog`. Other dialogs include `.demo-dialog`,
`.decline-to-vote-dialog`, `.ballot-validation-dialog`, `.audit-ballot-dialog`,
`.confirm-cast-ballot-dialog`, `.review-ballot-id-help-dialog`,
`.ballot-status-dialog` (the dialog's paper), `.vitrine-booth-refusal-dialog`,
`.logout-dialog` and `.session-expiry-dialog`.

## Individual contests

On both Vote and Review, a contest with `external_id: "1001"` has the class
`.c-1001` on its `.contest-container`. Existing `.contest-0`, `.contest-1`, etc.
remain supported. Use the external-ID class when styling must survive reordering,
pagination, import-generated UUIDs or different voter-group ballot styles.

```css
.c-1001 .candidate-item {
    background-color: #fff4cc;
}
```

The class uses the same formatting rules as election classes: preserve case,
remove whitespace and all characters except ASCII letters, digits, `_` and `-`,
prepend `c-`, then truncate the complete class to 40 characters. Missing, null,
empty or entirely stripped IDs add no class. Choose external IDs that remain
unique after this formatting; different IDs can otherwise produce the same class.

The existing `.app-root` `e-*` class comes from the election's translated alias
or name, with default-language and raw alias/name fallbacks, finally the election
UUID. It does **not** come from `election.external_id`, and can change with language.
Thus `.e-Senat .c-1001 .candidate-item` applies when the election's resolved
alias/name is `Senat` and the contest's external ID is `1001`.

The external ID is included in the Borsh representation used to sign ballot
styles. This changes the signed format even when the external ID is absent.
Upgrade and republish before voting begins; do not introduce this change during
an active election. Republishing does not update signatures on already-cast
ballots, which require verification software compatible with the old format.
Older JSON without the field still loads. No database migration or new
configuration field is required.

## Screen-specific components

| Area                         | Supported hooks                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| ---------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Ballot cards                 | `.elections-list`, `.election-list-item` (complete card), `.election-item`, `.election-info`, `.election-title`, `.election-vote-status`, `.election-open-status`, `.election-dates`, `.election-open-date`, `.election-close-date`, `.election-website-link`, `.election-countdown`, `.election-actions`                                                                                                                                                                                    |
| Ballot List actions/messages | `.click-to-vote-button`, `.click-to-vote-target` (wraps the button when it is off with the hint that the voter already cast the ballot), `.click-to-vote-hint` (that hint, for screen readers), `.locate-ballot-button`, `.election-results-button`, `.election-event-results-button`, `.support-materials-button`, `.materials-gate-banner`, `.election-selection-warning`, `.elections-empty`                                                                                              |
| Instructions                 | `.instructions-title`, `.instructions-description`, `.instructions-steps`, `.instructions-step`, `.instructions-select-step`, `.instructions-review-step`, `.instructions-cast-step`                                                                                                                                                                                                                                                                                                         |
| Eligibility declaration      | `.security-confirmation` (complete declaration), `.security-confirmation-checkbox`, `.security-confirmation-label`                                                                                                                                                                                                                                                                                                                                                                           |
| Start actions                | `.start-voting-button`, `.decline-to-vote-button`                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| Contests and choices         | `.contest`, `.contest-container`, `.contest-title`, `.contest-description`, `.contest-options-toggle`, `.candidates-container`, `.candidates-list`, `.candidates-list-title`, `.candidates-list-toggle`, `.candidates-list-checkbox`, `.candidates-selected-count`, `.candidate-item`, `.candidate-title`, `.candidate-description`, `.candidate-checkbox`, `.candidate-input`, `.candidate-writein-textfield`, `.candidate-position-select`, `.candidate-position-label`, `.candidate-link` |
| Vote/Review actions          | `.back-button`, `.clear-selection-button`, `.next-button`, `.edit-ballot-button`, `.audit-button`, `.cast-ballot-button`, `.cast-ballot-error`                                                                                                                                                                                                                                                                                                                                               |
| Review Tracking ID           | `.hash-container`, `.hash-text`, `.hash-actions`, `.hash-copy-button`, `.hash-help-button`, `.hash-copy-status`, and in events without transparency `.hash-check`, `.hash-check-icon`                                                                                                                                                                                                                                                                                                        |
| Tracking search/result       | `.ballot-locator-content`, `.ballot-locator-search`, `.ballot-locator-result`, `.ballot-id-field` (whole field), `.ballot-id-input`, `.ballot-id-error`, `.ballot-id-help`, `.ballot-lookup-status`, `.ballot-content-description`, `.ballot-content` (also the record viewer of the result), `.locate-ballot-button`, `.locate-again-button`, `.back-button`                                                                                                                                |
| Tracking tabs/logs           | `.ballot-locator-tabs`, `.ballot-lookup-tab`, `.ballot-lookup-panel`, `.cast-vote-logs-tab`, `.cast-vote-logs-panel`, `.cast-vote-logs-title`, `.cast-vote-logs-table`, `.cast-vote-logs-pagination`                                                                                                                                                                                                                                                                                         |

Validation messages also retain their [message-specific CSS hooks](./08-ballot-errors-custom-css.md).

## Supported CSS features

The voting portal applies Custom CSS after removing the parts that load files from
other sites or add text to the page:

- `url()` must point to the voting portal's own origin, the public files bucket or
  a `data:` URI. A declaration with any other URL is skipped, so host images in the
  public bucket instead of linking to another site.
- Only `@media`, `@supports`, `@container`, `@layer` and `@keyframes` at-rules are
  kept. `@import` and `@font-face` rules are skipped, as are `image-set()`,
  `image()` and `src()` values.
- `content`, `quotes`, `list-style`, `list-style-type`, `text-overflow`,
  `text-emphasis`, `text-emphasis-style` and `hyphenate-character` are skipped when
  their value has a non-empty string or a `var()` reference. `content: ""` for
  decorative pseudo-elements still works.
- Comments are removed. Declarations, rules and at-rules with backslash escapes,
  control characters or `//` outside a string are skipped.
- If the stylesheet cannot be parsed, for example because of an unbalanced brace
  or quote, none of it is applied.

The same rules apply to the CSS of ballot styles that were published earlier.

## Confirmation Tracking ID

| Component                                             | Selector within `.confirmation-screen`                                            |
| ----------------------------------------------------- | --------------------------------------------------------------------------------- |
| Complete Tracking ID row, with its label and controls | `.ballot-id-container`                                                            |
| Bordered value group                                  | `.ballot-id-border`                                                               |
| Label / status icon                                   | `.ballot-id-label` / `.ballot-id-status-icon`                                     |
| Status check mark, with transparency                  | `.ballot-status-button`                                                           |
| Both responsive values                                | `.ballot-id-value`                                                                |
| Individual responsive variants                        | `.ballot-id-value-desktop`, `.ballot-id-value-mobile`                             |
| Short ID and its tag, with transparency               | `.ballot-id-text`, `.ballot-id-short`, `.status-tag`                              |
| Help control                                          | `.ballot-id-help-button`                                                          |
| Copy control / status                                 | `.hash-copy-button`, `.hash-copy-status`                                          |
| Verification heading / description / QR               | `.ballot-verification-title`, `.ballot-verification-description`, `.qr-container` |
| Receipt / finish actions                              | `.print-receipt-button`, `.finish-button`                                         |

When the election event keeps a transparency log, `.ballot-status-button`
replaces `.ballot-id-status-icon`, and one `.ballot-id-value` link holds
`.ballot-id-short` and is followed by a `.status-tag`. `.ballot-id-status-icon`,
`.ballot-id-value-desktop` and `.ballot-id-value-mobile` exist only in events
without transparency.

Confirmation dialogs use `.ballot-id-help-dialog`, `.demo-ballot-id-help-dialog`,
`.demo-ballot-url-dialog`, `.demo-print-receipt-dialog` and `.print-receipt-error-dialog`.

To hide **only the complete Tracking ID row**, on desktop and mobile:

```css
.confirmation-screen .ballot-id-container {
  display: none;
}
```

This rule also hides the row's copy/help controls and responsive value links, so they do
not leave keyboard stops behind. It does not hide the verification description,
QR code, receipt action or Finish button. No section is hidden by default, and
CSS does not change ballot encryption, casting or verification. Fully acclaimed
elections already omit the Tracking ID and QR because no ballot is cast.

Keep labels and their controls together when hiding other sections; do not hide
an accessible-name or description target while leaving its control visible.
Recheck keyboard focus, contrast and responsive layout after applying tenant CSS.
Rebuild `ui-essentials` before verifying source changes to shared components.

## Tracking ID and status in events with transparency

When an election event keeps a transparency log, the Review and Confirmation
Tracking ID boxes, the Ballot List and the Ballot status dialog use these hooks.
The check mark and the tag also appear in the Confirmation box listed above.
Events without transparency keep the hooks of the sections above, with the new
wording only.

| Component                              | Supported hooks                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| -------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Review layout, within `.review-screen` | `.review-header` (steps, title and Tracking ID box), `.review-header-steps`, `.review-header-title`, `.review-header-id`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| Review Tracking ID box                 | `.vitrine-tracking-id` (box with its label), `.vitrine-tracking-id-label` (label shown on phones), `.vitrine-hash-container` (the box, also `.hash-container`), `.vitrine-tracking-id-row`, `.vitrine-tracking-id-text` (also `.hash-text`), `.vitrine-tracking-id-short`, `.vitrine-tracking-id-prefix` (lead-in text, not shown on phones), `.vitrine-short-ballot-id` (the button that shows the full ID), `.vitrine-tracking-id-full` (the full ID, once shown), `.vitrine-tracking-id-full-label`, `.vitrine-tracking-id-full-value`, `.vitrine-tracking-id-full-lead` (its first two groups)                                                            |
| Status tag                             | `.status-tag` (`data-tone` is `neutral`, `success`, `info`, `warning` or `danger`), `.status-tag-dot`, `.status-tag-dot-circle`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| Check mark                             | `.ballot-status-button` (`data-status` is `notcast`, `checking`, `recorded`, `witnessed`, `verified`, `late`, `problem`, `auditable` or `auditfail`), `.ballot-status-icon`, `.ballot-status-icon-single`, `.ballot-status-icon-double`, `.ballot-status-mark`, `.ballot-status-badge`, `.ballot-status-badge-disc`, `.ballot-status-badge-mark`                                                                                                                                                                                                                                                                                                              |
| Ballot cast label of the Ballot List   | `.ballot-cast-button` (also a `.ballot-status-button`), `.ballot-cast-label`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| Ballot status dialog: frame            | `.ballot-status-dialog`, `.ballot-status-bar`, `.ballot-status-title`, `.ballot-status-close`, `.ballot-status-body`, `.ballot-status-meta` (election and date), `.ballot-status-actions`, `.ballot-status-save`, `.ballot-status-ok`                                                                                                                                                                                                                                                                                                                                                                                                                         |
| Ballot status dialog: Tracking ID      | `.ballot-status-id`, `.ballot-status-id-head`, `.ballot-status-id-label` (next to its `.status-tag`), `.ballot-status-short-id`, `.ballot-status-full-label`, `.ballot-status-full-row`, `.ballot-status-full-id`, `.ballot-status-full-line`, `.ballot-status-full-lead`, `.ballot-status-copy`, `.ballot-status-copy-status`                                                                                                                                                                                                                                                                                                                                |
| Ballot status dialog: state and steps  | `.ballot-status-headline`, `.ballot-status-headline-icon`, `.ballot-status-headline-text`, `.ballot-status-head`, `.ballot-status-sub`, `.ballot-status-steps`, `.ballot-status-step` (each step, also `.ballot-status-step-record`, `.ballot-status-step-witnesses` or `.ballot-status-step-anchors`), `.ballot-status-step-icon`, `.ballot-status-step-text`, `.ballot-status-step-title`, `.ballot-status-step-detail`, `.ballot-status-step-word`, `.ballot-status-late`, `.ballot-status-problem`, `.ballot-status-problem-text`, `.ballot-status-save-evidence`, `.ballot-status-caption`, `.ballot-status-caption-text`, `.ballot-status-tracker-link` |

The Ballot status dialog renders in a portal, like the other dialogs: target its
hooks from a global stylesheet, since the election's Custom CSS does not reach it.

## Ballot Tracking in events with transparency

Ballot Tracking keeps `.ballot-locator-screen` and the hooks listed above. When
the election event keeps a transparency log, it adds these. Scope repeated inner
hooks to their containing section.

| Component                     | Supported hooks                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| ----------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Header and messages           | `.ballot-locator-header`, `.ballot-locator-title-actions` (**Back** and **Find another Ballot**, at the top), `.ballot-locator-heading` (title and description, without transparency), `.ballot-locator-actions` (**Back** at the bottom, without transparency and in the Logs tab), `.ballot-locator-success` (found message, without transparency), `.ballot-found-status-text` (found message for screen readers), `.ballot-locator-failure` (not found message), `.ballot-found-short-note` (note for a ballot found by its short ID)                                                                                                                                                                                                                                                                                                                                                                                                          |
| Search card                   | `.tracker-search` (the card), `.tracker-search-content`, `.tracker-search-seal` (check badge), `.tracker-search-title`, `.tracker-search-intro`, `.tracker-search-field`, `.tracker-search-label`, `.tracker-search-icon`, `.ballot-id-field` (whole field, with its button), `.ballot-id-input`, `.locate-ballot-button` (**Find**), `.locate-ballot-label`, `.ballot-id-error`, `.ballot-id-help` (format chips), `.ballot-id-chip`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| Cards under the search card   | `.tracker-features` (the three cards), `.tracker-feature` (each card, also `.tracker-feature-cast`, `.tracker-feature-audited` or `.tracker-feature-secret`), `.tracker-feature-icon-box`, `.tracker-feature-title`, `.tracker-feature-text`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| Ballots that share a short ID | `.ballot-matches` (the list), `.ballot-matches-title`, `.ballot-match` (each ballot, a link), `.ballot-match-id`, `.ballot-match-time`, `.ballot-match-arrow`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| Result: Tracking ID           | `.tracker-receipt` (the whole result), `.tracker-ids` (this side), `.tracker-seal`, `.tracker-id`, `.tracker-id-label`, `.tracker-short-id`, `.tracker-tags` (the tags, each a `.status-tag`), `.tracker-id-help`, `.tracker-full-id-box`, `.tracker-full-id-label`, `.tracker-full-id`, `.ballot-id-short-groups` (the first two groups of an ID), `.tracker-copy-button` (also `.copied` for a moment after copying), `.tracker-record-entry-line`, `.tracker-entry`, `.tracker-save-button` (**Save receipt**)                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| Result: status and steps      | `.tracker-status` (this side), `.tracker-status-block`, `.tracker-status-head`, `.tracker-status-sub`, `.tracker-steps`, `.tracker-step` (each step, also `.tracker-step-record`, `.tracker-step-witnesses`, `.tracker-step-hubs` or `.tracker-step-browser`, and its state: `.tracker-step-done`, `.tracker-step-active`, `.tracker-step-scheduled`, `.tracker-step-waiting`, `.tracker-step-late`, `.tracker-step-failed` or `.tracker-step-skipped`), `.tracker-step-title`, `.tracker-step-time`, `.tracker-step-detail`, `.tracker-witnesses`, `.tracker-witness`, `.tracker-witness-count`, `.tracker-late`, `.tracker-problem`, `.tracker-problem-text`, `.tracker-problem-actions`, `.tracker-save-evidence-button`, `.tracker-how-to-report-button`, `.tracker-how-to-report-text`, `.tracker-caption`, `.tracker-caption-text`, `.tracker-check-button` (**Check it independently**), `.tracker-note` (when there is no receipt to show) |
| Audited ballot                | `.tracker-selections` (the **Your selections, decoded** card), `.tracker-selections-header`, `.tracker-selections-title`, `.tracker-print-button`, `.tracker-selections-note`, `.tracker-selection` (each contest), `.tracker-selection-title`, `.tracker-selection-choices`, `.tracker-audit-failed` (the **Audit failed** card), `.tracker-audit-failed-title`, `.tracker-audit-failed-text`, `.tracker-audit-unchecked` (the card for an audit that could not be checked), `.tracker-audit-unchecked-title`, `.tracker-audit-unchecked-text`                                                                                                                                                                                                                                                                                                                                                                                                    |
| Record viewer                 | `.record-viewer` (the card; in the result it is also `.ballot-content`), `.record-viewer-bar`, `.record-viewer-toggle` (the title row, which expands the record), `.record-viewer-badge`, `.record-viewer-title`, `.record-viewer-meta`, `.record-viewer-copy`, `.record-viewer-download`, `.record-viewer-preview` (the one-line preview), `.record-viewer-body` (the expanded record), `.record-viewer-line`, `.record-viewer-line-number`, `.record-viewer-code`, `.record-token` (also `.record-token-key`, `.record-token-string`, `.record-token-number`, `.record-token-literal`, `.record-token-punctuation` or `.record-token-text`)                                                                                                                                                                                                                                                                                                      |

## Inner elements and secondary screens

Scope repeated inner hooks to their containing section. For example,
`.instructions-select-step .instructions-step-title` selects only the first
instruction heading. Button labels and icons have separate hooks such as
`.finish-button-label`, `.print-receipt-label`, `.print-receipt-icon` and
`.print-receipt-progress`; hiding a label alone does not hide its button.

| Area                     | Useful inner hooks                                                                                                                                                                                                                                                                                             |
| ------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| App/loading              | `.voting-portal-wrapper`, `.loader`, `.loader-progress`, `.loader-label`, `.apollo-loading`, `.apollo-loading-progress`                                                                                                                                                                                        |
| Screen titles            | `.screen-title-text`                                                                                                                                                                                                                                                                                           |
| Header/footer            | `.header-page-limit`, `.header-content`, `.app-version-content`, `.app-version-label`, `.app-version-value`, `.footer-powered-by`                                                                                                                                                                              |
| Instructions             | `.instructions-step-title`, `.instructions-step-description`                                                                                                                                                                                                                                                   |
| Candidates               | `.candidate-text`, `.candidate-write-in`, `.candidate-write-in-label`, `.candidate-preference-label`, `.candidate-preference-option`                                                                                                                                                                           |
| Candidate groups         | `.candidates-list-heading`, `.candidates-list-actions`, `.candidates-list-toggle-label`, `.candidates-list-toggle-icon`, `.candidates-list-collapse`, `.candidates-subtype`, `.candidates-subtype-title`, `.candidates-subtype-list`                                                                           |
| Validation               | `.write-in-error`, `.contest-validation-warning`, `.contest-validation-info`, `.warn-box-content`, `.warn-box-severity`, `.warn-box-close-button`                                                                                                                                                              |
| Audit                    | `.audit-download-container`, `.audit-download-title`, `.audit-download-description`, `.download-auditable-ballot-button`, `.auditable-ballot-data`, `.audit-verification-title`, `.audit-verification-description`, `.ballot-verifier-link`, `.audit-warning`, `.print-audit-button`, `.restart-voting-button` |
| Materials                | `.support-materials-header`, `.support-materials-list`, `.support-material`, `.support-material-title`, `.support-material-subtitle`, `.support-material-actions`, `.support-material-preview-button`                                                                                                          |
| Material preview         | `.support-material-preview-dialog`, `.support-material-preview-content`, `.support-material-image`, `.support-material-pdf`, `.support-material-video`, `.support-material-audio`, `.support-material-download-button`                                                                                         |
| Material acknowledgement | `.materials-acknowledgement`, `.materials-acknowledgement-label`, `.materials-acknowledgement-checkbox`, `.materials-acknowledgement-error`, `.materials-actions`, `.materials-continue-button`                                                                                                                |
| Tracking logs            | `.cast-vote-logs-filter`, `.cast-vote-logs-table-container`, `.cast-vote-logs-table-head`, `.cast-vote-logs-table-body`, `.cast-vote-logs-row`, `.cast-vote-logs-sort-button`; column cells use `.cast-vote-logs-ballot-id-cell`, `.cast-vote-logs-kind-cell` and `.cast-vote-logs-timestamp-cell`             |
| Booth refusal dialog     | `.vitrine-booth-refusal`, `.vitrine-booth-icon`, `.vitrine-booth-explanation`, `.vitrine-booth-status`, `.vitrine-booth-status-text`                                                                                                                                                                           |
| Expandable text          | `.expandable-text`, `.expandable-text-content`, `.expandable-text-full`, `.expandable-text-preview`, `.expandable-text-ellipsis`, `.expandable-text-toggle`                                                                                                                                                    |
| Error                    | `.error-content`, `.error-description`, `.error-detail`, `.error-back-link`, `.back-button`                                                                                                                                                                                                                    |

Portalled surfaces also expose `.dialog-container`, `.dialog-backdrop`,
`.dialog-paper`, `.language-selector-paper`, `.language-selector-options`,
`.profile-menu-paper`, `.profile-menu-options`, `.candidate-preference-menu`,
`.candidate-preference-paper`, `.candidate-preference-options`, `.header-tooltip`
and `.header-tooltip-content`. The audit dialogs have their own
`.audit-ballot-id-help-dialog`, `.audit-help-dialog` and `.audit-download-help-dialog` hooks.

### Private ballot logs and cast recovery

Pseudonymous ballot logs use `.ballot-box-log`. Search and pagination controls
expose `.ballot-box-log-search`, `.ballot-box-log-clear-button`,
`.ballot-box-log-page-size`, `.ballot-box-log-previous-button` and
`.ballot-box-log-next-button`. The voter's own receipt uses
`.ballot-box-log-own-ballot`, with `.ballot-box-log-own-check-button` and
`.ballot-box-log-copy-button`. Public rows use `.ballot-box-log-row` on the
desktop table and `.ballot-box-log-mobile-row` on the phone list; their check
controls are `.ballot-box-log-check-button` and `.ballot-box-log-mobile-check-button`.
Loading, error, empty and count messages expose `.ballot-box-log-loading`,
`.ballot-box-log-error`, `.ballot-box-log-empty` and `.ballot-box-log-count`.

The portalled proof dialog uses `.ballot-proof-dialog`, `.ballot-proof-title`,
`.ballot-proof-content`, `.ballot-proof-loading`, `.ballot-proof-error`,
`.ballot-proof-actions`, `.ballot-proof-retry-button` and `.ballot-proof-close-button`.
The help dialog uses the `.ballot-box-log-help-*` family, including
`.ballot-box-log-help-dialog` and `.ballot-box-log-help-close-button`. The portal
stylesheet scope described above also applies to these dialogs.

Pending audit confirmation exposes `.audit-confirmation-loading`,
`.audit-confirmation-error` and `.audit-confirmation-retry-button`. Cast recovery
after secondary authentication uses `.review-cast-recovery`, with
`.review-cast-recovery-loading`, `.review-cast-recovery-error`,
`.review-cast-recovery-retry-button`, `.review-cast-recovery-back-button` and
`.review-cast-recovery-check-button`.
