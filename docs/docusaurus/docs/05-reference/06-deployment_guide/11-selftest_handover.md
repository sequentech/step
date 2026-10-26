---
id: selftest_handover
title: Self-Test and Handover
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

The installation is complete when every check of
[Verifying the Installation](10-verification.md) has passed and the operator
has given the administrators the confirmation of
[Confirm the server settings to the administrator](#confirm-the-server-settings-to-the-administrator).
The TOE is then handed over to the administrator's work in the
[Handbook for Election Administrators (AGD-02)](../../02-election_managers/04-administrator_handbook/01-about.md).

## Before the first import

:::warning
The TOE refuses every import of election data until the tenant's rules for
signed configuration packages are set. Set them before the first import.
Afterwards only a staff CA is replaced, between elections, as the
Handbook for Election Administrators describes.
:::

Before the election board imports any election data:

1. The administrator sets the tenant setting `configuration_signing` with
   `policy` "required", `required_approvals` of at least 2, and the election
   organizer's certificates in `package_roots` and `staff_roots`, as the
   section "Tenant and Election Event Settings" of the
   [Handbook for Election Administrators](../../02-election_managers/04-administrator_handbook/01-about.md)
   describes.
2. The administrator sets up the roles and the personal accounts and deletes
   the seeded accounts, as the section "Users, Roles and Role Separation" of
   the [Handbook for Election Administrators](../../02-election_managers/04-administrator_handbook/01-about.md)
   describes. The seeded administrator groups already hold none of the
   permissions that the handbook's role tables exclude from the administrator
   role ([Configuring the Deployment](07-hardening.md#configure-the-realms)). Only
   the election board's role holds "Create Election Event"
   (`election-event-create`), with which a board member imports election data
   with "Import Election Event" and a signed configuration package, and the
   permissions to publish configuration versions and to read the electoral
   log.
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

1. The operator asks the operator of the external authentication server to
   register the redirect URI
   `https://PORTAL_HOST/realms/E/broker/IDP_ALIAS/endpoint`, with the realm
   name of the election event in place of E (see
   [Connecting the Authentication Server](06-authentication_server.md)).
2. The operator makes the checks of
   [Election event realms](10-verification.md#election-event-realms) and
   tells the administrators the result.
3. The administrator checks the election event's authentication settings as
   the section "Authentication and Session Settings" of the
   [Handbook for Election Administrators](../../02-election_managers/04-administrator_handbook/01-about.md)
   describes.
4. The administrator sets the self-test settings in the "Settings" section of
   the election event's "Self-tests" tab, as the section "Self-Test Settings"
   of the
   [Handbook for Election Administrators](../../02-election_managers/04-administrator_handbook/01-about.md)
   describes. The default "Self-test interval (minutes)" is 15.
5. The administrator starts a self-test of the election event with "Run
   self-test". The election event's "Self-tests" tab must show the run with
   every test passed. The TOE also runs this self-test of each election event
   when it starts up.
6. The operator gives the administrators the confirmation of
   [Confirm the server settings to the administrator](#confirm-the-server-settings-to-the-administrator)
   again, before the election board approves the configuration version of
   the election event.

## Confirm the server settings to the administrator

The admin portal shows no plugins, no telephone provider and no other
tenants. The administrators therefore rely on the operator's written
confirmation of these settings. The operator gives it after the installation
and again for each new election event, before the election board approves its
configuration version.

1. Make every check of
   [Data correction, telephone channel, plugins and tenants](10-verification.md#data-correction-telephone-channel-plugins-and-tenants)
   and of [No smart-link sign-in](10-verification.md#no-smart-link-sign-in).
2. If every check gave the result stated there, give the administrators a
   written confirmation, with the date, the time and your name, through a
   channel that protects its integrity. It states that:
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
3. If a check gives another result, give no confirmation. Stop, tell the
   administrators that the election board must not approve the configuration
   version, and report the result to
   [security@sequentech.io](mailto:security@sequentech.io).

The administrators know that the server settings are confirmed when they hold
this confirmation for the current election event.

## Operating the servers

The operator operates the servers with the commands of this guide only, on
C1, as root, in every phase.

:::warning
Never change a running container, a Secret or a file under
`/var/lib/sequent-data` by hand, and never delete records to free storage.
Every change goes through `/root/deploy/kubernetes` and a deployment with
`secrets.env` loaded, as in [Deploy](08-installing_toe.md#deploy), followed
by every check of [Verifying the Installation](10-verification.md).
:::

- State of the platform: `k3s kubectl get pods -A` (every pod `Running` or
  `Completed`) and `k3s kubectl get nodes` (both nodes `Ready`).
- Operating log of a component: `k3s kubectl -n <namespace> logs
  deployment/<name>`, for example `-n step-infra logs deployment/keycloak`.
  These are the platform's operating logs. The TOE's audit records are the
  electoral log, which only the election board reads.
- Restart of a component that has failed:
  `k3s kubectl -n <namespace> rollout restart deployment/<name>`, then
  `k3s kubectl -n <namespace> rollout status deployment/<name>`, then the
  checks [Running images](10-verification.md#running-images) and
  [Version](10-verification.md#version). In the execution phase, restart a
  component only when the election board has decided so with the
  administrator: if harvest or windmill failed, the election event is in
  maintenance mode. Tell the administrators and the election board when the
  component runs again, and leave the self-test and "Resume Voting" to the
  election board. Outside the execution phase, end with a self-test as above.
- Time: repeat the checks of
  [Authenticated time](04-hosts_network_time.md#authenticated-time). If one
  fails, run `systemctl restart chronyd` on the server concerned and repeat
  them. If they still fail, stop and report it.
- Certificates: renew them before they expire, as
  [Set up the TLS endpoint](05-certificates_tls.md#set-up-the-tls-endpoint)
  describes, and repeat
  [TLS endpoint and ingress ports](10-verification.md#tls-endpoint-and-ingress-ports).
- Updates: only a new TOE delivery, accepted as
  [Verifying the Delivery](../08-delivery_verification.md) describes and
  installed with this guide. Never patch a running TOE.
- Operating system and cluster software: change them only with a new
  installation kit from Sequent, verified and installed as
  [Installing the Servers](03-installation_kit.md) describes, outside the
  execution phase, followed by every check of
  [Verifying the Installation](10-verification.md).

### Storage

`df -h /var/lib/sequent-data` on C1 shows the space left for the databases,
the object storage and the electoral log. When the storage of the audit data
is more than 80 % full, the TOE refuses voters' sessions and requests until
space is available again. Check the space every day in the preparation and
execution phases, and extend the storage before that point, outside the
execution phase: give the volume that holds `/var/lib/sequent-data` more
capacity, grow its file system, and check with `df -h /var/lib/sequent-data`
that the space has grown. If the TOE reports that the storage of the audit
data is more than 80 % full, extend the storage at once in the same way, also
in the execution phase: tell the election board first, which decides whether
to pause voting while you extend it. Voters can sign in again once the used
space is below 80 %. If the storage cannot be extended in time, stop and
report it. Never delete records, logs or files to make space.

### Backups and restore

:::warning
A backup holds the election data, the election execution data, the electoral
log and the deployment directory with its secrets. Make it only encrypted,
keep its passphrase apart from it, and keep both in separate locked places to
which only the operators have access.
:::

Make a backup after the installation, before each election event enters its
execution phase and after its post-processing phase, never during the
execution phase. The TOE is not available while the backup runs. From the
administration workstation, with a backup medium mounted at the path you give:

<!-- deployment-step: 11-001 -->

**Session: admin.** Run `step 11-001`. Review its implementation with `show 11-001`.

Repeat the last command until it prints `0`. Then write the backup and start
the TOE again:

<!-- deployment-step: 11-002 -->

**Session: admin.** Run `step 11-002`. Review its implementation with `show 11-002`.

The last command must print two lines, one with `root/deploy/` and one with
`var/lib/sequent-data/`, each after the number of entries it holds. Then make the checks of
[Running images](10-verification.md#running-images) and
[Version](10-verification.md#version), and a self-test as above.

A restore replaces the election data, the election execution data and the
electoral log with an older state. Restore only outside the execution phase,
or in it only after the election board has decided so, and tell the
administrators and the election board of every restore, so that the board
checks the electoral log. To restore:

1. Make a backup of the current state as above, and keep it.
2. Scale the deployments to 0 as above and wait until the count is `0`.
3. Set B to the backup to restore (step 1 has set it to the new backup),
   empty the data directory and extract the backup on C1:

   <!-- deployment-step: 11-003 -->

   **Session: admin.** Run `step 11-003`. Review its implementation with `show 11-003`.

The step asks for explicit restore authorization, then validates decryption and
the complete archive before deleting any current data. Plaintext is streamed,
never written to an intermediate file. If this preflight fails, do not proceed.

4. Deploy again with the commands of [Deploy](08-installing_toe.md#deploy) on
   C1, then make every check of [Verifying the Installation](10-verification.md).

The backup does not hold the cluster's own state or the container image store.
If a server has to be installed again, install it as
[Installing the Servers](03-installation_kit.md) describes before the restore.
After extracting the backup and before step 4, load the delivered TOE images
onto each reinstalled node as
[Load the TOE images](08-installing_toe.md#load-the-toe-images) describes.

If C1 was reinstalled and the cluster state was lost, recreate the resources
that the manifests do not carry, using the restored files and original keys:

- apply the namespaces and recreate the two `step-tls-cert` Secrets as in
  [Install the certificate](05-certificates_tls.md#install-the-certificate);
- reinstall the TLS endpoint's files as in
  [Set up the TLS endpoint](05-certificates_tls.md#set-up-the-tls-endpoint);
- recreate `auth-server-ca` as in
  [Install the trust anchor](06-authentication_server.md#install-the-trust-anchor);
- recreate `keycloak-bootstrap-admin` and `keycloak-service-credentials` with
  the first block of [Create the service credentials](07-hardening.md#create-the-service-credentials),
  loading the restored `secrets.env`; never generate new deployment secrets
  or repeat the manifest transformations;
- with each trustee present, recreate `trustee1-keys`, `trustee2-keys` and
  their authentication Secrets as in
  [Install the trustee keys](08-installing_toe.md#install-the-trustee-keys).
  Use the original keys from the trustees' own protected media or backups,
  which must match the restored protocol state; do not generate new keys.
  Defer the rollout commands until after step 4.

Stop if an original key or restored credential is unavailable. Deploying
alone cannot recreate these resources. Restore one backup on the installed
servers before the handover, to show that the procedure works. If the recovery
plan includes replacement of C1, rehearse that case with a new cluster too.

Report every failed check and every security problem to
[security@sequentech.io](mailto:security@sequentech.io).
