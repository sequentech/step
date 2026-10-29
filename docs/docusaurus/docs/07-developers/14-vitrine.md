---
id: vitrine
title: Vitrine Transparency Logs
sidebar_label: Vitrine
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Vitrine transparency logs

[Vitrine](https://github.com/sequentech/vitrine) is Sequent's
data-transparency toolkit. An election event can publish its ballots and its
milestones in Vitrine transparency logs: voters get a receipt that their ballot
is in the log, and observers and auditors can check that everyone is shown the
same log. This page describes how step's backend uses Vitrine, how a deployment is
configured, and how to run Vitrine in the development environment. The
integration's design is `docs/step-integration.md` in Vitrine's repository;
the formats are in its `specs/` directory.

Step depends on no Vitrine Rust crate. Its backend talks to the `vitrine-log`
sidecar over HTTP and encodes the log entries itself, and its tests check that
encoding against Vitrine's test vectors. The portals and the ballot verifier
make every check with the vendored `@vitrine/verify` package, in the browser;
the backend makes none for them.

Transparency never blocks voting: no Vitrine failure fails a cast. The one
Vitrine check that stops the booth is its check of the election's
configuration before it encrypts a ballot.

## Overview

An event's transparency mode, `presentation.transparency`, is `disabled` (the
default, also when the key is absent), `public` or `private`. A transparent
event has two logs on the sidecar:

| Log | Profile | Entries | How they arrive |
| --- | --- | --- | --- |
| `ballots` | `vitrine/election/ballots/v1` | One per accepted ballot | The ballot box writes the entry to the log's queue in the event's electoral-log database, in the transaction that stores the ballot. |
| `events` | `vitrine/election/events/v1` | Each election's configuration, `voting-opened`, `voting-closed`, and a milestone when a tally completes and when results are published | Windmill posts signed statements to the sidecar's `POST add`, with the log's submission token. |

Witnesses co-sign the logs' checkpoints. The event's trust policy names the
logs, their keys, the witnesses and the quorum of witnesses that a receipt
needs. Step drafts the policy, the Electoral Board seals and signs it, and
step installs it in both logs.

In a public event, anyone can read the logs. In a private event, only the
policy's auditors can read them, and voters reach their receipts through
Harvest. Vitrine's log engine doesn't run private logs yet, so for now only
public events work end to end.

## Architecture

### Where things are stored

- **`presentation.transparency` and `presentation.vitrine`**, in the election
  event's `presentation`, hold what voters read: the policy ID, the sealed
  policy (public events only), and each log's URL and origin. They never hold
  a token, a secret or a private key.
- **`sequent_backend.vitrine_log`** holds each log as the sidecar created it:
  its name on the sidecar (`<tenant id>/<event id>/<log>`), its log ID and
  instance ID, origin, URL and keys, the `events` log's submission token, the
  log's acknowledgement of the policy, its witness enrolment note, and when it
  was frozen. Hasura doesn't track this table, because of the token: only
  Harvest and Windmill read it.
- **`sequent_backend.vitrine_event`** holds the uploaded signed policy and its
  ID, and the salt of a private event's draft.
- **`sequent_backend.vitrine_statement`** holds the statements that Windmill
  logged in the `events` log, with their entries, leaf hashes and, once a
  checkpoint that meets the quorum covers them, their receipt chains.
- **`vitrine.queue_<log id>`**, in the event's electoral-log database, is the
  `ballots` log's queue. The sidecar creates it and its `vitrine` schema; the
  ballot box only inserts into it.

The trigger `guard_election_event_transparency_update` refuses Hasura updates
of `presentation.transparency` and `presentation.vitrine` once voting opened in
the event. Harvest and Windmill enforce the same lock themselves.

### Making an event transparent

The election event's Transparency tab uses three Hasura actions, all handled
by Harvest and allowed with the permission `election-event-transparency-tab`:

1. **`enable_transparency(election_event_id, mode)`** sets the mode to
   `public` or `private` while voting hasn't opened. It refuses `public` while
   any election has `num_allowed_revotes` other than 0: a public `ballots` log
   shows a coercer whether a voter cast another ballot after the one they
   watched, so voters must be able to vote again as often as they need. It
   creates the `ballots` and `events` logs with the sidecar's
   `POST /admin/logs`, in the event's electoral-log database, and stores them.
