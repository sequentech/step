---
id: admin_portal_tutorials_initialization-ceremony
title: Initialization Ceremony (runbook)
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

This runbook is for the witnessed ceremony that initializes voting at each
Post before it opens. It covers the three initialization scopes, what each
screen and log entry shows, and how to recover when something goes wrong.

A **Post** is an election of the election event. Its **countries** are the
areas where its voters vote: the areas under the Post that have a ballot
style of it. Areas that only group other areas are not countries. Every
country under a Post uses the Post's timezone.

## Before the ceremony

Check these settings in the Admin Portal. They are part of the configuration,
so once the event is published they are signed with it.

| Where | Setting | What it does |
| --- | --- | --- |
| Election > Data | **Initialize Report Policy**: Required | The Post doesn't open until it is initialized. |
| Election Event > Data > Voting lifecycle | **Initialization scope** | What "initialized" means for opening: see the scopes below. |
| Election Event > Data > Language, Date and Time | Configured timezones, primary timezone; the Post's timezone in Election > Data | The zones the Initialization Report prints. |
| Election Event > Signatures > Protected actions | **Initialize voting** rule | Whether generating the report needs the election's signers, and how many. |

:::warning Only Posts whose report is Required wait

The initialization scope applies only to Posts whose **Initialize Report
Policy** is **Required**. A Post set to Not required neither waits for
initialization nor is waited for: with the scope Event, the other Posts don't
wait for it, and with the scope Post and country, its countries aren't
checked. If no Post requires its report, the scope gates nothing. Set the
policy to Required on every Post that must be initialized before it opens.

:::

Also check that:

- the Post is published (Publish tab) and its keys ceremony has finished;
- the Initialization Report is allowed for the Post (Election > Data, or the
  `ALLOW_INIT_REPORT` scheduled event);
- the witnesses can see the screen where the report is generated and the
  report itself.

## The three scopes

The Post's own report is always needed. The scope adds to it.

| Scope | A Post opens when | Typical use |
| --- | --- | --- |
| **Post** (default) | its Initialization Report exists | Each Post runs its own ceremony and opens on its own. |
| **Event** | its report exists **and** every Post that requires one is initialized | All Posts open together, after the last ceremony. |
| **Post and country** | its report exists **and** every country under it is initialized | A Post with several countries runs one ceremony per country, or one for all of them. |

**Both copies must allow it.** The scope is read from the event as it is now
and from the last published configuration. A Post opens only when both allow
it:

- making the scope stricter (for example Post → Event) applies at once;
- making it looser applies after the next publication (and its approval when
  Approve configuration needs signatures);
- before anything is published, the default (Post) applies.

## Steps by scope

### Post

1. Open the Post's **Publish** tab and press **Generate Initialization
   Report**. Confirm.
2. If **Initialize voting** needs signatures, a signing request starts. Read
   its code to the witnesses; the election's signers sign it in the
   **Signatures** panel, each with their certificate. The report is generated
   when the last required signature arrives.
3. The report runs as an initialization report tally (Tally tab). When it
   completes:
   - the Post is initialized: **Generate Initialization Report** is disabled
     and the Post can open;
   - the report is available in the Tally tab's results for that session;
   - the Logs tab shows an **ElectionInitialized** step for each country of
     the Post.
4. Print or show the report. The witnesses check the Post's name, the
   generation time, zero ballots counted and the **Report hash**, and compare
   the hash with the ElectionInitialized entries in the Logs tab.

### Event

Run the Post steps for **every** Post that requires initialization. No Post
opens until the last one is initialized: an opening before that is refused
with *with the initialization scope "event", no Post opens until every Post
whose initialization report is required is initialized*, naming the Posts
still missing (the first five, then how many more).

### Post and country

Either initialize the whole Post at once (the Post steps: one report covers
every country, one ElectionInitialized step per country), or one country at a
time:

1. Generate the report for one country: `create_tally_ceremony` with
   `tally_type: INITIALIZATION_REPORT`, the Post in `election_ids` and the
   country in `area_ids`. The report covers that country only.
2. If **Initialize voting** needs signatures, the signing request is for that
   Post **and country** (scope Post and country): one request per country can
   wait at a time. A country that isn't under the Post, or a scope without
   Post and country, is refused before anyone signs.
3. When it completes, the country is recorded as initialized (one
   ElectionInitialized step naming it). The Post counts as initialized once
   its last country is.
