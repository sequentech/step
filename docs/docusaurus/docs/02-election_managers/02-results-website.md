---
id: results_website
title: Results Website
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Results Website

The results website publishes a deliberately reduced, read-only view of a completed tally. Publication creates a new version; it never modifies the tally database or exposes ballots, document references, internal statistics, or data from unselected events, elections, contests, or areas.

## Policy and permissions

Configure the policy in the election event data screen using its dedicated **Save** action. Saving the rest of the election event does not change this policy.

- `Disabled` removes the event from discovery and cleans up active publication artifacts.
- `Enabled` permits publication using the configured access and visibility values.
- `Public` access supports only `Full event` visibility.
- `Authenticated` access supports `Full event` or `Area based` visibility. Area-based voters receive only the artifact for the area in their event-realm token.

`publish-results-write` is required to configure the policy, publish, revoke, or rebuild discovery. `publish-results-read` is required to read publication history. The generic `admin-user` role cannot read history or change `presentation.results_website` directly.

## Publishing and routes

After a tally completes, select the contests and publish from the tally screen. Event publications are available at `/:electionEventId`; election-specific publications are available at `/:electionEventId/elections/:electionId`. An election route may use an event publication only when that publication explicitly contains the requested election.

Each route has monotonically increasing versions. A successful replacement supersedes the prior version and deletes its public or private artifacts. Mutable discovery indexes and latest aliases use `no-store`; immutable versioned artifacts may be cached.

Empty tallies still render General Information and Participation Summary pie charts as 100% non-voters. Candidate, blank, explicit/implicit blank, invalid, and preferential results retain their typed tally meaning.

## Revocation and audit

Revocation records `Revoked` and `revoked_at` before cleanup, so retrying the same action safely resumes cleanup after an object-store failure. Revocation removes the publication from discovery and deletes its public or private backing objects and document rows. Publication history shows the revocation time.

Publish and revoke actions are written to the electoral log with the publication id, action, route, access, visibility, contest ids, and acting user. If discovery becomes stale after an infrastructure failure, use the results-publication refresh action; retry a revoked publication's revoke action to resume artifact cleanup.

## Custom CSS

The published manifest captures `presentation.css` from the election event and each selected election. Event CSS loads first; the current election CSS loads second and therefore can override it. This works on both event and election routes and when the selected election changes.

Stable selectors use the `seq-results-` prefix. Important examples include:

- `.seq-results-page`, `.seq-results-summary`, `.seq-results-selector`, and `.seq-results-contest`;
- BEM-style elements such as `.seq-results-summary__pie`, `.seq-results-contest__title`, and `.seq-results-selector__selected-result`;
- fixed state classes such as `.seq-results-access--public`, `.seq-results-visibility--area_based`, and `.seq-results-contest--preferential`;
- entity classes such as `.seq-results-event--<id>`, `.seq-results-election--<id>`, `.seq-results-contest--<id>`, `.seq-results-area--<id>`, and entity-specific row/tab classes.

Entity values are reproducibly normalized by replacing characters outside letters, digits, `_`, and `-` with `-`; a missing area uses `global`. Prefer semantic classes for layout and entity classes for election-specific branding.

## Deployment configuration

`RESULTS_PORTAL_URL` must be configured before application deployment so event creation/import can create the dedicated `results-portal` Keycloak client with only the results-site redirect pattern. The results portal uses `RESULTS_PORTAL_CLIENT_ID=results-portal`. `DISABLE_AUTH` is development-only and must not be enabled in deployed environments.
