---
id: enrollment_windows
title: Per-Post enrollment windows
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

Enrollment opens per Post at the Post's local time and closes for every Post
at one event-wide instant. The realm's `registrationAllowed` switch still
opens at the first Post's start and closes at the end. Within that range, a
Keycloak form action checks the window of the Post the voter picked.

## Moving parts

| Part | Where | What it does |
| --- | --- | --- |
| `enrollment_windows::refresh` | `packages/windmill/src/services/enrollment_windows.rs` | Writes the event realm attribute `enrollment_windows`. It runs when an enrollment scheduled event is saved or runs, and on publication. |
| `enrollment-window-check` | `packages/keycloak-extensions/voter-enrollment/.../EnrollmentWindowCheck.java` | A form action in the registration form. It refuses a Post that isn't open and gives `register.ftl` each Post's times. |
| `register.ftl`, `js/enrollment-window.js` | `sequent-theme` | Show the Post's notice and hold Continue back while the Post isn't open. |
| `registration-manual-finish.ftl` | `voter-enrollment` theme resources | Shows the reply-by time (`reply-by-hours` on `lookup-and-update-user`) in the Post's zone. |
| `realm_localization` | `packages/windmill/src/services/realm_localization.rs` | On publication, copies the event's `global:`/`votingPortal:` overrides of `timezones.*` and `enrollment.*` into the realm's localization. |
| `migrate_registration_flows` | `packages/windmill/src/tasks/`, `step-cli step migrate-registration-flows` | Adds the form action to event realms made before it existed. |

## The realm attribute

```json
{
  "Dubai PCG": {
    "election_id": "…",
    "opens_at": "2028-02-08T20:00:00Z",
    "closes_at": "2028-05-08T10:00:00Z",
    "time_zone": "Asia/Dubai",
    "close_time_zone": "Asia/Manila"
  },
  "Rome PE": { "problem": "ambiguous-post" }
}
```

- **Keys** are the options of the `embassy` user-profile attribute.
- **Mapping an option to a Post.** An area whose name or description equals the option decides. Otherwise, the areas whose description contains the option decide (the enrollment application uses the same rule). Either way, the matching areas must vote in exactly one election.
- **Problems.** An option that is ambiguous, or that has no Post, is written as a `problem` entry. Such an entry is refused, logged at ERROR, posted to the electoral log as a Keycloak event of type `enrollment-windows-problem`, and returned in `RefreshReport.problems`.
- **Times.**
  - `opens_at`: the Post's own START_ENROLLMENT_PERIOD, else the event-wide one.
  - `closes_at`: the event-wide END_ENROLLMENT_PERIOD.
  - `null`: no limit on that side.
  - The opening shows in `time_zone`, the Post's zone. The close shows in `close_time_zone`, the event's primary zone.
- **No schedule.** Without any enrollment scheduled event, the attribute is removed. A Post without an entry has no schedule and is let through.
- **Nothing passes unchecked.** A problem entry, an entry the form action can't read, or an attribute that isn't a JSON object refuses the registration with `enrollment.postNotConfigured`.

## Message keys

Defaults are in the `sequent.admin-portal` login bundles for all eight languages. Arguments use Keycloak's `{0}` style:

| Key | Arguments |
| --- | --- |
| `enrollment.opensOn` | `{0}` Post, `{1}` opening (Post's zone), `{2}` close (primary zone) |
| `enrollment.openUntil` | `{0}` close time, `{1}` zone name |
| `enrollment.replyBy` | `{0}` time with zone |
| `enrollment.postNotOpen` | `{0}` Post |
| `enrollment.postNotConfigured` | `{0}` Post |
| `timezones.voterDateTimeZone` | `{0}` date and time, `{1}` zone name |
| `timezones.name.<zone>` | none. Without it, the zone's CLDR long name shows |

Publication copies the event's overrides into the realm:

- Apostrophes are doubled, because Keycloak formats every text with MessageFormat.
- `{{dateTime}}` and `{{zoneName}}` become `{0}` and `{1}`.
- Realm keys under `timezones.` and `enrollment.` that are no longer overridden are removed. A failed removal is logged at ERROR.

## Realm templates and existing realms

- **Templates.** New event realms get the form action from their template:
  - `.devcontainer/keycloak/import/tenant-…-event-….json`. This is the default event realm, the file `KEYCLOAK_ELECTION_EVENT_REALM_CONFIG_S3_KEY` points to in MinIO.
  - The janitor's client realm templates (`windmill/external-bin/janitor/templates/<client>/keycloak.hbs`).
- **S3 copies.** A deployment that keeps its own copy of the default realm in S3 must add the execution there too. Without it, new events start without the check until the migration runs.
- **Existing realms.** Windmill's beat runs `migrate_registration_flows` once at start, as it does for `migrate_realm_permissions`. To run it by hand, use `step-cli step migrate-registration-flows`. For each event realm, it finds the form flow of the realm's registration flow (the `registration-page-form` execution) and appends a REQUIRED `enrollment-window-check`. A realm that already has the check is skipped.

## Known gaps

- **The page state is fixed when the page renders.** A page left open across the opening instant keeps Continue held back until it is reloaded. The server check always uses the current time.
- **Realm texts are written inside the publication's database transaction.** If the transaction then fails, the realm already has the new texts. The copy is idempotent, and the next publication writes them again.
- **Removing a realm text whose key contains `/` (zone keys) needs an end-to-end check against Keycloak.** The VOTE-LIFECYCLE journey removes a `timezones.name.Asia/Manila` override and checks that the theme default shows again.
