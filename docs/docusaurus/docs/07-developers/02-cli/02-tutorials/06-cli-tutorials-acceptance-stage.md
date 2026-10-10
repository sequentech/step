---
id: cli_tutorials_acceptance_stage
title: Checking an Acceptance Stage with the CLI
position: 6
---

<!--
-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

`step-cli acceptance` checks one stage of an acceptance test, such as voting, against a live election event. It keeps an evidence ledger of every check and gives the stage a verdict: one failed critical check fails the stage.

Voters vote as they would in a real election, on their own devices and in front of witnesses. The CLI does not vote for them. It reads what the platform recorded and stores what the witnesses observed.

---

## Prerequisites

- The CLI is configured with a tenant administrator who can read voters, cast votes and the election event's log. See [Getting Started With the CLI](./01-cli-tutorials-getting-started.md).
- The election event is published and voting is open.

---

## Step 1: Write the stage definition

A stage definition is a YAML file that lists the checks of the stage. Start from a bundled template:

```bash
step-cli acceptance init --template voting --output voting-stage.yaml
```

| Template | Checks |
| --- | --- |
| `voting` | The voting conditions: voters authenticate, a ballot is published and displayed, the ballot is cast, a confirmation or receipt is produced, and the cast ballot is stored. |
| `pqri-voting` | The same checks, plus one witnessed check for each condition of Annex A D.4 of the post-qualification demonstration. |

Edit the file to describe your own stage. Each check has:

| Field | Meaning |
| --- | --- |
| `id` | Name of the check on the command line and in the ledger. |
| `title` | What must hold. |
| `source` | Optional. The clause or document the check comes from. |
| `severity` | `critical` (default): a failure, or no result, keeps the stage from passing. `advisory`: a failure is reported and does not change the verdict. |
| `method` | `automatic`: the CLI decides it from the election event's records. `witnessed`: a person observes it and the CLI stores the result. |
| `probe` | For automatic checks only: what the CLI reads. See the table below. |

Unknown fields and values are rejected, so a misspelled severity cannot relax a check.

### Automatic checks

| Probe | Passes when |
| --- | --- |
| `voter-authenticated` | Each voter exists in the election event, is enabled and has a successful sign-in in the event's log. |
| `ballot-published` | A ballot style is published for each voter's area. |
| `ballot-cast` | Each voter has a valid cast vote. |
| `receipt-produced` | Each Ballot ID read from a voter's confirmation screen or receipt is a stored ballot of the checked voters, and every cast ballot of those voters has its Ballot ID presented. |
| `ballot-stored` | Each cast ballot's stored content can be read, and its hash equals the hash the ballot box recorded in the event's log for that Ballot ID. |

Only records made after the run started count, so sign-ins and votes from an earlier rehearsal are not evidence.

That the ballot is displayed is a witnessed check: the server cannot see a voter's screen.

---

## Step 2: Open the ledger

```bash
step-cli acceptance open voting-stage.yaml \
  --ledger voting-ledger.jsonl \
  --election-event-id <ELECTION_EVENT_ID> \
  --release <RELEASE>
```

- `--ledger`: the file to create. An existing file is never reused.
- `--started-at`: optional start of the run, as a date and time with its UTC offset (for example `2028-01-10T09:00:00+08:00`). It defaults to now. Open the ledger before the voters start.
- `--release`: optional identifier of the release or build under test.
- `--recorder`: optional name of who records. It defaults to the configured administrator.

The first ledger entry holds the stage definition and its SHA-256, the tenant, the election event, the start time and the release. Later changes to the YAML file do not change the run.

Every command prints the **ledger head**, the hash of the last entry. Witnesses can note it down: it changes if anything recorded before it changes.

---

## Step 3: Run the automatic checks

After the voters have voted, name them and give the Ballot IDs they were shown:

```bash
step-cli acceptance run voting-ledger.jsonl \
  --voters-file voters.txt \
  --ballot-ids-file ballot-ids.txt
```

- `--voter <USERNAME>` or `--voters-file <FILE>` (one username per line): the voters who took part.
- `--ballot-id <ID>` or `--ballot-ids-file <FILE>` (one per line): the Ballot IDs read from the voters' confirmation screens or receipts. Without them the receipt check is not run and stays without a result.
- `--check <ID>`: run only this check. Repeat it for several.

The command prints the result of each check and exits nonzero if one failed. A check can be run again, for example after a typing mistake in a Ballot ID: the last result counts and both stay in the ledger.

---

## Step 4: Record the witnessed checks

```bash
step-cli acceptance record voting-ledger.jsonl \
  --check voting.ballot-displayed \
  --outcome pass \
  --witness "<WITNESS NAME>" \
  --note "Ballot shown on a phone and a laptop after signing in" \
  --evidence photo.jpg
```

- `--outcome`: `pass` or `fail`.
- `--witness`: who observed it.
- `--note`: optional description of what was observed.
- `--evidence`: optional file kept as evidence. The ledger stores its name and SHA-256, not the file. Repeat it for several files.

---

## Step 5: Get the verdict

```bash
step-cli acceptance verdict voting-ledger.jsonl --report voting-report.md
```

The command verifies the ledger, prints one line per check and the verdict, and exits nonzero unless the stage passed:

| Verdict | Meaning |
| --- | --- |
| `passed` | Every critical check passed. |
| `failed` | At least one critical check failed. |
| `incomplete` | No critical check failed, and at least one has no result. |

`--report` writes a Markdown report with the run's details, the result of each check, who recorded it and the findings.

---

## The ledger

The ledger is a text file with one JSON entry per line. Each entry holds its number, the time, who recorded it, the check, the outcome, the findings and the hash of the previous entry. If an entry is edited, removed or moved, every command that reads the ledger stops and names the entry.

Entries cannot be removed from the end without the ledger head changing, so keep the head the witnesses noted with the report.

### Voter secrecy

A finding about a voter names the voter and never a Ballot ID. A finding about a ballot gives the Ballot ID and never the voter. Ballot contents stay encrypted and are never written to the ledger.

The ledger does list which voters were checked and which Ballot IDs were presented in the same run. With very few voters these can be related to each other, so use it with test voters, or check enough voters at once.
