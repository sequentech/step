---
id: about
title: About the Deployment Guide
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

| Item | Value |
| --- | --- |
| Document ID | AGD-04 |
| Title | Deployment Guide |
| TOE reference | Sequent Voting Platform - uniWAHL Version, v10.0.0 |
| Document version | 1.1 |
| Software release | v10.0.0 |

## Who this guide is for

This guide is for the operator: the administrator in their duties on the servers
([Roles and people](../../02-election_managers/04-administrator_handbook/01-about.md#roles-and-people)).
The operator installs the servers and the cluster, the certificates, the
connection to the external authentication server, the TOE images and the initial
accounts, needs root access to the two servers and works from the administration
workstation. The election organizer may give these duties to the person who works
in the admin portal or to someone else. Either way the operator counts as an
administrator and is never a board member or a trustee.

The administrator's duties inside the platform (users and roles, tenant and
election event settings, self-tests) are in the
[Handbook for Election Administrators](../../02-election_managers/04-administrator_handbook/01-about.md).
The administrator never imports, reads or changes election data and never
reads the TOE's audit records (the electoral log): the election board imports
the election data and reviews the electoral log. The duties of the election
board and the trustees are in the
[Handbook for Election Organizers](../../02-election_managers/05-election_organizer_handbook/01-about.md),
and the voters' guidance is the
[Handbook for Voters](../../03-voters/02-voter_handbook/01-about.md).

## This document

This guide holds the preparative procedures of the Sequent Voting Platform -
uniWAHL Version, v10.0.0 (the TOE). It describes how to accept the delivered
TOE, how to prepare its operational environment and how to install and
configure the TOE, so that it starts in its evaluated configuration.

There is exactly one evaluated configuration: all eleven TOE parts deployed and
operated together, on the operational environment that this guide describes, with
every setting this guide makes. A deployment that leaves out a step, changes a
setting or adds a component the guide does not name is not the evaluated
configuration.

## Terms

| Term | What it means |
| --- | --- |
| operator | The administrator in their duties on the servers, as described above. In this guide the word alone always means this person. |
| administrator | A person the election organizer appoints to set up and look after the TOE. In the admin portal: the role `admin`. |
| election board | The group of people the election organizer appoints to run the election event. Its members are never administrators. |
| tenant | The organisation's space in the TOE. It holds the administrators', board members' and trustees' accounts and the election events. |
| election event | What the Security Target calls an election: the whole vote that the election organizer holds. |
| electoral log | The audit records of one election event. Each election event has its own electoral log, which the election board reads and the administrators and operators do not. |
| operating logs | The logs of the servers and containers, which the operator reads to run the system. They are not audit records. |
| the board's minutes | The election board's own record of the election event, kept outside the TOE in the form the electoral regulation sets ([Write the electoral regulation](../../02-election_managers/05-election_organizer_handbook/02-organizer-duties.md#write-the-electoral-regulation)). |
| administration workstation | The protected Linux computer from which the operator runs `./deploy` and the commands of this guide ([Administration workstation](02-operational_environment.md#administration-workstation)). |
| customer | The organisation whose election events the TOE holds. It has one tenant, the customer's tenant. |
| slug | The customer's short name: the field "Slug" of [Create the customer's tenant](08-installing_toe.md#create-the-customers-tenant). People enter it in the field "Tenant Name" at sign-in. |
| super-admin tenant | The tenant the installer creates, only for the installation steps. It holds no election event. |
| realm | The identity service's space for one tenant or one election event. |
| instance | One of the two `braid` deployments, `trustee1` and `trustee2`, with the machine account of the same name. |
| session store | A file in which an instance keeps the messages of one bulletin board, including the key shares encrypted to that instance. The session stores are the instance's protocol state. |
| C1, C2 | The two servers ([Servers](02-operational_environment.md#servers)). |
| kit | The USB drive that Sequent builds for your site ([Installing the Servers](03-installation_kit.md)). |
| site plan | The values of your site that Sequent builds the kit from ([Agree the site plan](03-installation_kit.md#agree-the-site-plan)). |
| deployment directory | Your writable, root-only copy of the directory `guidance/deployment` of the delivery medium, on the administration workstation ([Before running the installer](#before-running-the-installer)). On the servers, the installer's own directory is `/root/deploy`. |
| external authentication server | The OpenID Connect server at which people sign in ([External authentication server](02-operational_environment.md#external-authentication-server)). This guide also calls it the provider or the identity provider, as the field `identity_provider` of `site.json` does. |
| mail administrator | The person who provides the SMTP relay. |
| authentication server's administrator | The person authorized to register applications and enforce MFA at the external authentication server. |

## The TOE

| Part | What it does |
| --- | --- |
| tls-endpoint | The only listener for clients: TLS 1.3 on port 443 of C1, forwarding the requests to the other parts |
| harvest | Central backend: access control, voters' register, ballot boxes, import and export |
| graphql-engine | The GraphQL interface: checks the token of each request, enforces table permissions and forwards actions to harvest |
| windmill | Two workers for the asynchronous tasks |
| keycloak | Identities, sessions and tokens; the link to the external authentication server |
| voting-portal | The voters' web application, with the code that encrypts the vote |
| admin-portal | The web application of the administrators, the election board and the trustees |
| ballot-verifier | The voter's check that a ballot was cast as intended |
| election-verifier | Verifies the shuffle and decryption proofs of an election event's tally |
| braid | The trustee: key generation, mixing and threshold decryption |
| b4 | The bulletin board of the trustees |

Each part is an OCI image. The guidance documents AGD-01 to AGD-04 are
delivered with them. Everything else this guide installs is operational
environment, not TOE: the servers, the operating system and the cluster
software installed from the installation kit, the ingress behind the TLS
endpoint, the container runtime, the database, the object storage, the
monitoring chart renderer, the network, the time server, the external
authentication server and the web browsers.

## Install from one configuration file

Start here. The normal platform installation is **two commands on the administration
workstation**. The `deploy` program is an actual file in the delivered directory
`guidance/deployment`; it is not a command that must already be installed or a
function hidden in a special shell. Customer and trustee commissioning then uses
one further command, with checkpoints for the authentication server's
administrator, the administrator and the trustees.

The first command checks the supplied configuration, files and servers. The second
prepares the hardened configuration, installs the platform and checks the result.
The detailed chapters explain those operations and their security conditions;
they are not a second list to repeat after the installer has run.

### Before running the installer

Complete these preparations once:

1. Accept the delivered TOE and guidance using
   [Verifying the Delivery](../08-delivery_verification.md), including its
   confirmation of receipt. The installation command does not authorize acceptance.
2. Provide the [Operational Environment](02-operational_environment.md): the two
   servers, protected administration workstation, network, authenticated time
   service and the people it names.
3. Obtain the site TLS certificate and private key from your certificate process,
   and the external identity provider's registration and CA certificate. The
   requirements of [Certificates and TLS](05-certificates_tls.md) and
   [Connecting the Authentication Server](06-authentication_server.md) remain mandatory.
   The installer cannot issue a trusted certificate or register an application at
   another organization's identity provider for you.
4. Mount the delivery medium read-only again
   ([Decrypt the medium](../08-delivery_verification.md#3-decrypt-the-medium)) and
   copy the directory `guidance/deployment` to a root-only directory on the
   administration workstation. This guide calls that copy the deployment directory.
   In it, `./deploy check` without a configuration file compares the copy with the
   directory's own file hashes and prints
   `Package hashes match. Independently verify its delivery signature and source identity.`
   On the servers, the installer's own directory is `/root/deploy`. In the
   deployment directory, copy `site.example.json` to `site.json`, open it in a text
   editor and fill in the site's public values and file paths. Keep passwords,
   tokens and private keys out of this JSON file: it refers to protected files where
   those are needed.
5. Verify the kit and install C1 and C2, as
   [Installing the Servers](03-installation_kit.md) describes. The operator
   confirms the disk at each server console before it is erased. Then verify both
   SSH host keys against their server-console fingerprints, as
   [Verify SSH host keys before first use](03-installation_kit.md#verify-ssh-host-keys-before-first-use)
   describes. These instructions show the commands, actual console and
   first-connection prompt, and when to refuse the connection.

### Where to run each instruction

| Place | What it means in this guide |
| --- | --- |
| Administration workstation | The protected Linux computer from which you run `./deploy`. Open its terminal in the deployment directory. |
| C1 or C2 console | The physical or trusted management console of that server, used for disk confirmation and host-key verification. |
| C1 or C2 over SSH | A command sent to a server using the administration workstation's verified SSH connection. The guide supplies the complete command. |
| Client-network computer | A separate computer on a voter/client network, outside the administration allowlist. It tests which server ports clients can reach. |
| Trustee workstation | The clean offline session used by one trustee at a time with their own protected medium. It is not another always-on server. |

There are two different configuration files. The delivered kit's `nixos/site.json`
records the server configuration already signed into that kit; do not edit it.
Your working `site.json` in the deployment directory points to the accepted kit,
certificates and other site inputs used by `./deploy`. The following field table
describes this working file.

Fill the configuration with the values of your site and of the verified delivery:

| Field | What to supply |
| --- | --- |
| `nodes` | C1 and C2 administration-network addresses |
| `ssh_identity` | Path to the administration workstation's protected SSH key |
| `hosts` | Voting, administration, ballot-verifier and external-authentication host names |
| `idp_alias` | The external provider's alias in the TOE |
| `kit` | Verified kit directory and the signing fingerprint received independently |
| `toe` | Accepted delivery root containing `images/` and `image-digests.txt`, `format: "oci"`, and the SHA-256 of `image-digests.txt` that you kept at [Compare with the signed delivery note](../08-delivery_verification.md#5-compare-with-the-signed-delivery-note) |
| `tls` | Paths to the issued site certificate, protected private key and CA, plus the independently checked CA fingerprint |
| `identity_provider` | Paths to the protected provider definition and CA, plus the independently checked CA fingerprint |
| `operator` | The operator's existing identity-provider user name and e-mail; the separate service-administrator name |
| `notifications` | Security-notification sender address and path to the private SMTP configuration supplied by the mail administrator |
| `release` | Software version: the value of the `tag:` line of the delivered file `image-digests.txt` without its leading `v` (`tag: v10.0.0` gives `10.0.0`). Source revision: the complete value of the `commit:` line. |
| `commissioning` | Customer slug, first administrator identity, and two trustee names with paths to their independently held key files; used by the commissioning command |

Use absolute paths. The example's addresses, hash placeholders and names are
examples only. The installer refuses unknown fields and invalid values. A
successful schema check does not prove that the values describe your site.

Point `toe.directory` directly at the accepted delivery medium. For example, this
section of `site.json` selects the OCI images already supplied by Sequent:

```json
"toe": {
  "directory": "/mnt/temenos-delivery",
  "format": "oci",
  "manifest_sha256": "REPLACE_WITH_ACCEPTED_IMAGE_MANIFEST_SHA256"
}
```

The medium contains `image-digests.txt` and one OCI image directory under `images/`
for each of the eleven TOE parts. Leave those files unchanged. You do not create
tar files or enter ten more hashes: the check validates the delivered image
descriptors and blobs, and installation prepares and checks its own temporary
transport archives for the ten server images. The election-verifier image remains
part of the accepted delivery even though it is not installed on the servers.

Keep the `identity_provider.definition` file private: it contains the client
secret. Its format and required settings are described in
[Connecting the Authentication Server](06-authentication_server.md#write-the-provider-definition).
The certificate and CA inputs are checked before they are installed. Do not put
private-key contents, client secrets or passwords into `site.json`.

### Configure security email

Obtain an authenticated SMTP relay from the mail administrator. Security-state
changes and audit-storage warnings must reach the administrators and election
board as distinct email messages.

In your private editor, save the mail administrator's settings as
`/root/installation-inputs/smtp.json`. The file has this shape; replace the
example relay and credentials with the supplied values:

```json
{
  "server_url": "smtps://USERNAME:PASSWORD@smtp.example.org:465",
  "timeout": 5
}
```

Use the complete URL supplied by the mail administrator, including any required
percent-encoding in its credentials. Keep the file private:

```bash
chmod 600 /root/installation-inputs/smtp.json
```

Set `notifications.sender` to the approved sender address and
`notifications.smtp_config_file` to that file's absolute path in `site.json`.
The installer transfers it privately and creates the application's protected
mail Secret. You do not enter the password in a terminal command or edit a
Kubernetes manifest.

**Expected result:** `./deploy check site.json` accepts the private configuration
and `./deploy install site.json` supplies it before workers start. You cannot
test delivery at installation: the TOE sends security notifications only for an
election event. At installation, have the mail administrator confirm that the
relay accepts the sender address. The administrators then test delivery with the
election board
([Check monitoring and authentication](../../02-election_managers/04-administrator_handbook/11-security_relevant_events.md#6-check-monitoring-and-authentication)):
creating and archiving an empty election event sends an email. If an email does
not arrive, follow
[A security notification is not delivered](11-selftest_handover.md#a-security-notification-is-not-delivered).
Configuration validation alone does not establish delivery.

**If it fails:** check the reported path, file permissions, relay URL and timeout.
The evaluated installer accepts authenticated `smtps://` URLs and a timeout of
1–10 seconds. For a private SMTP certificate authority, the mail administrator
may supply the additional `root_certificate_pem` field with the approved CA
certificate. Hostname and certificate verification remain required. Never
substitute a console transport or disable TLS verification to obtain a pass.

### Check the site

Open a root terminal on the administration workstation, in the deployment
directory, and run:

```bash
./deploy check site.json
```

Here `./deploy` means the file named `deploy` in this directory, and `site.json`
is your working configuration. From another directory, use the full paths to both
files instead.

**Expected result.** The checks finish with
`CHECK PASSED — ready to install the platform`. Read the individual check results:
the file paths, certificates, server identities and policies must match this site.

![Actual terminal running the configuration check against two fresh servers](./assets/installation/installer-preflight.png)

*Terminal capture from a test installation. Your addresses, names and digests differ.*

**If the result differs.** Correct the configuration or the named prerequisite and
run this read-only check again. If it refuses the administration workstation,
install what its message names
([Administration workstation](02-operational_environment.md#administration-workstation))
and run it again. Do not change a certificate fingerprint to match
an unexpected certificate, suppress an error, or proceed after a failed check.
A missing `deploy` means the terminal is not in the deployment directory; it does
not mean a package must be downloaded from the Internet. If existing platform
state is detected, stop and follow [If something fails](#if-something-fails): this
fresh-install workflow is not an upgrade or an instruction to clear an occupied
cluster.

If the check reports missing supporting-image metadata or an incompatible
supporting image on a server, obtain the corrected complete signed installation
kit from Sequent. The installer compares the signed image and public-asset records
with the actual installed supporting image automatically. Do not edit an inventory,
replace an individual tool or accept a different image just to pass the check.

![Actual preflight refusing a supporting image that does not match the signed kit](./assets/installation/installer-preflight-refusal.png)

*Terminal capture from a test installation. Your addresses, names and digests differ.*

### Install the platform

After the check passes, run:

```bash
./deploy install site.json
```

Use the same configuration file. The installer works over the administration
network with the configured SSH identities; you do not log into each server to
paste a sequence of shell fragments.

Image transfer and verified public-asset setup can take several minutes. Keep the
terminal open while the installer reports progress. The message
`VERIFY: waiting for the GraphQL metadata API to finish startup` is a normal,
bounded readiness wait; it needs no additional command. If startup does not finish
within the allowed time, the installer stops and reports the failure.

**Expected result.** The installer reports its stages and ends with
`INSTALL 4/4: platform installed; customer and trustee commissioning still required`. This means the platform
installation finished; it does not complete the customer accounts, trustee custody or
[Handover to the election board](11-selftest_handover.md#handover-to-the-election-board).

![Actual clean VM installation completing all four stages with exit status zero](./assets/installation/installer-complete.png)

*Terminal capture from a test installation. Your addresses, names and digests differ.*

**If the result differs.** Stop, and follow [If something fails](#if-something-fails).
A stopped installation may already have written files or started services: do not
erase its state or repeat secret generation. Never put a partially configured
installation into operation.

### Commission the customer and trustees

Platform installation is followed by one commissioning command. First, create the
customer's tenant through the portal using the installation account, as described
in [Create the customer's tenant](08-installing_toe.md#create-the-customers-tenant).
Use the customer's agreed short name; do not create a second tenant if one already
exists.

Fill the `commissioning` section in the same `site.json`. It names that customer
slug, the first administrator's existing external identity, and the two trustees'
registered names and private key-file paths. Each trustee generates and retains
their own keys; this command does not generate election trustees' keys or take over
their custody decisions.

```bash
./deploy commission site.json
```

The command resolves the customer tenant and performs the account and server
configuration. Follow its checkpoints for the external identity-provider
registration, public-key registration in the portal and the trustees' physical
handover of their media. Type `CONTINUE` only after the stated action is complete.
These are actions by the authentication server's administrator, the administrator
and the trustees; do not mark them done merely to make the command continue.

**Expected result.** The configured customer administrator and both native trustee
instances pass their checks. The command ends with
`COMMISSIONING COMMANDS COMPLETE — perform the browser, client-network and board handover checks in AGD-04.` Complete the
checks of [Handover to the election board](11-selftest_handover.md#handover-to-the-election-board);
a server check alone cannot establish those conditions.

![Actual clean VM commissioning completing the customer and both native trustee checks](./assets/installation/installer-commission-complete.png)

*Terminal capture from a test installation. Your addresses, names and digests differ.*

**If the result differs.** Stop at the reported stage. Check the customer slug,
registered trustee names and key media with their owners. If a previous attempt
stopped during a write, follow [If something fails](#if-something-fails). Never
choose a different tenant, substitute another trustee's keys, repeat account
creation or give a personal account a local password.

Then complete the personal-account and role checks in [Initial Accounts](09-accounts.md)
and the browser and handover checks in
[Verifying the Installation](10-verification.md) and
[Self-Test and Handover](11-selftest_handover.md). Each person authenticates
at the external identity provider; the installer does not impersonate them.

### Repeat a check later

To repeat the read-only installation checks later, use:

```bash
./deploy verify site.json
```

A verification result describes the checks that ran. It is not an authorization
to hold an election event.

After customer and trustee commissioning, use the broader inspection of
[Run the platform inspection](10-verification.md#run-the-platform-inspection):

```bash
./deploy inspect site.json platform
```

That section shows its actual output and remaining browser, client-network and
election-event checks. The storage probe creates and removes a temporary check pod.

Backup, restore, certificate renewal and trustee-key removal are later lifecycle
operations. They are not performed by the installation command. Their commands are in
[Backups and restore](11-selftest_handover.md#backups-and-restore),
[Renew the certificate](05-certificates_tls.md#renew-the-certificate) and
[Remove the trustee keys at the end of the election event](08-installing_toe.md#remove-the-trustee-keys-at-the-end-of-the-election-event).

## If something fails

These rules apply wherever this guide says to stop:

- **Keep what the tools report.** Where this guide says to keep a stage name, an
  error or a result: write down the stage, the message and the time, without any
  secret. During installation you give them to Sequent if you ask for support. Once
  an election event exists, also tell the election board: it records them in the
  board's minutes.
- **Find the stage.** Each stage's number is one below its chapter:
  `07-set-site-hostnames` belongs to [Configuring the Deployment](07-hardening.md).
- **No retry.** You cannot retry on this cluster: the installer refuses while the
  directory `/root/deploy` exists on a server, and deleting it does not give a valid
  installation. `./deploy commission site.json` also refuses once it has stopped
  during a write. Stop, and do not put the installation into operation.
- **The way back.** While no election event exists, reinstall both servers from the
  kit, which erases them ([Install C1 and C2](03-installation_kit.md#install-c1-and-c2)).
  Remove their old known-host entries (`ssh-keygen -R ADDRESS`), verify the new keys
  ([Verify SSH host keys before first use](03-installation_kit.md#verify-ssh-host-keys-before-first-use))
  and start again at [Check the site](#check-the-site). Once an election event
  exists, ask Sequent.
- **Ask Sequent.** Wherever this guide says to ask Sequent, write to
  [election-support@sequentech.io](mailto:election-support@sequentech.io),
  or to [security@sequentech.io](mailto:security@sequentech.io) for a security
  problem. Give the stage name, the error message and the TOE reference, and no
  secret.

## Detailed operations and acceptance criteria

The following chapters explain the configuration requirements, complete commands,
expected outputs and fault handling. Their terminal captures come from a test
installation; the admin portal captures come from a local build with test data.
Run a command where its section specifies; a server name or internal operation
number by itself is never an instruction to execute. The numbered operations in the
directory `steps/` of the deployment directory are run by `./deploy` itself, never
by hand. Installation is automated;
the explicit acceptance checks establish conditions that still require the
operator, an independent client network or another person.

| Reference | Purpose |
| --- | --- |
| [Installing the Servers](03-installation_kit.md) | Kit acceptance, disk confirmation and installed-node checks |
| [Network, Time and Cluster Checks](04-hosts_network_time.md) | Firewall, root-key access, authenticated time and encrypted cluster secrets |
| [Certificates and TLS](05-certificates_tls.md) | Trusted site certificate and required TLS profile |
| [Connecting the Authentication Server](06-authentication_server.md) | Trusted external identity provider and its registration |
| [Configuring the Deployment](07-hardening.md) | Generated secrets, production identity settings, routes and role templates |
| [Installing the TOE](08-installing_toe.md) | Delivered image identities, customer tenant and trustee commissioning |
| [Initial Accounts](09-accounts.md) | Personal accounts, role separation and first sign-in |
| [Verifying the Installation](10-verification.md) | Runtime and browser acceptance checks |
| [Self-Test and Handover](11-selftest_handover.md) | Handover to the election board, checks for each election event, telling the administrators that the server checks passed, operating the servers, backups and recovery |

Never copy, print or send a password, private key, token or client secret. Do not
record an installation with shell tracing or an indiscriminate terminal transcript.
The terminal captures in this guide show only the stated, non-secret operations;
their example addresses and identifiers are not values to enter at your site.

## Document history

| Version | Date | Changes |
| --- | --- | --- |
| 1.0 | 2026-10-27 | First edition for the evaluated configuration of v10.0.0. |
| 1.1 | 2026-11-10 | Numbered sections; chapters revised; handover, maintenance and recovery added; new portal screenshots. |