4. Repeat for each country. Until the last one, opening the Post is refused
   with *with the initialization scope "Post and country", a Post whose
   initialization report is required opens once every country under it is
   initialized*, naming the countries still missing.

Generating a report per country is refused unless the initialization scope is
Post and country (in either copy), the report is an initialization report of
one Post, and every country chosen is under that Post.

## What the report shows

The Initialization Report prints:

- **Generated**: the time it was generated in the Post's zone, with the time
  in the primary zone in brackets when the Post uses another zone. Each time
  carries its zone label (the `timezones.abbr.<zone>` text);
- **Voting Period**: from the Post's opening (in its zone) to the common close
  (in the primary zone);
- the number of registered voters, zero ballots counted and every candidate at
  zero;
- **Report hash**: the hash of the report's data. The same value is in the
  ElectionInitialized log entries, so a printed report can be checked against
  the electoral log.

Zone labels and date wording are translation texts (`timezones.*`); change
them in the event's Localization tab with the templates or global scope.

:::note Example

A Post in Dubai with a Manila primary zone, whose label for `Asia/Manila` is
PhST: *Generated April 08, 2028 23:41 GMT+4 (April 09, 2028 03:41 PhST)* and
*Voting Period: 09 April 2028 00:00 GMT+4 - 08 May 2028 19:00 PhST*.

:::

## What the logs show

Each initialization logs, per country (or once for a Post without
countries), one **ElectionInitialized** step, filed under the Post and the
country:

- a USER entry naming the person who generated the report;
- a SYSTEM entry with what was recorded. It is an ERROR entry when the report
  hash is missing (the report's results had none); the description says so.

The description reads *Initialized Post {Post}, country {country} at {time}
with initialization report hash {hash}.* Its details carry the Post, the
country, the report hash, the report document, the tally session and the
time. The steps are queued right after the report's tally completes and
posted to the electoral log in order, once each; the electoral log is
tamper-evident.

The database keeps a dated record of every initialization: one row per
country, with the report hash and document. A new report for the same country
adds a row; earlier ones stay as history. A tally re-run of the same report
records nothing again.

## Opening after the ceremony

| How the Post opens | When it isn't initialized at the scope |
| --- | --- |
| Manually, on the Post's Publish tab | Refused, with the reason. |
| With signatures (Open voting) | The request isn't created, and the reason is shown. A request created before (for example before the scope changed) is refused when it would run, with the same reason. |
| Scheduled opening of the Post | It doesn't open and the row stays active: the scheduler opens the Post at its first run after it is initialized at the scope. The wait is logged once per reason (a SigningActionExecuted step by `scheduled-event`). |
| Scheduled opening after the Post's voting period closed (its own close, else the event-wide one) | It doesn't open the Post; the row stops, and the refusal is logged. |
| Event-wide manual opening | Refused as a whole while any Post it would open can't, naming them. |
| Event-wide scheduled opening | Opens the Posts that can and keeps the row active for the others, logging once why each waits. Later runs only open the Posts still waiting (a Post it opened and that was paused since is not reopened). Once the event-wide close has passed, it stops and logs the Posts that didn't open. |

## Recovery

| Situation | What to do |
| --- | --- |
| The report tally fails or stalls | Check the Tally tab for the session's status and logs; generate the report again. Nothing is recorded until a report completes. |
| The signing request expired or was cancelled | Start again from **Generate Initialization Report**; a new request gets a new code. |
| A country was added to the Post after its report | Under **Post and country**, the Post can't open until the new country is initialized: generate a report for that country. Under **Post** and **Event**, nothing changes. |
| The scope was changed to Post and country after a Post was initialized with an older report | Rows exist only for initializations recorded by this version; generate a report for each country still missing (the refusal names them). |
| The wrong country was initialized | Nothing to undo: it is a valid record. Initialize the right one. |
| A scheduled opening is waiting | Finish the ceremony; the scheduler opens the Post at its next run. The common close stays authoritative: after it, the opening doesn't run. |
| An ElectionInitialized step is missing from the Logs tab | The steps are queued after the tally's commit; if the service stopped in between, they are queued with the next completed report of the event. Check the record of the initialization in the database meanwhile. |
| The printed hash doesn't match the log | Stop. Compare the session id and the time in the log entry with the report's; regenerate the report and keep both for the audit. |