2. **`get_transparency(election_event_id)`** returns the mode and why `public`
   is refused, whether the mode is locked, the sidecar's view of the logs and
   their status, the draft policy, the uploaded policy and its ID, the logs'
   enrolment notes, and the explorer URL of the policy.
3. The Electoral Board seals the draft with Vitrine's CLI (`vitrine policy
   seal`), and its authorities sign it as the policy's `sign` line requires.
4. **`upload_transparency_policy(election_event_id, signed_policy)`** checks
   that the policy is for this event, its mode and its elections, and installs
   it in both logs with `PUT /admin/logs/<name>/policy`. The sidecar refuses a
   policy that isn't sealed or isn't valid. Harvest stores the policy, its ID
   and each log's acknowledgement, and fetches each log's witness enrolment
   note with `GET /admin/logs/<name>/enrolment`.
5. The witnesses' operators post the enrolment notes, which the Transparency
   tab shows, to their witnesses. Until a witness has a log's note, it doesn't
   co-sign that log. In development, Harvest posts them itself (see
   `HARVEST_VITRINE_DEV_WITNESS_ADMINS` below).

### Casting a ballot

```mermaid
sequenceDiagram
    participant P as Voting portal
    participant H as Harvest
    participant DB as Event database
    participant S as vitrine-log
    P->>H: insert_cast_vote
    H->>DB: ACCEPT_WITH_VITRINE: the ballot and its entry, in one transaction
    DB-->>H: accepted, with the entry's leaf hash
    S->>DB: sequences the ballots log's queue
    H->>S: wait for the leaf hash, over HTTP/2 cleartext
    S-->>H: a receipt, a promise, queued, or an error
    H-->>P: InsertCastVoteOutput, with vitrine
