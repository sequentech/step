---
id: selftest_handover
title: Self-Test and Handover
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

This chapter describes the remaining human checks and the separate maintenance
procedures. Passing the installation checks alone does not complete the handover.

## Before the first import

:::warning
The TOE refuses every import of election data until the tenant's rules for
signed configuration packages are set. Set them before the first import.
Afterwards only a staff CA is replaced, between election events, as
[Expected results and recovery](../../02-election_managers/04-administrator_handbook/05-tenant_and_event_settings.md#expected-results-and-recovery)
describes.
:::

Before the election board imports any election data:

1. The administrator opens **Settings → Configuration packages** in the admin
   portal, uploads the public CA certificates that the election organizer approved,
   compares their fingerprints with the values the election organizer stated in
   writing and sets at least two approvals, following
   [Configure certificates in the admin portal](../../02-election_managers/04-administrator_handbook/05-tenant_and_event_settings.md#configure-certificates-in-the-admin-portal).
   A failed save or different fingerprint blocks
   the first import; do not substitute a different CA to make the form pass.
2. The administrator checks the roles, sets up the personal accounts and
   deletes the seeded account `admin`, following
   [Users, Roles and Role Separation](../../02-election_managers/04-administrator_handbook/04-users_and_roles.md).
   The seeded administrator groups hold none of the election
   board's permissions ([Configuring the Deployment](07-hardening.md#configure-the-realms)),
   and at sign-in the TOE gives an administrator account only the permissions
   of the administrator role. Only the election board's role holds "Create
   Election Event" (`election-event-create`), with which a board member imports
   election data with the menu entry "Import Election Event" and a signed configuration
   package, and the permissions to publish configuration versions and to read
   the electoral log.
3. No role holds "Edit Results Publication" (`publish-results-write`), no
   account holds "Datafix account" (`datafix-account`), and no role and no
   group holds "Manually Verify Voter" (`voter-manually-verify`). The import
   refuses a package that enables the results website.

## For each new election event

:::warning
An election event's realm is created at its import. Voters can sign in to it
only after its redirect address is registered, and it must be checked before
voters use it.
:::

Each time the election board has imported an election event, in its
preparation phase:

1. From the administration workstation's `deployment` directory, run
   `./deploy inspect site.json platform` as shown in
   [Election event realms](10-verification.md#election-event-realms). Copy the
   realm's complete name from the line `EVENT REALM:`. Ask the external provider's operator
   to register `https://HOST/realms/REALM/broker/ALIAS/endpoint`, replacing **HOST**
   with `hosts.voting` from `site.json`, **REALM** with that complete printed name,
   and **ALIAS** with `idp_alias`. Register the exact resulting URI, with no wildcard,
   following [Connecting the Authentication Server](06-authentication_server.md).
2. The operator makes the checks of
   [Election event realms](10-verification.md#election-event-realms) and
   tells the administrators the result.
3. The administrator follows
   [Authentication and Session Settings](../../02-election_managers/04-administrator_handbook/08-authentication_and_sessions.md)
   for the imported election event.
4. In the tab "Security and self-tests" of the election event, the administrator
   sets the interval, suite timeout and additional conditions during the
   preparation phase, and verifies the saved values against the electoral regulation.
5. A board member selects the purpose **On request**, then the button **Run
   self-tests**. Wait for all six results and check their details. If the run does
   not finish, the operator checks the services as
   [Read the current server state](#read-the-current-server-state) describes.
   Before opening or resuming voting,
   repeat with the required **Before starting voting** or **Before resuming
   voting** purpose. A passing run does not itself open or resume voting.
6. The operator repeats the checks of
   [Confirm the server settings to the administrator](#confirm-the-server-settings-to-the-administrator)
   and tells the administrators the result, before the election board approves the
   configuration version of the election event.

## Confirm the server settings to the administrator

The section "TOE software integrity" of the tab "Security and self-tests" of the
election event compares the six server TOE parts with the accepted delivery
([Verify the TOE software](../../02-election_managers/04-administrator_handbook/07-self_tests.md#verify-the-toe-software)),
but it does not cover the election verifier image or the content store of C2. The
admin portal shows no plugins, no telephone provider and no other tenants. The
operator therefore checks these settings and tells the administrators that the
checks passed, after the installation and again for each new election event, before
the election board approves its configuration version. If the operator and the
administrator are one person, no message is needed. To confirm the server settings:

1. Make the checks of
   [Repeat the complete TSF software integrity check](10-verification.md#repeat-the-complete-tsf-software-integrity-check),
   of [Data correction, telephone channel, plugins and tenants](10-verification.md#data-correction-telephone-channel-plugins-and-tenants)
   and of [No smart-link sign-in](10-verification.md#no-smart-link-sign-in).
2. If every check gave the result stated there, tell the administrators, through a
   channel that protects its integrity, that the checks passed, and give them the
   SHA-256 of `image-digests.txt` (`toe.manifest_sha256` of `site.json`), which they
   compare with the value "SHA-256 of the accepted digest list" that the admin portal
   shows. The checks establish that:
   - the running images are those of the accepted TOE delivery: the complete
     TSF software integrity check gave its four `PASS` lines;
   - no plugin is installed;
   - no telephone provider and no credentials for the telephone channel's
     endpoints are configured;
   - election work uses only the customer's tenant: the TOE has no tenant
     other than the customer's and the super-admin tenant, which serves only
     the installation steps of this guide, holds no election event and has
     no enabled personal account;
   - no account, group or role holds "Datafix account" (`datafix-account`)
     or "Manually Verify Voter" (`voter-manually-verify`);
   - the identity service has no smart-link sign-in: none of its extension
     files has `smart-link-hmac` in its name, and the smart-link sign-in
     address answers `404`.
3. If a check gives another result, stop. Tell the administrators that the
   election board must not approve the configuration version, and report the result
   to [security@sequentech.io](mailto:security@sequentech.io).

## Operating the servers

**Where:** use a root terminal in the extracted `deployment` directory on the
administration workstation. Keep the accepted `site.json` beside `./deploy`.
The helper uses its recorded C1 address and SSH identity; no separate server
shell or copied connection variables are needed. Unknown or changed SSH host
keys stop the operation. Verify the server identity as
[Verify SSH host keys before first use](03-installation_kit.md#verify-ssh-host-keys-before-first-use)
describes before proceeding.

### Read the current server state

```bash
./deploy services site.json status
```

**Expected result:** the recorded C1 hostname, both nodes marked `Ready`,
application pods marked `Running` and completed setup jobs marked `Completed`.
The final table shows the data volume's `Use%` and `Avail` columns. The command
does not report the host observer of [TOE image observation](#toe-image-observation).

![Actual terminal running the short status command against both laboratory VMs](./assets/installation/server-operations.png)

To read the identity service's most recent 20 operating-log lines:

```bash
./deploy services site.json logs keycloak
```

These are operating logs, not audit records: the election board reviews the
electoral log in the admin portal.

![Actual terminal reading the identity service log with the short maintenance command](./assets/installation/server-logs.png)

**If different:** preserve the failed component's error, investigate connectivity,
storage and its controlled configuration, and inform the administrator and the
election board. Do not delete records, change a Secret or edit a running container
to recover it. Check the tab "Security and self-tests" of the election event with
the election board. Stopping or restarting Harvest
or Windmill, an expired service heartbeat, a failed current check or an expired
test prevents voter admission. A successful service rollout does not clear the
state **Recovery required** or authorize resumption.

### Restart an authorized failed component

A restart interrupts service. During the execution phase, obtain the election
board's decision first
([A crash or shutdown of a server component](../../02-election_managers/04-administrator_handbook/10-modes_of_operation.md#a-crash-or-shutdown-of-a-server-component)).
If Keycloak is the agreed failed component:

```bash
./deploy services site.json restart keycloak
```

Type `RESTART keycloak` when prompted. Expect `COMPONENT READY: keycloak`, followed
by the reminder to complete platform and browser checks. The helper waits up to
45 seconds for that deployment. On timeout or failure, inspect its log; do not
restart unrelated services or bypass an active maintenance operation. The same
command, with the same confirmation, restarts any other of these components:
`postgres`, `rustfs`, `trustee1`, `trustee2`, `harvest`, `windmill`, `beat`, `b4`,
`hasura`, `admin-portal`, `voting-portal` or `ballot-verifier`.

Then run `./deploy inspect site.json platform` and complete the browser sign-in
check before telling the administrator that the component is back; the administrator
then makes the check of
[Verify the TOE software](../../02-election_managers/04-administrator_handbook/07-self_tests.md#verify-the-toe-software).
The election board retains all decisions on voting and audit review.

For time problems, use [Authenticated time](04-hosts_network_time.md#authenticated-time)
and its exact check. For certificate renewal, see
[Set up the TLS endpoint](05-certificates_tls.md#set-up-the-tls-endpoint). Change the
operating system, the cluster software and the TOE only with a new delivery or
installation kit from Sequent, outside the execution phase. The fresh-install command
refuses an occupied cluster, so obtain the steps for an existing installation from
Sequent. Never replace credentials or install an unaccepted image.

### Storage

Use `./deploy services site.json status` and read the final table's `Use%` and
`Avail` columns. Check daily during the preparation and execution phases and arrange
expansion before free space becomes insufficient.

The evaluated installation requires the automatic audit-storage monitor.
Above 80% use, or without a fresh trustworthy observation, voter token issuance,
refresh and voter operations are refused. Board and administrator access
remains available for recovery. Inspect the security state and notification
delivery; restoring capacity may also require the board's protected resumption.

If capacity is low, inform the administrator and the election board immediately.
The election board decides whether to pause operations while the operator expands
the storage. Never delete records, logs or arbitrary files to make space.
Volume/device expansion is installation-specific. Follow the site's host or storage
procedure, then repeat the capacity and platform checks.

### Request activity monitoring

`./deploy services site.json status` also reports `REQUEST MONITORING: available`
when the protected external-request observation is healthy and no more than
30 seconds old. The host counts completed external HTTP requests, including
unauthenticated refusals. Its raw observations omit client addresses, URLs,
queries, request bodies and credentials. The administrator configures detection
thresholds in the portal; the election board reviews the resulting detections.

**If unavailable:** inform the administrator and the election board. Voting remains
blocked and the election event enters maintenance mode (on screen, the state
**Recovery required**); staff access stays available.
An unavailable, stale, truncated or discontinuous source must never be treated
as zero activity. A successful Nginx restart alone does not repair a lost cursor
or missing observations.

Preserve the incident evidence and investigate the host, storage and collector.
Restore only verified continuous observation state. Never delete the collector's
state, change its generation, reseed counters or remove a failure marker to force
a passing result. If continuity cannot be established, keep voting stopped and
escalate the incident to the board and Sequent. After a verified repair, the board
still requires fresh passing self-tests and its protected recovery action.

The raw observation buffer is bounded. Overflow records a persistent continuity
failure before reclaiming raw transport storage; retained attack detections and
signed records of the electoral log are preserved. This failure remains visible after
a service or host restart.

### TOE image observation

The section "TOE software integrity" of the tab "Security and self-tests" of the
election event ([Verify the TOE software](../../02-election_managers/04-administrator_handbook/07-self_tests.md#verify-the-toe-software))
compares the six server TOE parts with the accepted delivery. It relies on a host
observer on C1, the units `sequent-image-status.service` and
`sequent-image-status.timer`. They come with the host configuration of the
installation kit, so there is nothing to install. The observer runs 5 seconds after
boot and then every 60 seconds, each run bounded at 90 seconds and at low priority.
It reads the file `/root/deploy/image-digests.txt`, which the installer copies
there, and reads the running containers and the content store through `k3s kubectl`
and `k3s ctr`. It publishes the file `/run/sequent-images/status.json`, which the
Windmill pod mounts read-only. `./deploy services site.json status` does not report
the observer. To read its state, connect to C1 as
[Verify SSH host keys before first use](03-installation_kit.md#verify-ssh-host-keys-before-first-use)
shows and run `systemctl status sequent-image-status.timer sequent-image-status.service`.

**If the admin portal shows the result "Differs from the accepted delivery":** keep
voting unavailable and run
[Repeat the complete TSF software integrity check](10-verification.md#repeat-the-complete-tsf-software-integrity-check).
Tell the administrators and the election board its result. The election event stays
in maintenance mode until the election board resumes it
([Maintenance mode](../../02-election_managers/04-administrator_handbook/10-modes_of_operation.md#maintenance-mode)).

**If it shows the result "Could not verify":** the security state does not change,
and the section gives the reason. For a single TOE part the overall reason is "Not
every TOE part could be verified", and the row of that part gives the detail.
Correct the cause, then tell the administrators to verify again:

| Reason shown | Cause and what to do |
|---|---|
| "The host image observation is stale" | The observation is older than 180 seconds. A run that reaches its 90-second limit publishes nothing. Check the two units on C1. |
| "The host image observation is dated in the future" | The observation carries a time later than the check's. Check [Authenticated time](04-hosts_network_time.md#authenticated-time). |
| "The host image observation is unavailable or invalid" | The Windmill pod cannot read a valid `/run/sequent-images/status.json`. Check the two units and the file on C1. |
| "The host image observation does not cover exactly the six TOE parts" | The published file does not describe the six parts of this release. Run [Repeat the complete TSF software integrity check](10-verification.md#repeat-the-complete-tsf-software-integrity-check) and ask Sequent. |
| "The accepted digest list cannot be read" | The file `/root/deploy/image-digests.txt` on C1 is missing or invalid. Never edit it. Compare its SHA-256 with `toe.manifest_sha256` of `site.json`, and ask Sequent for the steps if the file is missing or differs. |
| "The running containers cannot be read" | `k3s kubectl` failed on C1. Follow [Read the current server state](#read-the-current-server-state). |
| A message such as "1 of the 2 required containers are ready" | Too few containers of that part are ready: the check requires 2 for braid and Windmill and 1 for each other part. Follow [Read the current server state](#read-the-current-server-state) and [Restart an authorized failed component](#restart-an-authorized-failed-component). |
| "The accepted image could not be resolved", "The image of a running container could not be read", or another message about an image or the content store | The observer could not follow an image to its content. Run [Repeat the complete TSF software integrity check](10-verification.md#repeat-the-complete-tsf-software-integrity-check) and ask Sequent. |
| "The check could not be queued" or "The electoral-log record of this check could not be verified" | The check did not complete in the TOE's own services. Follow [Read the current server state](#read-the-current-server-state). |

### Backups and restore

A system backup contains election data, election execution data, audit records and the
working deployment directory with its secrets. The helper encrypts it before
writing it to the workstation or backup medium. Keep the passphrase in a
separate locked place. The archive does **not** contain the cluster's own state
or its imported container-image store.

These commands operate on an **existing accepted cluster and the same software
release**. They restore data and the working deployment directory. They do not
cover replacement nodes, lost Kubernetes Secrets or changed software releases:
obtain the procedure for those from Sequent.

#### Create and verify the encrypted backup

**When:** after commissioning, before the execution phase, or after voting has ended and
the election board authorizes maintenance.
A backup briefly stops the services. Coordinate the outage with the administrator
and the election board; do not use it in the execution phase. Mount the protected
backup medium and keep the accepted delivery inputs available.

Run this **one command**. It stops the services, creates and checks the encrypted
archive, then restores their original replica counts:

<!-- deployment-step: 11-002 -->

```bash
./deploy backup site.json
```

The helper asks for:

1. The existing, absolute path to the mounted backup directory.
2. A new passphrase of at least 16 characters, entered twice without displaying it.
3. The exact confirmation `BACKUP WITH SERVICE OUTAGE`.

No service is stopped until these prompts have succeeded. The helper records
replica counts, stops the applications, records the final electoral safety state,
and then stops PostgreSQL. It requires every application pod to stop before
reading the data. The encrypted archive is decrypted and checked without writing
plaintext to the workstation.

**Expected result, in order:**

- `APPLICATIONS STOPPED: backup may begin`.
- `BACKUP CONTENTS VERIFIED: system`.
- `READY:` for each deployment that originally had replicas.
- `RECORDED REPLICA COUNTS RESTORED` and the reminder that voting recovery remains
  a board decision.
- `SYSTEM BACKUP COMPLETE:` with the archive's absolute path, followed by its
  `SHA256:` digest.

![Actual VM terminal creating, verifying and completing an encrypted system backup with the short command](./assets/installation/system-backup.png)

The passphrase stays hidden. The laboratory screenshot shows successful archive
verification and restoration of the original service counts; your archive path
and digest will differ.

Retain the encrypted archive and the protected file `.backup-receipt.json`, created
in the current directory, which holds the archive's path and digest and identifies
the current backup when a restore is requested. Complete `./deploy inspect site.json platform` and the
browser sign-in check before declaring service restored.

**If stopping fails:** no backup is accepted. An API error or timeout never counts
as a stopped service. Inspect the current state and preserve the helper's maintenance
record (`/var/lib/sequent-maintenance` on C1). **If transfer, encryption or validation fails:** services remain stopped;
keep the original data and last known-good backup. Do not accept a partial
archive. Validation rejects unsafe paths, links, special files, duplicate entries,
missing directories and truncated data.

#### Stop services separately for maintenance

The backup command above already stops services. Use this separate command only
for an authorized maintenance outage:

<!-- deployment-step: 11-001 -->

```bash
./deploy services site.json stop
```

Type `STOP SERVICES FOR MAINTENANCE`. Expect
`APPLICATIONS STOPPED: backup may begin`. The helper saves the original replica
counts and allows up to 30 seconds for application shutdown, then up to 30 seconds
for the database and remaining pods. A system or trustee maintenance operation
already in progress prevents another operation from starting.

![Actual VM terminal stopping services with the short maintenance command](./assets/installation/system-stop.png)

**If it fails:** retain the maintenance record and inspect the actual service
state. Some services may already be stopped. Do not manually remove the record,
guess replica counts or start the normal backup command over an unfinished stop.

#### Resume unchanged services after a failed backup or separate stop

Use this only when the original data has not been replaced or changed by recovery
work. It also recovers an interrupted service stop. Never use it after restoration
has begun: the helper refuses a durable `restoring` state.

```bash
./deploy services site.json resume-unchanged
```

Type `RESUME UNCHANGED SERVICES` after confirming that the original data is intact.
Expect `READY:` for each previously running deployment, then
`RECORDED REPLICA COUNTS RESTORED`. The helper restores the saved counts, including
services that had zero replicas; it does not assume every service should run.

![Actual VM terminal restoring the saved replica counts with resume-unchanged](./assets/installation/system-resume.png)

Then run `./deploy inspect site.json platform`, complete the browser sign-in check
and report the failed operation to the administrator. If a rollout fails or the
replica record is missing, keep the maintenance record and investigate. Successful
rollouts do not authorize voting or clear a security failure.

#### Restore an authorized backup on the existing cluster

A restore replaces current data with older data. The election board must
authorize the selected backup and the outage
([A restore from a backup](../../02-election_managers/05-election_organizer_handbook/12-modes-and-failures.md#a-restore-from-a-backup));
tell the administrators and the election board of every restore. First make a verified current-state
backup, and retain it separately from the older archive. From the same workstation
directory, use these commands in order:

<!-- deployment-step: 11-003 -->

```bash
./deploy backup site.json
./deploy restore site.json
./deploy inspect site.json platform
```

The restore asks for the absolute path to the **authorized older backup**, its
passphrase and the exact confirmation `RESTORE AUTHORIZED BACKUP`. It requires
the current-state backup receipt from the preceding 24 hours and verifies that
its separate encrypted archive still has the recorded digest.

Before stopping services or replacing data, the helper authenticates and validates
the complete older archive and checks its installation, release and image-manifest
identity. It keeps a private copy of the encrypted bytes so a changed removable
medium cannot exchange the archive after validation.

After services stop, the helper compares the archive's electoral safety record
with the final current record. Both must contain the same election events in the same
phases. A restore that would remove or resurrect an election event, change an electoral
phase, or undo an unrecoverable security failure is refused. In particular,
**ended voting cannot reopen**.

The generic restore also refuses an election event in the execution phase, even when
the older archive has the same phase: it could omit ballots or audit records accepted
after that backup. Recovery in the execution phase requires a separately reviewed
procedure that preserves all accepted records: ask Sequent for it. Keep services stopped for that review when
a safety check fails.

During replacement, the maintenance record remains outside the restored
directories. Only PostgreSQL starts first. The helper checks the actual restored
safety records, invalidates prior service admissions and successful test generations,
and records the restoration before starting other services. Each recoverable
election event requires current self-tests and the board's recovery decision.

**Expected result:** `BACKUP CONTENTS VERIFIED: system`,
`APPLICATIONS STOPPED: backup may begin`, the deployment `READY:` messages,
`RECORDED REPLICA COUNTS RESTORED`, and finally `RESTORE CHECKS COMPLETE` with the
reminder to inspect the platform and obtain current tests and board review.
The third command must also complete successfully. Identity-service startup may
produce a bounded read-only wait of under one minute; it does not bypass issuer,
certificate or policy failures.

![Actual VM terminal authenticating the selected backup and completing the authorized restore](./assets/installation/system-restore.png)

The restored system still requires current self-tests and the board's recovery
decision. Successful restoration does not reopen voting.

**If archive preflight fails:** no service or current data has been changed by the
restore. Check the archive and passphrase. **If a safety-state comparison fails
after stopping:** the original data remains intact but the services stay stopped;
have the board decide the next action. **If replacement, database checks or
recovery recording fails:** keep applications stopped, preserve both encrypted
backups and the maintenance record, and ask Sequent for a reviewed recovery.
Do not use `resume-unchanged` or manually start partly restored data.

After success, the administrator completes the browser checks. The board reviews
the restored electoral log and all six current self-tests. Original trustee keys
and restored protocol state must agree. Run
`./deploy inspect site.json integrity` to recheck all seven accepted software
parts. Recovery from an incident during the execution phase follows
[Maintenance mode](../../02-election_managers/04-administrator_handbook/10-modes_of_operation.md#maintenance-mode):
a fresh successful run with the purpose **Before resuming voting** and the election
board's protected resumption within the approved period. Never resume an unrecoverable
election event.
