---
id: installing_toe
title: Installing the TOE
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

This chapter covers the one-command platform installation, the browser checkpoint
for creating the customer, and the guided trustee-key import. Later backup and
retirement procedures are separate maintenance actions, not installation steps.

:::warning
Use only the TOE delivery accepted through
[Verifying the Delivery](../08-delivery_verification.md). The installation kit's
supporting images do not replace the delivered TOE images.
:::

## Deploy

**Where:** a root terminal on the administration workstation, in the deployment
directory. Keep the accepted delivery medium mounted read-only and
use the checked `site.json` of [Install from one configuration file](01-about.md#install-from-one-configuration-file).
After `./deploy check site.json` passes, run:

```bash
./deploy install site.json
```

<!-- deployment-step: 08-005 -->

**Expected result.** Four numbered installation stages appear. The final line is
`INSTALL 4/4: platform installed; customer and trustee commissioning still required`.
The installer loads the protected credentials in its own server-side process;
you do not type a separate deployment command or source `secrets.env` yourself.

This stage starts the TOE parts, all on C1, with network policies in the cluster
namespaces `step-apps` and `step-infra`: they refuse, in both directions, every
connection between pods that the parts do not need. The TLS endpoint and the
ingress run on C1's own network and are outside these policies. The stage also
creates the administration API client `step-admin-api` in the master realm, with
which Harvest and Windmill manage the realms, and disables the account clients
of the master realm.

![Actual VM terminal showing the configuration-driven installation completing](./assets/installation/installer-complete.png)

*Terminal capture from a test installation. Your addresses, names and digests differ.*

**If it stops:** keep the reported stage and the original credentials, and follow
[If something fails](01-about.md#if-something-fails). Do not regenerate secrets, replace a digest with the tag `latest`, or
rerun the fresh installation as a recovery operation.

## Load the TOE images

<!-- deployment-step: 08-002 -->

**Automatic part of the command above.** The installer validates the eleven
accepted OCI image directories, prepares private transfer archives for the ten
server images, transfers them to C1 and C2, and checks their hashes and imported
digests. It prints `PASS: c1 delivery archive hashes and imported image digests`
and the corresponding line for `c2`.

You do not create tar archives or type a separate image-import command. Keep the
accepted delivery unchanged and the imported content cache available for later
verification. Either server failing its import blocks installation.

## Pin the TOE images

<!-- deployment-step: 08-003 -->

**Automatic part of the command above.** Every TOE container reference is pinned
to its accepted `@sha256:` digest; the two native trustee instances run the
delivered `braid` image. The two remaining `trustee:latest` references name the
kit's supporting setup image, which is not part of the TOE: the init container
`create-bucket` of the `b4` deployment, which runs at each start of `b4`, and the
job that creates the object storage's buckets. The storage check of the platform
inspection also runs it. The installer checks that image on both servers against
the accepted kit before it installs anything. The kit's other images, such as
those of the database and the object storage, are referenced by name and tag.
The servers reach no image registry, so each of these names resolves only to the
image that the accepted kit installed on that server.
The later [Running images](10-verification.md#running-images) check verifies
actual running content rather than relying only on a manifest's text.

## Remove the secrets from the realm manifest

<!-- deployment-step: 08-006 -->

**Automatic part of the command above.** After the initial realm has imported,
the installer removes its imported credentials from the stored manifest and
checks the resulting cluster Secret. The running realm retains the values it
imported. Expect
`PASS: imported realm credentials removed from its stored manifest and verified in the cluster`.

If the realm is unavailable or cleanup fails, stop and follow
[If something fails](01-about.md#if-something-fails). Do not print secret-bearing documents, recreate the realm or repeat
secret generation.

## Create the customer's tenant

:::warning
All election work happens in the customer's tenant. The super-admin tenant serves
only the steps of this guide, holds no election event, and none of its personal
accounts stays enabled.
:::

The operator creates the customer's tenant once, after [Deploy](#deploy) and
before the trustee keys are installed. `ADMIN_HOST` below means the value of
`hosts.administration` in your `site.json`; replace that label in the browser URL:

1. Open `https://ADMIN_HOST` on the administration workstation and sign in at
   the external authentication server as the installation account of
   [Add the installation account](07-hardening.md#add-the-installation-account).
2. In the menu, next to the tenant's name, select the plus button "Create new
   tenant". Enter the customer's short name in the field "Slug" and select
   the button "Save". The TOE creates the tenant and its realm from the tenant template
   that [Configure the realms](07-hardening.md#configure-the-realms) has
   hardened, with the service accounts and a seeded `admin` account without a
   password.

   The task **Create Tenant** can remain **IN_PROGRESS** while the customer is
   provisioned. Wait for it to finish and confirm that the agreed customer slug
   exists before commissioning; do not submit another creation request while this
   task is running.

   ![Portal showing the customer-creation task in progress](./assets/installation/customer-creation-in-progress.png)

   *Admin portal capture from a local build with test data. The displayed
   installation account and site values are examples.*

3. Put the customer slug and administrator identity in `commissioning` in
   `site.json`, then run `./deploy commission site.json` from the administration
   workstation. The command resolves the customer tenant on C1.

   **Required result.** The lookup identifies exactly the super-admin tenant and the customer's tenant matching the configured slug. The customer realm is `tenant-` followed by that customer UUID.

   **If the result differs.** If the message is `Create exactly one customer tenant in the portal before commissioning` or `Configured customer slug is absent or ambiguous; create/select it in the portal`, check in the portal that the customer's tenant exists with the slug of `site.json`, finish step 2 if it does not, and run the command again. If more than one customer tenant exists, stop and ask Sequent; never choose an arbitrary tenant or repeat creation.


   The super-admin tenant is `90505c8a-23a9-4cdf-a26b-4e19f6a097d5`; the
   other tenant must have the customer slug you supplied. The commissioning
   command records the resolved customer realm. Do not type or guess a UUID or
   edit a shell environment file to select a different tenant.
4. At the commissioning checkpoint, copy the complete URI printed after
   `Register this exact customer redirect URI`. Have the authentication server's
   administrator register that exact address, following
   [Connecting the Authentication Server](06-authentication_server.md). Type
   `CONTINUE` only after registration and the external administrator account
   are ready; do not construct a URI by guessing the customer identifier.
5. The commissioning command adds the customer's first administrator and the
   two native trustee accounts, configures the trustee client secret, and disables
   the personal accounts of the super-admin tenant. The administrator's user name
   and e-mail must match the identity supplied by the external authentication
   server; they come from the same configuration file.

   <!-- deployment-step: 08-008 -->

   **Expected result.** The terminal prints `PASS: customer administrator and native accounts configured; installation personal accounts disabled`. The next prompt asks you to register the two trustees in the customer portal.

   **If the result differs.** If account creation reports a conflict or a user remains enabled in the super-admin tenant, stop and ask Sequent. Never give these accounts local passwords.


   The installer checks the account creation and disables the personal accounts of
   the super-admin tenant. Native service accounts stay enabled. You do not receive
   or type a service password, and no later step uses the installation account.
   The exact prompts and success messages are shown in the screenshot of
   [Transfer the matching keys and start the instances](#transfer-the-matching-keys-and-start-the-instances).
6. Continue with [Install the trustee keys](#install-the-trustee-keys) below in
   the same commissioning terminal. Carry out
   [Run the platform inspection](10-verification.md#run-the-platform-inspection) only
   after both trustees are ready and commissioning completes.

## Install the trustee keys

Each trustee generates their keys on a trustee workstation (see
[Preparing a trustee workstation](#preparing-a-trustee-workstation) and
[Trustee Procedures](../../02-election_managers/05-election_organizer_handbook/09-trustee-procedures.md)),
and the administrators register the public keys as
[Registering the trustees' public keys](../../02-election_managers/04-administrator_handbook/04-users_and_roles.md#registering-the-trustees-public-keys)
describes. The trustee then hands over the keys file for their instance of
the trustee server: `trustee1` for the first trustee, `trustee2` for the
second.

### Bind each native account to its registered trustee

Before starting either instance, agree with the administrator which
registered trustee and public keys belong to each instance. The machine login
names remain `trustee1` and `trustee2`; they can differ from the names registered
in the tab "TRUSTEES" of Settings. Each machine account's `trustee` attribute must name
exactly its registered trustee. The trustee's personal account names the same
registered trustee in its field "Act as Trustee".

The commissioning configuration associates the first trustee entry with native
instance `trustee1` and the second with `trustee2`. At its checkpoint, the
administrator registers the two public keys in the portal and each trustee
provides the corresponding private key file on their own medium. Before importing
either file, the command compares its public-key pair with the registered trustee
record. It changes only the native account's trustee binding; personal trustee
accounts remain separate and receive no local password.

The administrator opens each registered trustee record and checks both public keys
against that trustee's own keys before you continue.

![Customer portal listing two registered example trustees](./assets/installation/trustees-registered.png)

*Admin portal capture from a local build with test data. The customer and trustee
names shown here are examples.*

<!-- deployment-step: 08-009 -->

**Required result.** Each native account is bound to its configured registered trustee name, and the installed key file matches that record’s public keys. Both instances must pass.

**If the result differs.** If this step stops, check the registered name, the customer tenant and the instance number with the administrator and the trustee. Do not continue with an uncertain key-to-account mapping.


The binding is persistent customer-realm state. The environment variable
`TRUSTEE_NAME` of each instance selects its native login, so it stays `trustee1` or
`trustee2`; that name does not replace verification of the trustee's signing public
key.

### Transfer the matching keys and start the instances

:::warning
The keys file holds the trustee's private keys. Take it over only in person,
with the trustee watching every step, directly from the trustee's own medium.
No copy may remain on the administration workstation or on the disk of C1. The
installer's transfer copy stays in C1's memory and is removed after the import;
on the servers the keys file is then kept only in the cluster's encrypted Secret.
The trustee keeps the original on their own medium.
:::

**Where to work:** the administration workstation—the computer from which you
connect to C1 and C2 over SSH. This is not the admin portal, a user named `admin`,
or a separate trustee server. To transfer the matching keys and start the
instances:

1. Complete [Create the customer's tenant](#create-the-customers-tenant)
   and register both trustees as described above. In `site.json`, each
   `commissioning.trustees` entry must contain the exact registered `name` and
   the absolute `keys_file` path on that trustee's mounted medium. Each instance's
   `keys_file` is the trustee's `working_directory` followed by `/trustee.toml`, as
   `/media/trustee-one/session/trustee.toml`.
2. Open a root terminal in the deployment directory. If
   commissioning is not already running, start it with the same configuration:

   ```bash
   ./deploy commission site.json
   ```

   If that command is already waiting for you, continue in that terminal;
   do not start a second copy. `./deploy` is the executable in the current
   directory. From another directory, use its full path and the full path to
   `site.json`.
3. At the trustee-registration prompt, have the administrator verify the names and
   both public keys in the portal, as
   [Bind each native account to its registered trustee](#bind-each-native-account-to-its-registered-trustee)
   shows.
   Type `CONTINUE` and press Enter only after completing that check.
4. At each `Trustee 1: be present...` / `Trustee 2: be present...` prompt,
   have that trustee confirm their own mounted medium and private-key file.
   Type `CONTINUE` and press Enter. Keep both media available, unchanged, and
   under their owners' supervision until both imports finish: the installer
   checks both key pairs before importing either, then reads each file again.
5. Let the command transfer the keys, create the encrypted cluster Secrets,
   bind the native accounts and start both instances. These are automatic
   operations; no additional command on C1 is required.

**Expected result.** First the terminal prints
`PASS: both trustee media match their registered public-key pairs before either import`.
After each successful import, it prints
`PASS: trustee 1 public-key pair, native binding and startup; transfer copy removed`
and the corresponding line for trustee 2. Only then unmount each trustee's
medium and return it to its owner. These messages confirm removal of the
installer's temporary transfer copies; do not make additional copies yourself.

![Actual VM commissioning terminal showing the trustee prompts, both key checks and successful startup](./assets/installation/installer-commission-complete.png)

*Terminal capture from a test installation. Your addresses, names and digests differ.*

**If the result differs:**

- **Keys do not match:** stop. With the trustee, check the configured file,
  registered name, tenant and both registered public keys. Do not replace keys
  or alter the registration simply to silence a mismatch.
- **Transfer, Secret creation or startup fails:** stop and follow
  [If something fails](01-about.md#if-something-fails). Do not overwrite an existing Secret, generate replacement keys or assume
  that temporary-file cleanup succeeded.
- **A previous attempt stopped during writes:** the installer refuses automatic
  replay. Follow [If something fails](01-about.md#if-something-fails); do not remove its state markers to bypass this refusal.

<!-- deployment-step: 08-011 -->

The same commissioning command configures each native instance's sign-in and
waits for its deployment to become ready. Do not change the names or the key-file
paths under `commissioning` in `site.json`: a later `./deploy commission site.json`
refuses a changed configuration. Personal trustee accounts remain separate from
these native machine accounts.

**Final expected result.** The terminal prints
`PASS: customer login restrictions and both native trustees Ready`, followed by
`COMMISSIONING COMMANDS COMPLETE — perform the browser, client-network and board handover checks in AGD-04.`
This is not authorization to start an election event: the checks of
[Handover to the election board](11-selftest_handover.md#handover-to-the-election-board) still follow.

Before each key or tally ceremony, check that both instances are ready, and tell
the election board the result. From the deployment directory, run `./deploy commission site.json`
with the unchanged completed configuration. On a completed installation it checks
readiness and login restrictions without importing the keys again. Expect the
`PASS: customer login restrictions and both native trustees Ready` line; the
first-run completion banner is not repeated. If the check fails, tell the election
board not to start the ceremony until you have resolved the fault and the check
passes.

### Show each trustee the protocol manager key

In each key ceremony, each trustee compares two values of the fingerprint
(SHA-256) of the ceremony's protocol manager key before confirming custody of
their key: the value that the election board reads in the admin portal and gives
them over a separate channel, and the value that their own instance logged when it
accepted the ceremony's configuration
([Trustee Procedures](../../02-election_managers/05-election_organizer_handbook/09-trustee-procedures.md)).
Trustees have no access to the cluster, so you read the log of their instance for
them while they watch. The commands only read the log.

**When:** after the election board has created the key ceremony and before the
trustee downloads the custody request. The instance logs the value within seconds
of the ceremony's creation.

**Where:** the administration workstation, in the deployment directory, with the
trustee watching the screen.

To show a trustee the value that their instance logged:

1. Ask the trustee for the board name that the election board gave them: `dkg_`
   followed by the ceremony's identifier without dashes. Their instance is
   `trustee1` for the first trustee and `trustee2` for the second.
2. Read the instance's log lines for that board. Put the board name and the
   instance in place of the examples:

   ```bash
   BOARD=dkg_0123456789abcdef0123456789abcdef   # the board name the trustee gives you
   INSTANCE=trustee1                            # or trustee2
   SERVER=$(jq -er '.nodes.c1' site.json)
   ADMIN_KEY=$(jq -er '.ssh_identity' site.json)
   ssh -i "$ADMIN_KEY" "root@$SERVER" "k3s kubectl -n step-infra logs deployment/$INSTANCE" \
     | grep 'protocol manager key SHA-256' | grep -F "$BOARD"
   ```

3. Let the trustee compare all 32 pairs of the logged value with the value that
   the election board gave them, not only the first and last pairs.
4. Repeat steps 1 to 3 with the other trustee and their instance.

**Expected result.** At least one line with
`session opened: Configuration accepted, protocol manager key SHA-256`, followed
by the value, 32 pairs of upper-case hexadecimal digits separated by colons, and
by `board=` with the board name. Every line for that board shows the same value.
The time, the level and the field name `board` may appear in colour.

**If the result differs.** If no line names the board and the instance has
restarted since the ceremony was created (the column `RESTARTS` of
`./deploy services site.json status`), repeat step 2 with `logs --previous` in
place of `logs`: the line is then in the log of the instance's earlier container.
If the values differ, or no line names the board, the trustee does not create or
upload a custody proof for this ceremony and tells the election board, which does
not use the ceremony. Keep the instance's complete log: run the command of step 2
without the two `grep` filters and save its output to a file.

## Back up the trustee state

**When:** after a key ceremony, with no ceremony running, and with the board and
trustees informed. The command briefly stops the two native trustee instances,
encrypts and verifies their state, then restores their original replica counts.

**Where:** the administration workstation, in the deployment directory.
Use the existing `site.json` and mount the encrypted backup medium. Run:

```sh
./deploy backup-trustees site.json
```

The command asks for the mounted backup directory and the encryption passphrase
privately. Enter the passphrase again when requested. It creates a new dated
backup; it does not overwrite an earlier one. Keep the encrypted backup and its
passphrase in separate locked places.

**Expected result**, in this order:

1. `TRUSTEES STOPPED: state backup may begin`.
2. `BACKUP CONTENTS VERIFIED: trustee`.
3. `TRUSTEE REPLICA COUNTS RESTORED`.
4. `TRUSTEE BACKUP COMPLETE:` followed by the encrypted file's path.
5. `Encrypted backup SHA-256:` followed by its SHA-256.

Tell the election board the path and the digest: it records them in the board's
minutes, and they name the backup to destroy at
[Remove the trustee keys at the end of the election event](#remove-the-trustee-keys-at-the-end-of-the-election-event).
The encrypted archive is accepted only after decryption and complete content validation succeed.
There is no command to restore this archive. If an instance's state is lost, tell
the election board, which postpones every key or tally ceremony that needs the
instance. Keep the archive and its digest unchanged, and ask Sequent
([If something fails](01-about.md#if-something-fails)).

![Actual terminal completing an encrypted trustee-state backup](./assets/installation/trustee-backup.png)

*Terminal capture from a test installation. Your addresses, names and digests differ.*

**If the result differs:** keep the original state and the last known-good
backup. Do not accept a partial archive. Failure to stop all trustee pods prevents
copying. Encryption or verification failure leaves the trustees stopped, so
inspect the reported error before retrying or resuming.

### Resume unchanged trustees after a failed backup

Use this only when the original state is intact and the board authorizes resuming
it. This does not restore damaged state or undo key removal. Run:

```sh
./deploy resume-trustees site.json
```

Read the warning, then type `RESUME UNCHANGED TRUSTEES`. Expect
`TRUSTEE REPLICA COUNTS RESTORED`. The command checks and uses the saved original
replica counts. If their record is missing or invalid, it stops: do not guess the
counts or remove markers manually, and tell the election board not to start another
ceremony.

## Remove the trustee keys at the end of the election event

:::warning
This destroys the keys and the session stores of both trustee server instances.
Use it only on the election board's recorded instruction, after the tally and the
export, with the board present. Afterwards no election event of this installation
can be tallied with these keys. It is not an installation or troubleshooting command.
:::

**When:** after the tally and the export of the election execution data
([Exporting the election execution data](../../02-election_managers/05-election_organizer_handbook/08-evaluation-and-post-processing.md#exporting-the-election-execution-data)),
once the election board has recorded in its minutes the instruction to destroy the
keys. The two instances serve every election event of the installation, so the
command refuses unless every election event is in the post-processing phase or
archived in the preparation phase.

**Where:** the administration workstation, in the deployment directory. Show the
board the two addresses under `nodes` in `site.json`, and have it confirm that these
are the servers of this installation. Then run:

```sh
./deploy remove-trustees site.json
```

Read the warning and type `REMOVE TRUSTEE KEYS`. The command checks the election
events and the accepted trustee image, then stops both instances. For each instance
it runs `trustee destroy` on an in-memory copy of the instance's keys file and on its
session stores: it overwrites them with zeros, removes them and prints the instance's
public keys. The command then deletes the four trustee Secrets, empties the two state
directories and waits until the cluster datastore has discarded the deleted Secrets.

**Expected result**, in this order:

1. `Checking that no election event needs the trustee keys, then destroying them with the session stores. This can take up to 13 minutes…`.
2. The output of `trustee destroy`, first for `trustee1`, then for `trustee2`, each
   line starting with the instance's name and a colon: the line that starts with
   `destroyed the keys file /run/destroy/trustee.toml`, the instance's two public
   keys (`public_key:` and `share_encryption_public_key:`) and, if the instance holds
   session stores, a line `destroyed the session store` for each of their files and
   `removed the session store directory /opt/trustee/message_store`.
3. `TRUSTEE REMOVAL COMPLETE: four Secrets absent; state directories empty`.
4. `Waiting for the cluster datastore to discard the deleted trustee Secrets. This can take up to 30 minutes…`.
5. `DATASTORE COMPACTED: no earlier revision of the trustee Secrets remains in the cluster datastore`.
6. `Before the next election event: each trustee generates a new key pair offline and an administrator registers both new public keys in the customer administration portal; then run ./deploy commission site.json with both trustees and their key media present.`

No trustee instance is restarted. The board compares the printed public keys with
those of the two registered trustee records, which an administrator opens in the
portal, and records the result in its minutes. The lines of step 2 also remain in
the file `/root/deploy/trustee-destroy.log` on C1, which holds no secret.

![Actual terminal confirming trustee retirement](./assets/installation/trustee-removal.png)

*Terminal capture from a test installation. Its warning and the lines before step 3
differ from the current output, and steps 4 to 6 are missing. Your addresses, names
and digests differ.*

**If the result differs:**

| The message starts with | Meaning | What to do |
| --- | --- | --- |
| `Stopped: Trustee key destruction failed: An election event can still use the trustee keys` | An election event is neither in the post-processing phase nor archived in the preparation phase. Nothing was changed. | Tell the election board. Run the command again once that election event is in the post-processing phase or archived in the preparation phase. |
| `Stopped: Trustee key destruction failed: The accepted trustee image does not provide trustee destroy` | The installed trustee image has no `trustee destroy`. Nothing was changed. | Ask Sequent. |
| `Stopped: Trustee key destruction failed: A trustee destroy run did not complete` | `trustee destroy` failed or did not finish for an instance. Both instances stay stopped and the Secrets are kept; the output is in `/root/deploy/trustee-destroy.log` on C1. | Keep the message and ask Sequent. Once the cause is resolved, run the command again: it continues the stopped run. |
| `Stopped: The cluster datastore still holds earlier revisions of the deleted trustee Secrets` | The keys and session stores are destroyed, but the cluster datastore has not discarded the deleted Secrets within 30 minutes. | Run the command again later: it prints `TRUSTEE KEYS ALREADY DESTROYED: four Secrets absent; state directories empty` and waits again. |
| Any other `Stopped: Trustee key destruction failed` or `Stopped: Cluster datastore compaction check failed` | Another maintenance operation is running or was interrupted, or a check or the connection to C1 failed. | Keep the message and tell the election board. Run the command again once the cause is resolved, or ask Sequent. |

Running the command again is always safe: it refuses again, continues a run that
stopped part way, or reports `TRUSTEE KEYS ALREADY DESTROYED: four Secrets absent;
state directories empty`. Until a stopped run is complete, both trustee instances
stay stopped, and the other maintenance commands refuse, except those that read the
server state and the logs.

While the outcome is uncertain, keep the remaining state and backups, tell the
election board what was removed, and tell it not to start another ceremony. After
confirmed server removal, the board directs the people who keep them to destroy
retained trustee-state backups and system backups
([Backups and restore](11-selftest_handover.md#backups-and-restore)), which also hold
the protocol state, and records their destruction in the board's minutes. Each
trustee then destroys their own keys file and each copy of it on the offline
trustee workstation with `./trustee destroy trustee.json`
([Trustee Procedures](../../02-election_managers/05-election_organizer_handbook/09-trustee-procedures.md)).

The command overwrites files through the file system and deletes the Secrets
through the cluster; it does not reach the storage medium. Remnants can remain on
C1's disks (for example on SSDs, on virtual disks, on storage with snapshots, and in
space that the cluster datastore has freed) and in snapshots of C1 made outside the
kit. Only purging or destroying the media removes them; the election organizer
decides what happens to the media.

### Install new trustee keys for a later election event

After the removal, the trustee instances have no keys, and a later election event
needs new ones.

**When:** after `./deploy remove-trustees site.json` has printed
`DATASTORE COMPACTED: …`, and before the key ceremony of the next election event.

**Where:** the administration workstation, in the deployment directory, with
both trustees and their key media present, as in
[Transfer the matching keys and start the instances](#transfer-the-matching-keys-and-start-the-instances).
Leave `site.json` unchanged: each `commissioning.trustees` entry keeps its name and
the path of its keys file on the trustee's medium.

To install the new keys:

1. Have each trustee generate a new key pair on the offline trustee workstation
   ([Generate keys in the trustee's private session](#generate-keys-in-the-trustees-private-session)).
2. Run the commissioning command:

   ```bash
   ./deploy commission site.json
   ```

3. The command first checks the installed platform, then prints
   `The trustee keys of the earlier election event were removed.` and asks for the
   registration of both trustees with their new public keys under the names in
   `site.json`. Have the administrator register them and check both public keys in
   the portal, as
   [Bind each native account to its registered trustee](#bind-each-native-account-to-its-registered-trustee)
   shows. Then type `CONTINUE` and press Enter.
4. At each `Trustee 1: be present...` / `Trustee 2: be present...` prompt, have
   that trustee confirm their own mounted medium and private-key file, then type
   `CONTINUE` and press Enter.

**Expected result**, in this order:

1. `PASS: both trustee media match their registered public-key pairs before either import`.
2. `PASS: trustee 1 public-key pair, native binding and startup; transfer copy removed`,
   then the same line for trustee 2.
3. `PASS: customer login restrictions and both native trustees Ready`.
4. `NEW TRUSTEE KEYS COMMISSIONED — both trustees run with their newly registered keys.`

Then unmount each trustee's medium and return it to its owner. A later
`./deploy commission site.json` only checks the instances again.

**If the result differs:**

| The message starts with | Meaning | What to do |
| --- | --- | --- |
| `Stopped: A trustee key removal stopped part way` | The removal did not finish. | Run `./deploy remove-trustees site.json` to complete it, then this command again. |
| `Stopped: The removed trustee keys are not yet confirmed gone from the cluster datastore` | The removal has not yet printed `DATASTORE COMPACTED`. | Run `./deploy remove-trustees site.json` again until it does, then this command again. |
| `Stopped: An import of new trustee keys stopped part way` | An earlier run of this command stopped during an import. | Tell the election board. Run `./deploy remove-trustees site.json`, which removes what that import installed, then this command again. |
| `Stopped: The configured customer differs from the commissioned one` | `site.json` names another customer. Nothing was changed. | Use the unchanged `site.json` of this installation. |
| `Stopped: Trustee private keys do not match the exact registered public-key pair` | A medium does not hold the keys registered for its trustee. Nothing was imported. | Proceed as for **Keys do not match** in [Transfer the matching keys and start the instances](#transfer-the-matching-keys-and-start-the-instances). |
| `Stopped: Trustee medium changed after the two-key precheck` | A medium changed during the import. | Keep the message and tell the election board. The next run of this command stops with the message of an import that stopped part way; proceed as that row says. |
| `Stopped: Commissioning paused; no automatic continuation` | Something other than `CONTINUE` was typed. Nothing was imported. | Run this command again when the step is complete. |

## Preparing a trustee workstation

**Who:** the trustee, with an operator preparing the computer.

Use the **`./trustee`** command of the deployment directory. It selects the native
trustee program from the accepted `braid` image and supplies the offline container
options automatically. No shell function, container command or script needs to
be typed.

Trustees may use one organizer-controlled computer in separate clean offline
sessions. Each keeps their own encrypted working medium, keys and backups. Follow
[Trustee key handling](02-operational_environment.md#trustee-key-handling): disable
network interfaces, swap, hibernation and session recording before handling keys.
The wrapper checks the network interfaces, swap, encrypted working medium and
file permissions; it cannot verify physical custody or a clean boot for you.

### Set up the public configuration

**Where:** a root terminal on the offline workstation. Python 3, Podman, Skopeo,
`findmnt` and `cryptsetup` must come from its approved operating environment.
Open the accepted delivery medium read-only as described in
[Decrypt the medium](../08-delivery_verification.md#3-decrypt-the-medium).

Copy the directory `guidance/deployment` from the medium to a writable root-only
directory: on this workstation, that copy is the deployment directory. In it, copy
`trustee.example.json` to `trustee.json` using the file manager, then fill these
public values:

| Setting | What to enter |
| --- | --- |
| `delivery_directory` | The read-only accepted delivery mount, normally `/mnt/temenos-delivery`. |
| `manifest_sha256` | The SHA-256 of `image-digests.txt` kept at [Compare with the signed delivery note](../08-delivery_verification.md#5-compare-with-the-signed-delivery-note), the value of `toe.manifest_sha256` in the installation configuration. |
| `working_directory` | The private `session` directory on this trustee's own unlocked LUKS-encrypted medium, for example `/media/trustee-one/session`. It must be owned by root with permissions **700**. |
| `keys_file` | Keep `trustee.toml`. This file is created on the private medium. |
| `request_file` | Keep `custody-request.json`; copy and rename the downloaded public request to this name when needed. |
| `proof_file` | Keep `custody-proof.json` for the key ceremony; use a new name for each tally. |
| `tally_session_id` | Keep `null` for the key ceremony. For a tally, enter the exact tally identifier shown by the portal, in quotes. |

Do not put passwords, private keys or the contents of `trustee.toml` into this
configuration. Set it up once for each trustee: a separate `trustee.json` for each
trustee avoids selecting another person's medium accidentally.

### Prepare the accepted tool

Before attaching a trustee's key medium, run:

```sh
./trustee prepare trustee.json
```

**Expected result:** `TRUSTEE IMAGE VERIFIED: all accepted manifests and layers
match`, followed by `TRUSTEE TOOL READY: accepted image loaded; safely eject the
delivery medium`. The command checks the entire OCI image graph against the
accepted manifest, loads it locally and retains only the public digest manifest
beside `trustee.json`. Safely eject the delivery medium through the file manager.

![Actual offline VM verifying and preparing the delivered trustee image](./assets/installation/trustee-prepare.png)

**If the result differs:** stop. A checksum or image mismatch requires the
accepted delivery to be checked again. Do not replace the expected checksum with
the observed value or enable networking to download an image.

### Generate keys in the trustee's private session

Unlock only this trustee's encrypted working medium and select its private
working directory in `trustee.json`. Then run:

```sh
./trustee generate trustee.json
```

**Expected result:** `TRUSTEE KEYS CREATED: trustee.toml`, followed by JSON with
only `signing_public_key` and `share_encryption_public_key`. The keys file is
created with owner-only permissions. An existing file is never replaced.

![Actual offline VM creating trustee keys on the encrypted working medium](./assets/installation/trustee-generate.png)

To display those public keys again, run:

```sh
./trustee public-keys trustee.json
```

![Actual offline VM displaying only the two public trustee keys](./assets/installation/trustee-public-keys.png)

**If the result differs:** an active interface or swap requires shutting down and
correcting the offline operating environment. A medium or permission error means
the private working directory is not ready. An existing keys file means this
trustee already has keys: use `public-keys`; do not delete it and generate a
replacement. Only on the election board's instruction at the end of an election
event does the trustee destroy it, with `./trustee destroy trustee.json`
([Trustee Procedures](../../02-election_managers/05-election_organizer_handbook/09-trustee-procedures.md)). If the accepted image is missing, repeat the prepare command with
the accepted delivery medium.

Only this trustee's working directory is mounted into the offline container.
The command supplies the private-key file path explicitly, so private keys are
never intentionally printed. Make the two backups that
[Trustee Procedures](../../02-election_managers/05-election_organizer_handbook/09-trustee-procedures.md)
requires, and follow it for the portal's public-key registration and custody-proof upload.