```

- **The entry.** Harvest casts through Windmill's `insert_cast_vote`, which
  computes the ballot's `accepted_at` once, in whole seconds, and binds it as
  a parameter, so that the stored ballot and its `election/v1` entry agree.
  `ACCEPT_WITH_VITRINE` stores the ballot and inserts the entry into
  `vitrine.queue_<log id>` in the same transaction, and returns the entry's
  leaf hash.
- **Idempotent casting.** A retried cast with the same `ballot_id` from the
  same voter, in the same election, returns the original acceptance: the same
  stored ballot, the same entry and the same receipt. The same `ballot_id`
  from another voter is still refused.
- **The receipt.** Harvest then waits for the entry's receipt on the sidecar's
  internal listener (`GET <log>/wait/<leaf hash>?timeout=600`), within a
  budget of 800 ms. `InsertCastVoteOutput.vitrine`
  is null when the event's transparency is disabled; otherwise it has a
  `kind`, a `text` (data for the portal, never shown as is) and, for `queued`
  and `unavailable`, a `reason`:

  | `kind` | `text` | When |
  | --- | --- | --- |
  | `receipt` | The receipt | The log sequenced the entry within the budget. |
  | `promise` | The promise | The log promised to sequence it within the policy's `mmd`. |
  | `queued` | The entry, in base64 | The log hadn't sequenced it when the wait ended. |
  | `unavailable` | The entry, in base64, if there is one | Something went wrong: the cast still succeeded. |

  The reasons are `queued`, `unknown-entry`, `rejected`, `frozen`, `busy`,
  `log-unavailable`, `timeout`, `unreachable`, `breaker-open`,
  `bad-response`, `not-queued` and `no-log`. The portal asks for the receipt
  later with the entry.
- **The circuit breaker.** After 5 failures in a row that show a sick sidecar
  (`busy`, `log-unavailable`, `timeout`, `unreachable` and `bad-response`),
  Harvest stops calling it for 30 seconds, so that casts lose no time. Then
  one cast tries again: its success closes the breaker, its failure opens it
  for another period.

### Statements in the `events` log

Windmill beat runs `schedule_vitrine_statements` every 15 seconds (the beat
option `--vitrine-statements-interval`). It queues `log_vitrine_statements`
for each transparent event with statements to log or receipts to keep. A
status change that opens or closes voting in a transparent event queues a run
at once. A run logs, in this order:

1. When the event's first election opens: a `vitrine/config/v1` statement for
   each election, then the `voting-opened` milestone, with a `config` line per
   election that names its config entry's leaf hash. Step can publish an
   election's ballots again until voting opens, so its key and ballot styles
   are final then.
2. When a tally completes and when results are published: a milestone,
   `tally-completed` or `results-published`.
3. When the event's last election closes: `voting-closed`, with each
   election's ballot count.

A config statement names the hashes that the booth compares before it
encrypts:

- `key`: the base64 SHA-256 of the election public key's bytes, as the booth
  encrypts to it.
- `styles`: the base64 SHA-256 of the concatenation, in ascending byte order
  and without repeats, of the 32-byte SHA-256 digests of the election's ballot
  styles, each over the exact text that the voting portal receives. Two areas
  with byte-identical styles give one digest.
- `release`: `sequent/voting-portal <version>`, from
  `WINDMILL_VITRINE_PORTAL_VERSION`. The deployment sets it and the portal's
  `APP_VERSION` from one value.

Windmill signs each statement with the operator's `authority` key and stores
it before it submits it, so that every retry submits the same bytes. It then
waits for the entry's receipt (`GET events/wait/<leaf hash>`), upgrades it to
a checkpoint that meets the policy's quorum, and stores the receipt chain.

Vitrine has one voting window per event, spanning all its elections, while
step can open, pause and close elections separately. So pausing writes
nothing, an election's ballots can't be published again after
`voting-opened`, and a transparent event can't reopen after `voting-closed`.

### What voters' booths get

Two more Hasura actions, for voters:

- **`vitrine_voter_context(election_event_id, election_id)`** returns what the
  booth must hold before it encrypts: the mode, the policy and the `ballots`
  log's acknowledgement of it, the election's config statement and the
  `voting-opened` milestone (each with its entry and receipt chain, or null
  until Windmill logged it), the hex SHA-256 of each of the election's
  distinct ballot styles, sorted, and the release.
- **`vitrine_voter_forward(election_event_id, route, body, hub)`**, for
  private events only, forwards `upgrade`, `opening` and `policies` requests
  to the sidecar's internal voter routes. `upgrade` and `opening` require a
  receipt or promise owned by the logged-in voter; `opening` also names a hub.
  `policies` requires an authenticated voter of the event, an empty `body`
  and no `hub`. Harvest supplies the server administrator bearer and voter
  identity to the sidecar; neither is exposed to the browser.

The private version-1 policy-proofs frame supplies supplemental inclusion
only. It cannot establish that no policy entries were omitted, their earliest
quorum times, complete discovery freshness or automatic successor adoption.
The SDK therefore retains the independently accepted history and leaves
those claims pending even for an authentic selective frame. A full private
events prefix is never forwarded to voters. Public discovery can check a
complete prefix for absence and order; a new candidate still needs a separate
earliest-quorum proof before adoption.

### What the portals show

The admin portal's Transparency tab follows Vitrine's design board for the
event's overview. It shows only what `get_transparency` returns, and says
"Not reported" for the rest instead of guessing:

- **The record's health** comes from the stored policy, the logs' reported
  alerts and their witnesses: it is healthy unless the stored policy is
  invalid, a log runs under another policy ID, or a log or witness reports an
  alert.
- **Figures** the sidecar reports: the entries in the logs' witnessed
  checkpoints, the sequencer queue and how long its oldest entry has waited,
  and when an app hub last received an anchor. Not reported: the accepted
  rate, the receipt time, the browser layer, overdue promises, evidence
  reports, the booth release and which milestones the record holds.
- **Witnesses, hubs, monitors and redactions** come from the sealed policy,
  with each witness in its worst state across the event's logs.
- **Visibility** is the mode the logs were created with, locked once voting
  opened. The draft, upload and enrolment workflow stays below the overview.

The voting portal's confirmation screen shows the receipt checked by the
website. Its headline says the ballot is cast until the witnesses confirm it,
then that it is in the public (or election) record; contradicting signed
proofs, or a required receipt that never came, show the failure page instead.
The kept receipts on the election list say whether each ballot is still in
the record. The pages brand the UI kit's elements through the kit's
`--vitrine-accent`, `--vitrine-text`, `--vitrine-surface` and
`--vitrine-radius` custom properties.

### Deleting, exporting and importing events

- Deleting a transparent event freezes its logs first, with
  `POST /admin/logs/<name>/freeze`. The deletion fails if a freeze fails,
  unless the sidecar doesn't know the log.
- An event's export leaves the `vitrine` schema of its electoral-log database
  out. An imported event's transparency is disabled, since the logs belong to
  the event it was exported from.

## The deployment configuration

Harvest drafts each event's policy from the deployment configuration, a TOML
file that `HARVEST_VITRINE_CONFIG_FILE` names, and from the event. The
development environment's is
[`.devcontainer/vitrine/deployment.toml`](https://github.com/sequentech/step/blob/main/.devcontainer/vitrine/deployment.toml).
Harvest checks it whenever it reads it, against the rules of Vitrine's
`specs/policy.md`, section 3, that the configuration decides, so that every
draft made from it is valid. Unknown keys are errors.

Names (of authorities, auditors, witnesses, groups, monitors, hubs and parties)
are 1 to 64 bytes of `A-Z a-z 0-9 . _ -`, and not `none`. URLs are `https`,
or `http` on `localhost`, `127.0.0.1` or `[::1]`. Verifier keys are signed-note
vkeys: Ed25519 for authorities, auditors, monitors and hubs, and
`cosignature/v1` for witnesses.

### Top-level keys

| Key | Required | Meaning |
| --- | --- | --- |
| `domain` | Yes | The logs' origins are `<domain>/<tenant id>/<event id>/<log>`. Printable ASCII without `+`; it can't start with `vitrine/` or `cosignature/`. |
| `operator` | Yes | The party that runs the logs, as a name. |
| `quorum` | Yes | The app quorum: the name of a group or a witness. |
| `min_faults` | No | Every quorum must tolerate this many more faulty parties. |
| `words` | No | The language of policy and receipt words: `en`, `es`, `fr`, `it`, `pt`, `cs`, `ja`, `ko`, `zh-hans` or `zh-hant`. The event's default language is used instead when it is one of them. |
| `mmd` | No | The maximum merge delay of promises, in seconds. Default 30, at most 3600. |
| `explorer_url` | No | The explorer's page for a policy, with `{policy_id}` where the ID goes. The Transparency tab links to it. |
| `monitor_quorum` | No | How many monitors, of distinct parties, must report an availability failure before verifiers show it as established. At least two and at most the number of monitors; at least two distinct monitor parties are required. |
| `browser_period` | No | How often hubs anchor in the browser layer, and app hubs in bootstrap hubs, in seconds. Vitrine's default is 600. |

### Sections

| Section | Keys | Meaning |
| --- | --- | --- |
| `[profiles]` | `election`, `events` | The meaning hashes of the `vitrine/election/ballots/v1` and `vitrine/election/events/v1` profiles that the sidecar runs, each `sha256:` and 64 hex digits. A Vitrine image lists them in `/usr/local/share/vitrine-dev/meaning-hashes`. |
| `[sign]` | `threshold`, `authorities` | How many of the listed authorities must sign a policy. It must list the Board, which seals. |
| `[redaction]` | `threshold`, `authorities` | Optional: who approves redaction notices, in the same shape. Without it, nothing can be redacted. |
| `[anchor]` | `period`, `max_gap` | Optional: public logs that grew anchor in hubs every `period` seconds (at least 1); private logs send blinded anchors every period even when idle. Monitors report `anchor-late` when a checkpoint waited more than `max_gap` seconds for an anchor. Vitrine's default is 10 and 60. |
| `[anchor_hubs]` | `min`, `min_independent` | Optional stronger requirements. The election profiles always require a subset of at least two pairwise independent app hubs, at least one independent of the operator; bootstrap hubs do not count. |
| `[[authorities]]` | `name`, `vkey`, `role` | The policy's authorities. `role` is `board`, `operator` or `other`; exactly one is the Board and exactly one the operator, whose key signs the `events` statements. |
| `[[auditors]]` | `name`, `vkey` | The auditors who may read private logs. A private event needs one. |
| `[[witnesses]]` | `name`, `vkey`, `url`, `party` | The logs' witnesses. `url` is optional; `party` defaults to the witness's own name. |
| `[[groups]]` | `name`, `threshold`, `members` | Groups of witnesses and groups, for the quorum. `threshold` is a number, `any` or `all`. Members are defined before the group and belong to one group only. |
| `[[monitors]]` | `name`, `vkey`, `url`, `party` | The policy's monitors, which check the logs and collect evidence. |
| `[[mirrors]]` | `url`, `party`, `logs` | Mirrors of the logs' tiles. `url` has `{path}` where the log's path on the sidecar goes; `party` defaults to the operator; `logs` (`ballots`, `events`) defaults to both. A public event needs a `ballots` mirror that the operator doesn't run. |
| `[[hubs]]` | `name`, `vkey`, `url`, `operator`, `role`, `witnesses`, `groups`, `quorum` | Anchor hubs for both public and private events. `role` is `app` (the default: the logs anchor in it) or `bootstrap` (it stands in for the browser layer). `witnesses` and `groups` have the shapes above. |

Harvest also checks that:

- no key appears twice anywhere in the configuration, and the logs' own keys
  are none of them;
- with the operator's witnesses counted as faulty, two inconsistent
  checkpoints can't both meet a quorum, and the quorum tolerates `min_faults`
  more faulty parties;
- with app hubs, the witnesses have distinct key names, and there are at most
  31 of them;
- a bootstrap hub isn't run by the operator and is independent of every app
  hub, and the app hubs meet `[anchor_hubs]` and the unconditional election-profile floor;
- the profile bindings equal the two reviewed built-in meaning hashes in the configuration. Unknown, obsolete or future meanings need an explicit integration update and are refused.

### What comes from the event

The draft takes its `name` (`<tenant slug>/<event id>`, or
`<tenant id>/<event id>` when the slug can't be a name), `visibility` (the
mode), `required on`, and for a private event a random `policy-salt`, from the
event. Private drafts also include a `blinded` line for each log: Harvest calls the authenticated, idempotent `POST /admin/logs/<name>/blinded-key` with the derived pseudo-origin. The sidecar generates and durably stores the secret keys; Step keeps only their public vkeys. Both modes retain the hubs and monitor requirements. Its `opens` is the earliest scheduled start of the event's elections and
`closes` the latest scheduled end, grace periods included: the policy can't be
drafted while an election has no scheduled start or end, its own or the
event's. Its `quiet` period is the last 3600 seconds before `closes`, or the
whole window if it is shorter. The `ballots` log's `param` lines name the event
and its elections. Each log's `log`, `log-time`, `mirror`, `mirror-party` and
`profile` lines come from the logs that the sidecar created and the
configuration. Everything else comes from the configuration.

## Environment variables

Empty values count as unset. The compose files pass every one of them to the
services that read them.

### Harvest

| Variable | Default | Meaning |
| --- | --- | --- |
| `HARVEST_VITRINE_INTERNAL_URL` | Unset | The sidecar's internal listener, over HTTP/2 cleartext. Unset, transparent events' casts get `unavailable` and the Transparency tab can't create logs. |
| `HARVEST_VITRINE_ADMIN_TOKEN_FILE` | Unset | A file with the sidecar's admin token, for the Transparency tab. |
| `HARVEST_VITRINE_CONFIG_FILE` | Unset | The deployment configuration. Unset, no event can be made transparent. |
| `HARVEST_VITRINE_BUDGET_MS` | 800 | How long a cast waits for its receipt in all. |
| `HARVEST_VITRINE_WAIT_MS` | 600 | The `timeout` of the sidecar's `wait`, below the budget, so that the sidecar answers before Harvest gives up. |
| `HARVEST_VITRINE_BREAKER_FAILURES` | 5 | The failures in a row that open the circuit breaker. |
| `HARVEST_VITRINE_BREAKER_OPEN_SECS` | 30 | How long the breaker stays open. |
| `HARVEST_VITRINE_DEV_WITNESS_ADMINS` | Unset | Development only: see below. |

`HARVEST_VITRINE_DEV_WITNESS_ADMINS` is a comma-separated list of
`<witness internal URL>|<admin token file>` pairs. When it is set, Harvest
posts each log's enrolment note to those witnesses after a policy upload. In
production, leave it unset: the witnesses' operators post the notes.

### Windmill

| Variable | Meaning |
| --- | --- |
| `WINDMILL_VITRINE_INTERNAL_URL` | The sidecar's internal listener. |
| `WINDMILL_VITRINE_ADMIN_TOKEN_FILE` | A file with the sidecar's admin token, to freeze an event's logs before deleting it. |
| `WINDMILL_VITRINE_AUTHORITY_KEY_FILE` | The operator's `authority` key, which signs the `events` statements, in the `PRIVATE+KEY+<name>+<key ID>+<key>` format of `golang.org/x/mod/sumdb/note`. Its vkey must be the policy's `operator` authority. |
| `WINDMILL_VITRINE_PORTAL_VERSION` | The voting portal's version, for the config statements' `release` line. The compose files set it from `APP_VERSION`. |

Windmill beat's `--vitrine-statements-interval` (default 15) is the time, in
seconds, between its scans for statements to log.

### Electoral log

| Variable | Meaning |
| --- | --- |
| `ELECTORAL_LOG_PG_VITRINE_USER` | The sidecar's role. Windmill lets it connect to and create schemas in each event database it creates. Unset, event databases get no Vitrine grants. |
| `ELECTORAL_LOG_PG_VITRINE_PASSWORD` | Its password. The database's initialization sets it, and `vitrine-setup` writes it into the sidecar's configuration. |
| `ELECTORAL_LOG_PG_VITRINE_DATABASE` | The sidecar's own database, with the registry of its logs. |
| `ELECTORAL_LOG_PG_VITRINE_QUEUE_ROLE` | The role that the sidecar grants a `ballots` log's queue to. The application role and the sidecar's role are members of it. |

`.devcontainer/postgresql/init-electoral-log.sh` creates both roles and the
sidecar's database. An event database created before the sidecar's role
existed gets its grants from `electoral-log-admin upgrade-event-databases`.

## Running in development

The sidecar and two witnesses run under the opt-in compose profile `vitrine`.
The profile supplies the sidecar and log witnesses only. Before enabling an event, separately run the two app hubs, their witnesses, and two monitors named in `.devcontainer/vitrine/deployment.toml`, or replace their URLs, verifier keys and party labels with an explicitly provisioned infrastructure configuration using `HARVEST_VITRINE_CONFIG_FILE`. The checked-in development key files describe those identities; they do not start these additional services. The full Vitrine `deploy/` stack is an example of the required lifecycle, but its keys must be copied from its actual configuration rather than assumed to match this file.

The hub operators must authorize the sidecar's enrolment signer, and the monitor operators must load each event's sealed policy into their monitor configurations before monitoring that event; an answering listener alone does not establish that it watches the event. Until that infrastructure is running, do not enable a production-like transparent event or claim complete verification. Missing hub or monitor responses remain pending or unavailable; the application never lowers profile requirements to make a local stack pass. Without the compose profile, the URLs above don't answer: transparent events' casts get an
`unavailable` receipt, and the Transparency tab can't create logs.

1. Build the image from a Vitrine checkout. It has every Vitrine binary,
   including the `vitrine` CLI:

   ```bash
   cd vitrine
   docker build -f deploy/Dockerfile --target runtime -t vitrine-log:dev .
   ```

   The meaning hashes in `.devcontainer/vitrine/deployment.toml` must be the
   image's:

   ```bash
   docker run --rm vitrine-log:dev cat /usr/local/share/vitrine-dev/meaning-hashes
   ```

2. Start the services with the base stack:

   ```bash
   docker compose --profile base --profile vitrine up -d
   ```

   `vitrine-setup` copies the development keys and tokens of
   `.devcontainer/vitrine/secrets/` into a volume, readable by the services'
   user only, and writes the services' configuration files. The services share
   the network namespace of the `vitrine` service:

   | Port | Service |
   | --- | --- |
   | `8094` | The sidecar's public listener, also on the host at `http://localhost:8094` |
   | `8095` | The sidecar's internal listener: `http://vitrine:8095` for Harvest and Windmill |
   | `8096`, `8097` | Witness 1: public listener and admin listener |
   | `8098`, `8099` | Witness 2: public listener and admin listener |

   The development sidecar runs with `--dev-no-standby`, since its PostgreSQL
   has no synchronous standby.

