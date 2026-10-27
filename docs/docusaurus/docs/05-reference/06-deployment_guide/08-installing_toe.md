---
id: installing_toe
title: Installing the TOE
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

This page covers the one-command platform installation, the browser checkpoint
for creating the customer, and the guided trustee-key import. Later backup and
retirement procedures are separate maintenance actions, not installation steps.

:::warning
Use only the TOE delivery accepted through
[Verifying the Delivery](../08-delivery_verification.md). The installation kit's
supporting images do not replace the delivered TOE images.
:::

## Deploy

**Where:** a root terminal on the administration workstation, in the extracted
`deployment` directory. Keep the accepted delivery medium mounted read-only and
use the checked `site.json` from [the start of this guide](01-about.md#install-from-one-configuration-file).
After `./deploy check site.json` passes, run:

```bash
./deploy install site.json
```

<!-- deployment-step: 08-005 -->

**Expected result.** Four numbered installation stages appear. The final line is
`INSTALL 4/4: platform installed; customer and trustee commissioning still required`.
The installer loads the protected credentials in its own server-side process;
you do not type a separate deployment command or source `secrets.env` yourself.

![Actual VM terminal showing the configuration-driven installation completing](./assets/installation/installer-complete.png)

*Actual VM installation capture. Read the final stage and verification lines.
The screenshot's site values are examples; match the running identity to your
accepted delivery. Trustee commissioning is still outstanding at this point.*

**If it stops:** preserve the reported stage and original credentials. Resolve
connectivity, capacity or the stated configuration fault before continuing.
The command refuses an occupied cluster and does not replay failed writes.
Do not regenerate secrets, replace a digest with `latest`, or rerun the fresh
installation as a recovery operation.

## Load the TOE images

<!-- deployment-step: 08-001 -->
<!-- deployment-step: 08-002 -->

**Automatic part of the command above.** The installer validates the seven
accepted OCI image directories, prepares private transfer archives for the six
server images, transfers them to C1 and C2, and checks their hashes and imported
digests. It prints `PASS: c1 delivery archive hashes and imported image digests`
and the corresponding line for `c2`.

You do not create tar archives or type a separate image-import command. Keep the
accepted delivery unchanged and the imported content cache available for later
verification. Either server failing its import blocks installation.

## Pin the TOE images

<!-- deployment-step: 08-003 -->

**Automatic part of the command above.** Every TOE container reference is pinned
to its accepted `@sha256:` digest. The two supporting `trustee:latest` references
belong to the kit's setup jobs, not to the two native braid trustee instances;
the installer verifies the supporting setup image separately against the kit.
The later [Running images](10-verification.md#running-images) check verifies
actual running content rather than relying only on a manifest's text.

## Remove the secrets from the realm manifest

<!-- deployment-step: 08-006 -->

**Automatic part of the command above.** After the initial realm has imported,
the installer removes its imported credentials from the stored manifest and
checks the resulting cluster Secret. The running realm retains the values it
imported. Expect
`PASS: imported realm credentials removed from its stored manifest and verified in the cluster`.

If the realm is unavailable or cleanup fails, stop and establish its actual
import state. Do not print secret-bearing documents, recreate the realm or
repeat secret generation.

## Create the customer's tenant

:::warning
All election work happens in the customer's tenant. The initial installation
tenant is the super-admin tenant: it serves only the steps of this guide, holds no
election event, and none of its personal accounts stays enabled.
:::

The operator creates the customer's tenant once, after [Deploy](#deploy) and
before the trustee keys are installed. `ADMIN_HOST` below means the value of
`hosts.administration` in your `site.json`; replace that label in the browser URL:

1. Open `https://ADMIN_HOST` on the administration workstation and sign in at
   the external authentication server as the installation account of
   [Add the installation account](07-hardening.md#add-the-installation-account).
2. In the menu, next to the tenant's name, select the plus button "Create new
   tenant". Enter the customer's short name in the field "Slug" and select
   "Save". The TOE creates the tenant and its realm from the tenant template
   that [Configure the realms](07-hardening.md#configure-the-realms) has
   hardened, with the service accounts and a seeded `admin` account without a
   password.

   ![Actual portal showing the customer-creation task in progress](./assets/installation/customer-creation-in-progress.png)

   *Actual portal capture from the final VM rehearsal. The **Create Tenant** task
   can remain **IN_PROGRESS** while the customer is provisioned. Wait for it to
   finish and confirm the agreed customer slug exists before commissioning; do
   not submit another creation request while this task is running. The displayed
   installation account and site values are laboratory examples.*

3. Put the customer slug and administrator identity in `commissioning` in
   `site.json`, then run `./deploy commission site.json` from the administration
   workstation. The command resolves the customer tenant on C1:

   <!-- deployment-step: 08-007 -->

   **Required result.** The lookup identifies exactly the installation tenant and the customer tenant matching the configured slug. The customer realm is `tenant-` followed by that customer UUID.

   **If the result differs.** If the customer row is absent, finish its creation in the portal first. If more than two tenants appear, stop and reconcile the installation state; never choose an arbitrary tenant or repeat creation.


   The installation tenant is `90505c8a-23a9-4cdf-a26b-4e19f6a097d5`; the
   other tenant must have the customer slug you supplied. The commissioning
   command records the resolved customer realm. Do not type or guess a UUID or
   edit a shell environment file to select a different tenant.
4. At the commissioning checkpoint, copy the complete URI printed after
   `Register this exact customer redirect URI`. Have the external provider
   operator register that exact address, following
   [Connecting the Authentication Server](06-authentication_server.md). Type
   `CONTINUE` only after registration and the external administrator account
   are ready; do not construct a URI by guessing the customer identifier.
5. The commissioning command adds the customer's first administrator and the
   two native trustee accounts, configures the trustee client secret, and disables
   the personal accounts of the installation realm. The administrator's user name
   and e-mail must match the identity supplied by the external authentication
   server; they come from the same configuration file:

   <!-- deployment-step: 08-008 -->

   **Expected result.** The terminal prints `PASS: customer administrator and native accounts configured; installation personal accounts disabled`. The next prompt asks you to register the two trustees in the customer portal.

   **If the result differs.** If account creation reports a conflict or a user remains enabled in the installation realm, stop and inspect the account state with the administrator. Do not rerun the entire creation block or give these accounts local passwords.


   The installer checks the account creation and disables the installation
   personal accounts. Native service accounts stay enabled. You do not receive
   or type a service password, and no later step uses the installation personal
   account. The exact prompts and success messages are shown in the
   [commissioning terminal screenshot](#transfer-the-matching-keys-and-start-the-instances).
6. Continue with [Install the trustee keys](#install-the-trustee-keys) below in
   the same commissioning terminal. Run the
   [platform inspection](10-verification.md#run-the-platform-inspection) only
   after both trustees are ready and commissioning completes.

## Install the trustee keys

Each trustee generates their keys on a trustee workstation (see
[Preparing a trustee workstation](#preparing-a-trustee-workstation) and the
trustee procedures of the
[Trustee Procedures (AGD-03)](../../02-election_managers/05-election_organizer_handbook/09-trustee-procedures.md)),
and the administrators register the public keys as the section "Registering
the trustees' public keys" of the
[Registering the trustees' public keys (AGD-02)](../../02-election_managers/04-administrator_handbook/04-users_and_roles.md#registering-the-trustees-public-keys)
describes. The trustee then hands over the keys file for their instance of
the trustee server: `trustee1` for the first trustee, `trustee2` for the
second.

### Bind each native account to its registered trustee

Before starting either instance, agree with the election administrator which
registered trustee and public keys belong to each instance. The machine login
names remain `trustee1` and `trustee2`; they can differ from the names registered
in Settings > "TRUSTEES". Each machine account's `trustee` attribute must name
exactly its registered trustee. The person's separate account uses "Act as
Trustee" for the same registered name.

The commissioning configuration associates the first trustee entry with native
instance `trustee1` and the second with `trustee2`. At its checkpoint, the
administrator registers the two public keys in the portal and each trustee
provides the corresponding private key file on their own medium. Before importing
either file, the command compares its public-key pair with the registered trustee
record. It changes only the native account's trustee binding; personal trustee
accounts remain separate and receive no local password.

![Actual customer portal listing the two registered laboratory trustees](./assets/installation/trustees-registered.png)

*Actual customer portal from the final VM rehearsal. Open each registered trustee
record and check both public keys against that trustee's own keys before
continuing. The customer and trustee names shown here are laboratory examples.*

<!-- deployment-step: 08-009 -->

**Required result.** Each native account is bound to its configured registered trustee name, and the installed key file matches that record’s public keys. Both instances must pass.

**If the result differs.** If there is no success line, check the exact registered name, customer tenant and instance number with the administrator and trustee. Do not start either server with an uncertain key-to-account mapping.


Record the native
account, registered trustee name and checked public-key pair in the protected
installation record. The binding is persistent customer-realm state; verify it
again after restoring a customer backup. The process's `TRUSTEE_NAME` also
selects its native login, so keep it as `trustee1` or `trustee2`. That display
alias does not replace verification of the trustee's signing public key.

### Transfer the matching keys and start the instances

:::warning
The keys file holds the trustee's private keys. Take it over only in person,
with the trustee watching every step, directly from the trustee's own medium.
No copy may remain on the administration workstation or on C1: it is kept
only in the cluster's encrypted Secret.
:::

**Where to work:** the administration workstation—the computer from which you
connect to C1 and C2 over SSH. This is not the admin portal, a user named `admin`,
or a separate trustee server.

1. Complete [customer creation and account setup](#create-the-customers-tenant)
   and register both trustees as described above. In `site.json`, each
   `commissioning.trustees` entry must contain the exact registered `name` and
   the absolute `keys_file` path on that trustee's mounted medium.
2. Open a root terminal in the extracted package's `deployment` directory. If
   commissioning is not already running, start it with the same configuration:

   ```bash
   ./deploy commission site.json
   ```

   If that command is already waiting for you, continue in that terminal;
   do not start a second copy. `./deploy` is the executable in the current
   directory. From another directory, use its full path and the full path to
   `site.json`.
3. At the trustee-registration prompt, verify the names and both public keys
   in the [portal screenshot above](#bind-each-native-account-to-its-registered-trustee).
   Type `CONTINUE` and press Enter only after completing that check.
4. At each `Trustee 1: be present...` / `Trustee 2: be present...` prompt,
   have that trustee confirm their own mounted medium and private-key file.
   Type `CONTINUE` and press Enter. Keep both media available, unchanged, and
   under their owners' supervision until both imports finish: the installer
   checks both key pairs before importing either, then reads each file again.
5. Let the command transfer the keys, create the encrypted cluster Secrets,
   bind the native accounts and start both instances. These are automatic
   operations; no additional command on C1 is required.

<!-- deployment-step: 08-010 -->

**Expected result.** First the terminal prints
`PASS: both trustee media match their registered public-key pairs before either import`.
After each successful import, it prints
`PASS: trustee 1 public-key pair, native binding and startup; transfer copy removed`
and the corresponding line for trustee 2. Only then unmount each trustee's
medium and return it to its owner. These messages confirm removal of the
installer's temporary transfer copies; do not make additional copies yourself.

![Actual VM commissioning terminal showing the trustee prompts, both key checks and successful startup](./assets/installation/installer-commission-complete.png)

*Actual terminal capture from the VM rehearsal on 2026-10-26. The command at
 the top is the one to run. The `CONTINUE` prompts and `PASS` lines show the
 expected sequence. Hostnames and account identifiers are laboratory examples;
 use your approved `site.json` values.*

**If the result differs:**

- **Keys do not match:** stop. With the trustee, check the configured file,
  registered name, tenant and both registered public keys. Do not replace keys
  or alter the registration simply to silence a mismatch.
- **Transfer, Secret creation or startup fails:** stop and inspect the stated
  error and the actual cluster state. Do not overwrite an existing Secret,
  generate replacement keys or assume that temporary-file cleanup succeeded.
- **A previous attempt stopped during writes:** the installer refuses automatic
  replay. Resolve the partial state with the operator before retrying; do not
  remove its state markers to bypass this refusal.

<!-- deployment-step: 08-011 -->

The same commissioning command configures each native instance's sign-in and
waits for its deployment to become ready. Keep the configured names and keys;
personal trustee accounts remain separate from these native machine accounts.

<!-- deployment-step: 08-012 -->

**Final expected result.** The terminal prints
`PASS: customer login restrictions and both native trustees Ready`, followed by
`COMMISSIONING COMMANDS COMPLETE — perform the browser, client-network and board handover checks in AGD-04.`
This is not authorization to start an election.

Before each key or tally ceremony, the operator must confirm that both instances
are ready. From the same `deployment` directory, run `./deploy commission site.json`
with the unchanged completed configuration. On a completed installation it checks
readiness and login restrictions without importing the keys again. Expect the
`PASS: customer login restrictions and both native trustees Ready` line; the
first-run completion banner is not repeated. If the check fails, do not authorize
the ceremony until the operator resolves the fault and the check passes.

## Back up the trustee state

**When:** after a key ceremony, with no ceremony in progress and with the board
and trustees informed. This briefly stops both native trustee instances. The
state is on C1 under `/var/lib/sequent-data/trustee1` and `trustee2`.

**Where:** a root terminal in the extracted `deployment` directory on the
administration workstation, using its unchanged `site.json`. Mount the backup
medium first. Copy the complete block; it asks for the mounted directory and a
new encryption passphrase privately. No private key is printed.

<!-- deployment-step: 08-013 -->

```bash
set -Eeuo pipefail
umask 077
C1=$(jq -er '.nodes.c1' site.json)
IDENTITY=$(jq -er '.ssh_identity' site.json)
c1() { ssh -o BatchMode=yes -o StrictHostKeyChecking=yes -i "$IDENTITY" "root@$C1" "$@"; }
read -r -p "Mounted encrypted-backup directory: " BACKUP_DIRECTORY
test -d "$BACKUP_DIRECTORY" && test ! -L "$BACKUP_DIRECTORY"
B="$BACKUP_DIRECTORY/trustee-state-$(date -u +%Y%m%dT%H%M%SZ).tar.gpg"
test ! -e "$B"
c1 'set -Eeuo pipefail; umask 077
 test ! -e /root/deploy/.trustee-backup-in-progress
 k3s kubectl -n step-infra get deploy trustee1 trustee2 -o json |
   jq "[.items[] | {name:.metadata.name,replicas:(.spec.replicas // 1)}]" > /root/deploy/trustee-backup-replicas.json
 touch /root/deploy/.trustee-backup-in-progress
 k3s kubectl -n step-infra scale deploy trustee1 trustee2 --replicas=0
 for attempt in $(seq 1 60); do
   count=$(k3s kubectl -n step-infra get pods -l "app in (trustee1,trustee2)" -o json | jq ".items | length")
   if test "$count" = 0; then echo "TRUSTEES STOPPED: state backup may begin"; exit 0; fi
   sleep 2
 done
 echo "Trustee shutdown incomplete; no backup taken" >&2; exit 1'
c1 'set -Eeuo pipefail
 test -f /root/deploy/.trustee-backup-in-progress
 count=$(k3s kubectl -n step-infra get pods -l "app in (trustee1,trustee2)" -o json | jq ".items | length")
 test "$count" = 0
 tar -C /var/lib/sequent-data -cf - trustee1 trustee2' |
  gpg --symmetric --cipher-algo AES256 --output "$B"
gpg --decrypt "$B" | python3 ./archive_check.py trustee
c1 'set -Eeuo pipefail
 jq -r ".[] | [.name,(.replicas|tostring)] | @tsv" /root/deploy/trustee-backup-replicas.json |
 while read -r name replicas; do k3s kubectl -n step-infra scale "deployment/$name" --replicas="$replicas"; done
 for name in trustee1 trustee2; do k3s kubectl -n step-infra rollout status "deployment/$name" --timeout=120s; done
 rm /root/deploy/.trustee-backup-in-progress'
printf 'TRUSTEE BACKUP COMPLETE: %s\n' "$B"
```

**Expected result.** `TRUSTEES STOPPED: state backup may begin`, then
`BACKUP CONTENTS VERIFIED: trustee`, successful rollout of both instances, and
`TRUSTEE BACKUP COMPLETE:` followed by the encrypted backup path. The script
records and restores each original replica count; it does not assume every
Deployment should have one replica.

![Actual rehearsal terminal completing an encrypted trustee-state backup](./assets/installation/trustee-backup.png)

*Actual terminal executing the complete backup block above. The laboratory test
driver enters the test paths and prompts; it is not part of the installation
instructions. No passphrase is shown. Keep the encrypted backup and its passphrase
in separate locked places, under the trustee-custody procedure.*

**If it fails:** keep the original state and the last known-good backup. Do not
accept a partial archive. A timeout waiting for the pods to stop prevents the
copy. Encryption or validation failure leaves the trustees stopped. Diagnose
that error before deciding whether to retry the backup or resume unchanged state.

<details>
<summary>Resume unchanged trustees after a failed backup</summary>

Use this only after a failed **backup**, while the original trustee state is
intact. It does not recover damaged state or undo removal. From a root terminal
in the workstation's `deployment` directory, copy:

```bash
set -Eeuo pipefail
C1=$(jq -er '.nodes.c1' site.json)
IDENTITY=$(jq -er '.ssh_identity' site.json)
ssh -o BatchMode=yes -o StrictHostKeyChecking=yes -i "$IDENTITY" "root@$C1" 'set -Eeuo pipefail
 test -f /root/deploy/.trustee-backup-in-progress
 test -f /root/deploy/trustee-backup-replicas.json
 jq -e "length == 2 and all(.[]; (.name == \"trustee1\" or .name == \"trustee2\") and .replicas >= 0)" /root/deploy/trustee-backup-replicas.json >/dev/null
 jq -r ".[] | [.name,(.replicas|tostring)] | @tsv" /root/deploy/trustee-backup-replicas.json |
 while read -r name replicas; do k3s kubectl -n step-infra scale "deployment/$name" --replicas="$replicas"; done
 for name in trustee1 trustee2; do k3s kubectl -n step-infra rollout status "deployment/$name" --timeout=120s; done
 rm /root/deploy/.trustee-backup-in-progress
 echo "TRUSTEE REPLICA COUNTS RESTORED"'
./deploy commission site.json
```

Expect successful rollouts, `TRUSTEE REPLICA COUNTS RESTORED` and the commissioning
readiness check's `PASS` line. The normal backup screenshot above shows the same
rollout result. If any command fails, keep the marker and do not authorize a
ceremony. A missing or invalid replica record requires investigation; do not
guess the counts or delete the marker manually.

</details>

## Remove the trustee keys at the end of the election

:::warning
This destroys server-side trustee keys and protocol state. Do it only after the
retention period has ended, the required election execution data has been deleted,
and the election board has authorized retirement and is present. It is not part
of installation or troubleshooting.
:::

**Where:** the same root terminal in the administration workstation's extracted
`deployment` directory. Verify the target addresses in `site.json` with the board.
Copy the complete block; it requires the exact confirmation phrase.

<!-- deployment-step: 08-014 -->

```bash
set -Eeuo pipefail
umask 077
C1=$(jq -er '.nodes.c1' site.json)
IDENTITY=$(jq -er '.ssh_identity' site.json)
c1() { ssh -o BatchMode=yes -o StrictHostKeyChecking=yes -i "$IDENTITY" "root@$C1" "$@"; }
read -r -p "Type REMOVE TRUSTEE KEYS to confirm: " CONFIRM
test "$CONFIRM" = "REMOVE TRUSTEE KEYS"
c1 'set -Eeuo pipefail
 k3s kubectl -n step-infra scale deployment/trustee1 deployment/trustee2 --replicas=0
 for attempt in $(seq 1 60); do
   count=$(k3s kubectl -n step-infra get pods -l "app in (trustee1,trustee2)" -o json | jq ".items | length")
   if test "$count" = 0; then break; fi
   sleep 2
 done
 test "$count" = 0
 k3s kubectl -n step-infra delete secret trustee1-keys trustee2-keys trustee1-auth trustee2-auth
 find /var/lib/sequent-data/trustee1 /var/lib/sequent-data/trustee2 -mindepth 1 -delete
 remaining=$(k3s kubectl -n step-infra get secret trustee1-keys trustee2-keys trustee1-auth trustee2-auth --ignore-not-found -o name)
 test -z "$remaining"
 count=$(find /var/lib/sequent-data/trustee1 /var/lib/sequent-data/trustee2 -mindepth 1 | wc -l)
 test "$count" = 0
 echo "TRUSTEE REMOVAL COMPLETE: four Secrets absent; state directories empty"'
```

**Expected result.** The instances stop before any deletion. Four Secrets are
removed, the state directories become empty, and the last line is
`TRUSTEE REMOVAL COMPLETE: four Secrets absent; state directories empty`.

![Actual isolated rehearsal terminal confirming trustee retirement](./assets/installation/trustee-removal.png)

*Actual terminal executing the complete removal block above on an isolated
disposable copy. The laboratory test driver enters the test prompts; it is not
part of the installation instructions.
The confirmation and completion messages show the authorized retirement workflow.*

**If it fails:** preserve the remaining state and backups and record exactly what
was removed. Do not authorize another ceremony or destroy backups while the
outcome is uncertain. After confirmed server removal, the board directs the
custodians to destroy every retained trustee-state backup and records removal
and backup destruction in its minutes.

## Preparing a trustee workstation

A trustee workstation runs the `trustee` program of the delivered `braid`
image, offline.

:::warning
The workstation has no network connection while handling keys. Use each
trustee’s encrypted medium and a clean offline session as described in
[Trustee key handling](02-operational_environment.md#trustee-key-handling).
The hardware may be shared; the keys, media and sessions must remain separate.
Load the tool only from the verified delivery after checking its digest.
:::

**Where:** a root terminal on the offline trustee workstation, before connecting
any trustee key medium. The workstation needs Podman and Skopeo from its approved
image. Open the accepted delivery medium read-only using the exact commands in
[Decrypt the medium](../08-delivery_verification.md#3-decrypt-the-medium).
Once it is mounted at `/mnt/temenos-delivery`, copy this block:

<!-- deployment-step: 08-015 -->

```bash
set -e -o pipefail
M=/mnt/temenos-delivery
D=$(sed -n 's/^braid \(sha256:[0-9a-f]*\)$/\1/p' "$M/image-digests.txt")
[ "$(skopeo inspect --format '{{.Digest}}' "oci:$M/images/braid:v10.0.0")" = "$D" ]
echo "braid image verified"
skopeo copy "oci:$M/images/braid:v10.0.0" containers-storage:localhost/braid:v10.0.0
[ "$(podman image inspect --format '{{.Digest}}' localhost/braid:v10.0.0)" = "$D" ]
echo "braid image loaded"
umount "$M" && cryptsetup close temenos-delivery
echo "D=$D" > ~/braid-digest
```

**Expected result.** Both `braid image verified` and `braid image loaded` appear. The delivery medium is unmounted and the checked digest is retained locally.

**If the result differs.** If either line is absent or a digest check fails, do not use the trustee tool. Re-check the verified delivery and local image import while keeping the workstation offline.


:::warning
Never start the image with its default command on the workstation: that
command starts the trustee server of the platform. Use only the `trustee`
command below. It runs the program `/usr/bin/trustee` instead, gives the
container no network, and mounts only the current directory, which holds the
keys file.
:::

In the same offline root terminal, define the `trustee` command. Repeat this
short block whenever you open a new shell; the function is local to that shell:

<!-- deployment-step: 08-016 -->

```bash
set -e -o pipefail
. ~/braid-digest
trustee() { podman run --rm --network none --entrypoint /usr/bin/trustee \
  -v "$PWD":/work:Z -w /work "localhost/braid@$D" "$@"; }
trustee --help
```

**Expected result.** The trustee help screen lists `generate`, `public-keys`, `attest` and `verify`, among its subcommands.

**If the result differs.** If the command is missing or starts a server instead, stop. Check the verified braid image and this wrapper; do not run the image's default entry point or enable network access.


The program sees only the current directory: run each key-handling command
from the directory on the trustee's own medium
that holds the keys file, and give file names relative to it, for example
`trustee public-keys --trustee-config trustee.toml`. The trustees use the
subcommands as the trustee procedures of the
[Trustee Procedures (AGD-03)](../../02-election_managers/05-election_organizer_handbook/09-trustee-procedures.md)
describe. Result verification is a separate procedure in
[Verifying the Result with the Election Verifier](../../02-election_managers/05-election_organizer_handbook/10-election-verifier.md);
it does not use private trustee keys.

![Actual network-disabled trustee-tool help screen from the verified rehearsal image](./assets/installation/trustee-tool.png)

*Actual Podman execution on the laboratory host, using the verified local OCI
image with `--network none`. This shows the tool and its help; it does not show
a wholly offline workstation session or actual private-key custody. Follow the
offline workstation, private-medium and clean-session procedure above.*