3. The sidecar's configuration lists the election event databases that exist
   when `vitrine-setup` runs. After creating an election event, and before
   enabling its transparency, run the setup again and restart the sidecar:

   ```bash
   docker compose --profile base --profile vitrine up -d --force-recreate vitrine-setup vitrine-log
   ```

4. In the admin portal, open the event's Transparency tab, enable
   transparency, and copy the draft policy to a file, for example
   `draft.txt`. Seal it as the development Board, whose key is in
   `.devcontainer/vitrine/secrets/board.key`, with the image's CLI:

   ```bash
   docker run --rm --network none --user "$(id -u):$(id -g)" \
     -v "$PWD:/work" -w /work vitrine-log:dev \
     vitrine policy seal draft.txt \
       --key .devcontainer/vitrine/secrets/board.key --authority board \
       --out policy.txt
   ```

   The development `[sign]` asks for the Board's signature only, which
   sealing adds. The `vitrine policy check policy.txt` checks the result; `vitrine policy words policy.txt` shows the policy's words. Upload
   `policy.txt` in the Transparency tab. Harvest installs it in both logs and,
   with `HARVEST_VITRINE_DEV_WITNESS_ADMINS`, enrols the logs in both
   witnesses.

The keys and tokens in `.devcontainer/vitrine/secrets/` are for development
only. In production, the Board keeps its key offline, the operator's key is in
the deployment's secrets, and every token is the deployment's own.

## Tests

From `packages/`:

- **Entries and statements.** `cargo test -p electoral-log --test
  vitrine_vectors` checks step's encoders against Vitrine's vectors, copied in
  `packages/electoral-log/tests/vitrine-vectors/` from Vitrine's
  `profiles/election/ballots/v1/vectors`, `profiles/election/events/v1/vectors` and
  `testdata/styles`. Copy them again when Vitrine's profiles change.
- **The ballot box and its queue.** The `ACCEPT_WITH_VITRINE` contract
  tests, idempotent casting included, are in
  `packages/electoral-log/tests/ballot_box.rs`. They create a minimal copy of
  the sidecar's queue table, and run with the electoral-log PostgreSQL suite
  against a disposable PostgreSQL, as the
  [electoral log's tests](./13-electoral-log/01-electoral-log-design.md#15-testing)
  do: `cargo test -p electoral-log --test ballot_box -- --ignored
  --test-threads=1` with the `ELECTORAL_LOG_TEST_DATABASE_URL` and
  `ELECTORAL_LOG_PG_*` variables of that suite. `--test vitrine_grants`
  checks the sidecar role's grants in the event databases that Windmill
  creates.
- **Harvest's receipt at cast.** `cargo test -p harvest services::vitrine`
  runs the fail-safe tests against local stand-ins of the sidecar: stopped,
  slower than the budget, and answering garbage, each cast gets
  `unavailable`, and the breaker opens and closes.
- **Windmill.** `cargo test -p windmill --lib services::vitrine` covers the
  configuration, the policy draft and its refusals, the statements and their
  hashes, and the own-ballot check of voters' forwards.
- **Against a running sidecar.** `windmill/tests/vitrine_sidecar.rs` creates
  an event's logs, installs a sealed policy, enrols the logs in the witnesses,
  casts a ballot through the ballot box's queue, checks a voter's forward of
  its receipt, and logs and receipts the config and `voting-opened`
  statements. It needs the sidecar and the witnesses, and an event database
  that the sidecar's configuration lists. In the dev container:

  ```bash
  S=/workspaces/step/.devcontainer/vitrine/secrets
  WINDMILL_VITRINE_TEST_INTERNAL_URL=http://vitrine:8095 \
  WINDMILL_VITRINE_TEST_ADMIN_TOKEN_FILE=$S/admin.token \
  WINDMILL_VITRINE_TEST_DATABASE_URL=postgres://electoral_log:electoral_log@postgres:5432/<event database> \
  ELECTORAL_LOG_PG_VITRINE_QUEUE_ROLE=vitrine_queue \
  WINDMILL_VITRINE_TEST_WITNESS_ADMINS="http://vitrine:8097|$S/witness-1.token,http://vitrine:8099|$S/witness-2.token" \
  cargo test -p windmill --test vitrine_sidecar -- --ignored
  ```

### Native queue and production verifier rehearsal

For an isolated contract run, start Vitrine's `vitrine-testkit` example
`integration_server` with a **new** absolute capability-file path. It creates
its own `vitrine-testkit-*` PostgreSQL container, native log, witnesses, hubs,
bootstrap hub and monitor listeners. Keep that process running while the
checks run; stop it with SIGTERM or Ctrl-C to shut down its services and
remove its own container. The owner-only capability file contains test
credentials: never commit or publish it.

Build `windmill`'s `vitrine_sidecar` test with `--no-run`, then use the exact
executable path printed by Cargo:

```sh
cargo test --offline --manifest-path packages/Cargo.toml \
  -p windmill --test vitrine_sidecar --no-run
python3 scripts/vitrine-live-contract.py \
  /absolute/fresh-capabilities.json /absolute/vitrine_sidecar-TEST_HASH \
  /absolute/new-run-directory
```

The helper accepts only an explicitly supplied local development simulation,
retains Step's Board/operator authority pins, and builds its deployment file
from the running services' actual public keys and URLs. It runs both public
and private contracts, refuses an existing output directory and keeps
credentials out of its output. The public case exports the exact queued
entry's original issued receipt, a witnessed record, the signed policy,
acknowledgement and config/opened statement chains. The private case exports
no private policy or read secret.

After building the ballot verifier and installing its Playwright Chromium,
run its production receipt page against that public output:

```sh
node scripts/vitrine-browser.mjs packages/ballot-verifier/dist \
  /absolute/new-run-directory/public /absolute/browser-results
```

The browser runner checks a pasted chain and an uploaded receipt file at
1280 and 390 pixels, compares the offered policy ID with the fixture's
independently supplied ID before pinning, verifies the voter's own ballot,
rejects another ballot ID and a modified signed policy, and records
screenshots plus proof results. It checks full-page overflow, runs axe over
the receipt result, and refuses cookies, authorization or referrers on public
proof requests. `STEP_VITRINE_PLAYWRIGHT_MODULE` can select a matching pinned
Playwright installation in a browser container; the validated container uses
Playwright 1.64.1. Every browser process and its temporary static listener
shuts down when the script ends. Set `STEP_VITRINE_RECORD_VIDEO=1` to retain
actual browser clips for the public workflow.

The final package source also supplies deterministic full receipt-chain
regressions for WAICT signature normalization. To compare the installed Step
browser SDK with the compiled native verifier, build `@vitrine/verify-wasm`
in a Vitrine checkout whose production source matches the recorded
`packages/vendor-npm/VITRINE_COMMIT`, then run:

```sh
node scripts/vitrine-conformance-browser.mjs /absolute/vitrine-checkout \
  /absolute/new-conformance-results
```

This runner reads the ten shared chains from that exact Git revision, checks
the installed SDK in Node and actual Chromium, and writes a screenshot plus
the two backends' verdicts and WASM digest. Valid quorums survive extra bad
known-witness lines, including malformed-length/time payloads; insufficient
distinct valid signatures are refused. It makes no live-service request.
`STEP_VITRINE_PLAYWRIGHT_MODULE` selects a matching installed runner when
using the pinned browser container. Its static listener and browser shut
down on completion.

This rehearsal uses synthetic accepted ballot rows and real native
cryptographic receipts. It exercises the transactional queue and production
independent-verifier page; it does not deploy the authenticated Step,
Hasura/Harvest or Keycloak stack, or perform a complete encrypted voting
ceremony. The voting/admin component suites and type checks cover those
frontend contracts separately. Local party labels do not establish
organizational independence, and an answering monitor listener does not
establish that it subscribed to the newly created event.
